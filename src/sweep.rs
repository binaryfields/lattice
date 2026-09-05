use crate::action::Action;
use crate::config::Sweep;
use crate::engine::{Engine, Grab};
use crate::geometry::Rect;

/// The action each 45 degree sector of a sweep picks, clockwise from right:
/// positive angles point down.
const SECTORS: [Action; 8] = [
    Action::RightHalf,
    Action::BottomRightQuarter,
    Action::BottomHalf,
    Action::BottomLeftQuarter,
    Action::LeftHalf,
    Action::TopLeftQuarter,
    Action::TopHalf,
    Action::TopRightQuarter,
];

pub struct Session<W> {
    grab: Grab<W>,
    start: (f64, f64),
}

impl<W> Session<W> {
    pub fn begin(grab: Grab<W>, start: (f64, f64)) -> Self {
        Session { grab, start }
    }

    pub fn grab(&self) -> &Grab<W> {
        &self.grab
    }

    pub fn track(&self, cursor: (f64, f64), engine: &Engine, cfg: &Sweep) -> Option<Rect> {
        classify(self.start, cursor, cfg.distance)
            .and_then(|action| engine.preview(action, &self.grab.snapshot))
    }

    pub fn commit(&self, cursor: (f64, f64), cfg: &Sweep) -> Option<Action> {
        classify(self.start, cursor, cfg.distance)
    }
}

fn classify(start: (f64, f64), end: (f64, f64), min_distance: f64) -> Option<Action> {
    let (dx, dy) = (end.0 - start.0, end.1 - start.1);
    if dx.hypot(dy) < min_distance {
        return None;
    }
    let sector = (dy.atan2(dx).to_degrees() / 45.0).round() as i32;
    Some(SECTORS[sector.rem_euclid(8) as usize])
}

#[cfg(test)]
mod tests {
    use global_hotkey::hotkey::Modifiers;

    use super::*;
    use crate::engine::{Snapshot, WindowId};

    const CFG: Sweep = Sweep {
        enabled: true,
        distance: 10.0,
        modifier: Modifiers::CONTROL.union(Modifiers::SUPER),
    };
    const SCREEN: Rect = Rect::new(0.0, 0.0, 1000.0, 800.0);
    const START: (f64, f64) = (100.0, 100.0);
    const RIGHT_HALF: Rect = Rect::new(502.5, 5.0, 492.5, 790.0);
    const LEFT_HALF: Rect = Rect::new(5.0, 5.0, 492.5, 790.0);

    fn session() -> Session<()> {
        let snapshot = Snapshot {
            window: Rect::new(120.0, 90.0, 400.0, 300.0),
            visible_frames: vec![SCREEN],
        };
        Session::begin(
            Grab {
                window: (),
                id: WindowId(1),
                snapshot,
            },
            START,
        )
    }

    #[track_caller]
    fn assert_tracks(path: &[((f64, f64), Option<Rect>)]) {
        let (session, engine) = (session(), Engine::new());
        for &(cursor, outline) in path {
            assert_eq!(session.track(cursor, &engine, &CFG), outline, "{cursor:?}");
        }
    }

    #[test]
    fn track_within_one_sector() {
        assert_tracks(&[
            ((160.0, 100.0), Some(RIGHT_HALF)),
            ((200.0, 100.0), Some(RIGHT_HALF)),
        ]);
    }

    #[test]
    fn track_across_sectors() {
        assert_tracks(&[
            ((160.0, 100.0), Some(RIGHT_HALF)),
            ((40.0, 100.0), Some(LEFT_HALF)),
        ]);
    }

    #[test]
    fn track_back_below_min_distance() {
        assert_tracks(&[(START, None), ((103.0, 100.0), None)]);
    }

    #[test]
    fn commit_after_track() {
        let (session, mut engine) = (session(), Engine::new());
        let cursor = (160.0, 100.0);
        assert_eq!(session.track(cursor, &engine, &CFG), Some(RIGHT_HALF));
        let action = session
            .commit(cursor, &CFG)
            .expect("a swept cursor commits");
        assert_eq!(
            engine.place(session.grab().id, action, &session.grab().snapshot),
            Some(RIGHT_HALF)
        );
    }

    fn swept(dx: f64, dy: f64) -> Option<Action> {
        classify((100.0, 100.0), (100.0 + dx, 100.0 + dy), CFG.distance)
    }

    #[test]
    fn classify_each_sector() {
        assert_eq!(swept(50.0, 0.0), Some(Action::RightHalf));
        assert_eq!(swept(50.0, 50.0), Some(Action::BottomRightQuarter));
        assert_eq!(swept(0.0, 50.0), Some(Action::BottomHalf));
        assert_eq!(swept(-50.0, 50.0), Some(Action::BottomLeftQuarter));
        assert_eq!(swept(-50.0, 0.0), Some(Action::LeftHalf));
        assert_eq!(swept(-50.0, -50.0), Some(Action::TopLeftQuarter));
        assert_eq!(swept(0.0, -50.0), Some(Action::TopHalf));
        assert_eq!(swept(50.0, -50.0), Some(Action::TopRightQuarter));
    }

    #[test]
    fn classify_at_sector_boundary() {
        assert_eq!(swept(100.0, 41.0), Some(Action::RightHalf));
        assert_eq!(swept(100.0, 42.0), Some(Action::BottomRightQuarter));
    }
}
