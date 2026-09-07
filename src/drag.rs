use crate::action::Action;
use crate::config::Drag;
use crate::display;
use crate::engine::{Engine, Grab, Snapshot};
use crate::geometry::Rect;

const CAPTURE_DISTANCE: f64 = 12.0;
const SIZE_TOLERANCE: f64 = 1.0;
const TRACK_DISTANCE: f64 = 12.0;
const UNSNAP_DISTANCE: f64 = 40.0;

pub struct Session<W> {
    start: (f64, f64),
    held: Option<Held<W>>,
    abandoned: bool,
}

impl<W> Session<W> {
    pub fn begin(start: (f64, f64)) -> Session<W> {
        Session {
            start,
            held: None,
            abandoned: false,
        }
    }

    pub fn start(&self) -> (f64, f64) {
        self.start
    }

    pub fn window(&self) -> Option<&W> {
        self.held.as_ref().map(|held| &held.grab.window)
    }

    pub fn probe(&self, cursor: (f64, f64)) -> Probe {
        if self.abandoned {
            return Probe::Idle;
        }
        match &self.held {
            None if travelled(self.start, cursor) >= CAPTURE_DISTANCE => Probe::Window,
            Some(held) if !held.tracking => Probe::Frame,
            _ => Probe::Idle,
        }
    }

    pub fn capture(&mut self, grab: Grab<W>, cursor: (f64, f64)) {
        self.held = Some(Held {
            grab,
            anchor: cursor,
            tracking: false,
        });
    }

    /// Judges from a fresh read of the captured window's frame whether it rides
    /// along with the drag. macOS moves a dragged window itself and tells the
    /// owning app afterwards, so the frame the Accessibility API reports trails
    /// the cursor: by well over 100 ms before it moves at all, then by a lag
    /// that grows with speed. The test is therefore not "frame matches cursor"
    /// but "frame has set off in the cursor's direction at its original size".
    /// Until it has, the session stays undecided and is judged again on the
    /// next sample; a change of size means a resize, and the session goes inert.
    pub fn confirm(&mut self, frame: &Rect, cursor: (f64, f64)) {
        if self.abandoned {
            return;
        }
        let Some(held) = self.held.as_mut() else {
            return;
        };
        if held.tracking {
            return;
        }
        let anchored = held.grab.snapshot.window;
        if (frame.width - anchored.width).abs() > SIZE_TOLERANCE
            || (frame.height - anchored.height).abs() > SIZE_TOLERANCE
        {
            self.abandoned = true;
            return;
        }
        let (fx, fy) = (frame.x - anchored.x, frame.y - anchored.y);
        let (dx, dy) = (cursor.0 - held.anchor.0, cursor.1 - held.anchor.1);
        held.tracking = fx.hypot(fy) >= TRACK_DISTANCE && fx * dx + fy * dy > 0.0;
    }

    pub fn track(&self, cursor: (f64, f64), engine: &Engine, cfg: &Drag) -> Option<Rect> {
        let held = self.held.as_ref()?;
        let (action, snapshot) = self.zone(cursor, held.grab.snapshot.window, cfg)?;
        engine.preview(action, &snapshot)
    }

    /// The action a drop at `cursor` picks, and `window` as it stands on the
    /// display the cursor is over.
    fn zone(&self, cursor: (f64, f64), window: Rect, cfg: &Drag) -> Option<(Action, Snapshot)> {
        let held = self.held.as_ref()?;
        if self.abandoned || !held.tracking {
            return None;
        }
        let frames = held.grab.snapshot.visible_frames.as_slice();
        let visible_frame = frames[display::display_at(cursor.0, cursor.1, frames)?];
        let action = classify(cursor.0, cursor.1, &visible_frame, cfg.edge, cfg.corner)?;
        let snapshot = Snapshot {
            window,
            visible_frames: vec![visible_frame],
        };
        Some((action, snapshot))
    }

    pub fn undecided(&self) -> bool {
        !self.abandoned && self.held.as_ref().is_some_and(|held| !held.tracking)
    }

    pub fn release(mut self, cursor: (f64, f64), frame: &Rect, cfg: &Drag) -> Option<Landing<W>> {
        self.confirm(frame, cursor);
        if self.abandoned {
            return None;
        }
        let zone = self.zone(cursor, *frame, cfg);
        let Held {
            grab,
            anchor,
            tracking,
        } = self.held?;
        match zone {
            Some((action, snapshot)) => Some(Landing::Snap(Grab { snapshot, ..grab }, action)),
            None if tracking && travelled(anchor, cursor) >= UNSNAP_DISTANCE => {
                Some(Landing::Unsnap(grab))
            }
            None => None,
        }
    }

    /// Takes the session out of the running for the rest of the drag: no
    /// preview, no landing.
    pub fn abandon(&mut self) {
        self.abandoned = true;
    }
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Probe {
    Idle,
    Window,
    Frame,
}

pub enum Landing<W> {
    Snap(Grab<W>, Action),
    Unsnap(Grab<W>),
}

struct Held<W> {
    grab: Grab<W>,
    anchor: (f64, f64),
    tracking: bool,
}

fn travelled(from: (f64, f64), to: (f64, f64)) -> f64 {
    (to.0 - from.0).hypot(to.1 - from.1)
}

/// Corners take priority over edges; an edge zone is `edge` deep, a corner
/// zone `corner` deep along both sides.
fn classify(x: f64, y: f64, frame: &Rect, edge: f64, corner: f64) -> Option<Action> {
    let near = |d: f64| {
        let (left, right) = (x <= frame.x + d, x >= frame.max_x() - d);
        (left, right, y <= frame.y + d, y >= frame.max_y() - d)
    };
    match (near(corner), near(edge)) {
        ((true, _, true, _), _) => Some(Action::TopLeftQuarter),
        ((_, true, true, _), _) => Some(Action::TopRightQuarter),
        ((true, _, _, true), _) => Some(Action::BottomLeftQuarter),
        ((_, true, _, true), _) => Some(Action::BottomRightQuarter),
        (_, (true, ..)) => Some(Action::LeftHalf),
        (_, (_, true, ..)) => Some(Action::RightHalf),
        (_, (_, _, true, _)) => Some(Action::Maximize),
        (_, (.., true)) => Some(Action::BottomHalf),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::WindowId;

    const VISIBLE_FRAME: Rect = Rect::new(0.0, 25.0, 1000.0, 775.0);
    const SCREEN: Rect = Rect::new(0.0, 0.0, 1000.0, 800.0);
    const RIGHT_SCREEN: Rect = Rect::new(1000.0, 0.0, 1000.0, 800.0);
    const WINDOW: Rect = Rect::new(300.0, 300.0, 400.0, 300.0);
    const START: (f64, f64) = (500.0, 400.0);
    const EDGE: (f64, f64) = (4.0, 400.0);
    const CFG: Drag = Drag {
        enabled: true,
        edge: 8.0,
        corner: 64.0,
    };

    fn captured(anchor: (f64, f64)) -> Session<()> {
        let mut session = Session::begin(anchor);
        let snapshot = Snapshot {
            window: WINDOW,
            visible_frames: vec![SCREEN, RIGHT_SCREEN],
        };
        session.capture(
            Grab {
                window: (),
                id: WindowId(1),
                snapshot,
            },
            anchor,
        );
        session
    }

    fn dragged_to(cursor: (f64, f64)) -> Rect {
        Rect::new(
            WINDOW.x + cursor.0 - START.0,
            WINDOW.y + cursor.1 - START.1,
            WINDOW.width,
            WINDOW.height,
        )
    }

    fn tracking(cursor: (f64, f64)) -> Session<()> {
        let mut session = captured(START);
        session.confirm(&dragged_to(cursor), cursor);
        session
    }

    fn shifted(dx: f64, dy: f64) -> Rect {
        Rect::new(WINDOW.x + dx, WINDOW.y + dy, WINDOW.width, WINDOW.height)
    }

    #[test]
    fn confirm_with_stationary_window() {
        let cursor = (START.0 - 200.0, START.1);
        let mut session = captured(START);
        assert_eq!(session.probe(cursor), Probe::Frame);
        session.confirm(&WINDOW, cursor);
        assert!(session.undecided());
        assert_eq!(session.probe(cursor), Probe::Frame);
        assert!(session.release(EDGE, &WINDOW, &CFG).is_none());
    }

    #[test]
    fn confirm_with_lagging_window() {
        let cursor = (START.0 - 100.0, START.1);
        let mut session = captured(START);
        session.confirm(&shifted(-30.0, 0.0), cursor);
        assert!(!session.undecided());
        assert_eq!(session.probe(cursor), Probe::Idle);
        let landing = session.release(EDGE, &dragged_to(EDGE), &CFG);
        assert!(matches!(landing, Some(Landing::Snap(..))));
    }

    #[test]
    fn confirm_with_window_short_of_track_distance() {
        let cursor = (START.0 - 100.0, START.1);
        let mut session = captured(START);
        session.confirm(&shifted(-(TRACK_DISTANCE - 1.0), 0.0), cursor);
        assert!(session.undecided());
        session.confirm(&shifted(-TRACK_DISTANCE, 0.0), cursor);
        assert!(!session.undecided());
    }

    #[test]
    fn confirm_with_window_moving_against_cursor() {
        let cursor = (START.0 - 100.0, START.1);
        let mut session = captured(START);
        session.confirm(&shifted(50.0, 0.0), cursor);
        assert!(session.undecided());
    }

    #[test]
    fn confirm_with_resized_window() {
        let cursor = (START.0 - 100.0, START.1);
        let mut resized = shifted(-100.0, 0.0);
        resized.width += 100.0;
        let mut session = captured(START);
        session.confirm(&resized, cursor);
        assert!(!session.undecided());
        assert_eq!(session.probe(cursor), Probe::Idle);
        assert!(session.release(EDGE, &dragged_to(EDGE), &CFG).is_none());
    }

    #[test]
    fn track_after_abandon_hides_preview() {
        let engine = Engine::new();
        let mut session = tracking(EDGE);
        assert!(session.track(EDGE, &engine, &CFG).is_some());
        session.abandon();
        assert_eq!(session.track(EDGE, &engine, &CFG), None);
    }

    #[test]
    fn release_after_abandon_while_tracking() {
        let mut session = tracking(EDGE);
        session.abandon();
        assert!(session.release(EDGE, &dragged_to(EDGE), &CFG).is_none());
    }

    #[test]
    fn release_with_window_catching_up() {
        let mut session = captured(START);
        session.confirm(&WINDOW, EDGE);
        assert!(session.undecided());
        let landing = session.release(EDGE, &dragged_to(EDGE), &CFG);
        assert!(matches!(landing, Some(Landing::Snap(..))));
    }

    #[test]
    fn release_in_zone_on_second_display() {
        let cursor = (1996.0, 400.0);
        let frame = dragged_to(cursor);
        let landing = tracking(cursor).release(cursor, &frame, &CFG);
        let Some(Landing::Snap(grab, action)) = landing else {
            panic!("a tracked drag into an edge snaps");
        };
        assert_eq!(action, Action::RightHalf);
        assert_eq!(grab.snapshot.window, frame);
        assert_eq!(grab.snapshot.visible_frames, vec![RIGHT_SCREEN]);
    }

    #[test]
    fn release_short_of_unsnap_distance() {
        let cursor = (520.0, 400.0);
        let landing = tracking((540.0, 400.0)).release(cursor, &dragged_to(cursor), &CFG);
        assert!(landing.is_none());
    }

    fn zoned(x: f64, y: f64) -> Option<Action> {
        classify(x, y, &VISIBLE_FRAME, CFG.edge, CFG.corner)
    }

    #[test]
    fn classify_off_visible_frame() {
        assert_eq!(zoned(500.0, 0.0), Some(Action::Maximize));
        assert_eq!(zoned(-4.0, 400.0), Some(Action::LeftHalf));
        assert_eq!(zoned(-4.0, 0.0), Some(Action::TopLeftQuarter));
    }

    #[test]
    fn classify_clear_of_every_zone() {
        assert_eq!(zoned(500.0, 400.0), None);
        assert_eq!(zoned(9.0, 400.0), None);
        assert_eq!(zoned(500.0, 34.0), None);
    }
}
