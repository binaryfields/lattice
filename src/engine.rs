use std::collections::HashMap;

use crate::action::Action;
use crate::display;
use crate::geometry::Rect;
use crate::layout::{self, Gaps};

const RESIZE_TOLERANCE: f64 = 2.0;

pub struct Engine {
    gaps: Gaps,
    windows: HashMap<WindowId, WindowState>,
}

impl Engine {
    pub fn new() -> Self {
        Self {
            gaps: Gaps::default(),
            windows: HashMap::new(),
        }
    }

    pub fn set_gaps(&mut self, gaps: Gaps) {
        self.gaps = gaps;
    }

    pub fn place(&mut self, id: WindowId, action: Action, snapshot: &Snapshot) -> Option<Rect> {
        match action {
            Action::Restore => self.windows.remove(&id).map(|state| state.original),
            Action::NextDisplay | Action::PrevDisplay => self.move_display(id, action, snapshot),
            _ => {
                let target = self.preview(action, snapshot)?;
                self.record(id, action, snapshot.window, target);
                Some(target)
            }
        }
    }

    fn move_display(&mut self, id: WindowId, action: Action, snapshot: &Snapshot) -> Option<Rect> {
        if snapshot.visible_frames.len() < 2 {
            return None;
        }
        let current = display::display_for(&snapshot.window, &snapshot.visible_frames)?;
        let mut order: Vec<usize> = (0..snapshot.visible_frames.len()).collect();
        order.sort_by(|&a, &b| {
            let (da, db) = (&snapshot.visible_frames[a], &snapshot.visible_frames[b]);
            da.x.total_cmp(&db.x).then(da.y.total_cmp(&db.y))
        });
        let pos = order.iter().position(|&i| i == current)?;
        let step = if action == Action::NextDisplay {
            1
        } else {
            order.len() - 1
        };
        let dest = order[(pos + step) % order.len()];

        let target = display::remap_between(
            &snapshot.window,
            &snapshot.visible_frames[current],
            &snapshot.visible_frames[dest],
        );
        self.record(id, action, snapshot.window, target);
        Some(target)
    }

    fn record(&mut self, id: WindowId, action: Action, before: Rect, target: Rect) {
        let state = self.windows.entry(id).or_insert(WindowState {
            original: before,
            presnap: None,
            placed: None,
        });
        if action.resizes() && state.presnap.is_none() {
            state.presnap = Some((before.width, before.height));
        }
        state.placed = Some(target);
    }

    pub fn preview(&self, action: Action, snapshot: &Snapshot) -> Option<Rect> {
        let display = display::display_for(&snapshot.window, &snapshot.visible_frames)?;
        let visible_frame = snapshot.visible_frames[display];
        layout::place(action, &snapshot.window, &visible_frame, self.gaps)
    }

    pub fn applied(&mut self, id: WindowId, actual: Rect) {
        if let Some(state) = self.windows.get_mut(&id)
            && state.placed.is_some()
        {
            state.placed = Some(actual);
        }
    }

    pub fn unsnap(&mut self, id: WindowId, cursor: (f64, f64), window: &Rect) -> Option<Rect> {
        let state = self.windows.get_mut(&id)?;
        let placed = state.placed?;
        if (window.width - placed.width).abs() > RESIZE_TOLERANCE
            || (window.height - placed.height).abs() > RESIZE_TOLERANCE
        {
            state.presnap = None;
            return None;
        }
        let (width, height) = state.presnap.take()?;
        state.placed = None;
        let grip = ((cursor.0 - window.x) / window.width.max(1.0)).clamp(0.0, 1.0);
        Some(Rect::new(cursor.0 - grip * width, window.y, width, height))
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct WindowId(pub u64);

#[derive(Clone, Debug)]
pub struct Snapshot {
    pub window: Rect,
    pub visible_frames: Vec<Rect>,
}

/// A window taken hold of: the handle that moves it, its identity, and its
/// frame and the displays' as they were read together.
pub struct Grab<W> {
    pub window: W,
    pub id: WindowId,
    pub snapshot: Snapshot,
}

#[derive(Clone, Debug)]
struct WindowState {
    original: Rect,
    presnap: Option<(f64, f64)>,
    placed: Option<Rect>,
}

#[cfg(test)]
mod tests {
    use super::*;

    const SCREEN: Rect = Rect::new(0.0, 0.0, 1000.0, 800.0);
    const LEFT_SCREEN: Rect = Rect::new(-1000.0, 0.0, 1000.0, 800.0);
    const RIGHT_SCREEN: Rect = Rect::new(1000.0, 0.0, 1000.0, 800.0);
    const WINDOW: Rect = Rect::new(300.0, 300.0, 400.0, 300.0);
    const WINDOW_ON_LEFT: Rect = Rect::new(-700.0, 300.0, 400.0, 300.0);
    const WINDOW_ON_RIGHT: Rect = Rect::new(1300.0, 300.0, 400.0, 300.0);
    const LEFT_HALF: Rect = Rect::new(5.0, 5.0, 492.5, 790.0);
    const ID: WindowId = WindowId(1);

    fn snapshot(window: Rect) -> Snapshot {
        Snapshot {
            window,
            visible_frames: vec![SCREEN],
        }
    }

    fn snapped() -> Engine {
        let mut engine = Engine::new();
        assert_eq!(
            engine.place(ID, Action::LeftHalf, &snapshot(WINDOW)),
            Some(LEFT_HALF)
        );
        engine
    }

    fn across_displays(window: Rect) -> Snapshot {
        Snapshot {
            window,
            visible_frames: vec![SCREEN, LEFT_SCREEN, RIGHT_SCREEN],
        }
    }

    #[track_caller]
    fn assert_cycles(action: Action, through: [Rect; 2]) {
        let mut engine = Engine::new();
        let mut from = WINDOW;
        for to in through {
            assert_eq!(engine.place(ID, action, &across_displays(from)), Some(to));
            from = to;
        }
    }

    #[test]
    fn place_restore_after_two_actions() {
        let mut engine = snapped();
        engine.place(ID, Action::Maximize, &snapshot(LEFT_HALF));
        assert_eq!(
            engine.place(ID, Action::Restore, &snapshot(WINDOW)),
            Some(WINDOW)
        );
    }

    #[test]
    fn place_next_display_with_frames_out_of_order() {
        assert_cycles(Action::NextDisplay, [WINDOW_ON_RIGHT, WINDOW_ON_LEFT]);
    }

    #[test]
    fn place_prev_display_with_frames_out_of_order() {
        assert_cycles(Action::PrevDisplay, [WINDOW_ON_LEFT, WINDOW_ON_RIGHT]);
    }

    fn dragged_clear() -> (Rect, (f64, f64)) {
        let frame = Rect::new(200.0, 100.0, LEFT_HALF.width, LEFT_HALF.height);
        (frame, (frame.x + frame.width / 2.0, frame.y + 10.0))
    }

    #[test]
    fn applied_with_granted_size() {
        let (frame, cursor) = dragged_clear();
        let granted = Rect::new(frame.x, frame.y, frame.width - 10.0, frame.height);
        let mut engine = snapped();
        assert_eq!(engine.unsnap(ID, cursor, &granted), None);

        let mut engine = snapped();
        engine.applied(
            ID,
            Rect::new(LEFT_HALF.x, LEFT_HALF.y, granted.width, granted.height),
        );
        assert!(engine.unsnap(ID, cursor, &granted).is_some());
    }

    #[test]
    fn unsnap_with_window_at_placed_size() {
        let (frame, cursor) = dragged_clear();
        let restored = snapped()
            .unsnap(ID, cursor, &frame)
            .expect("a window still at its snapped size unsnaps");
        assert_eq!(restored.width, WINDOW.width);
        assert_eq!(restored.height, WINDOW.height);
        assert_eq!(restored.y, frame.y);
        assert_eq!((cursor.0 - restored.x) / restored.width, 0.5);
    }

    #[test]
    fn unsnap_after_second_snap() {
        let mut engine = snapped();
        let maximized = engine
            .place(ID, Action::Maximize, &snapshot(LEFT_HALF))
            .expect("maximize places a rect");
        let frame = Rect::new(200.0, 100.0, maximized.width, maximized.height);
        let restored = engine
            .unsnap(ID, (frame.x + 10.0, frame.y + 10.0), &frame)
            .expect("a window still at its snapped size unsnaps");
        assert_eq!(restored.width, WINDOW.width);
        assert_eq!(restored.height, WINDOW.height);
    }

    #[test]
    fn unsnap_with_window_resized_since_placed() {
        let (frame, cursor) = dragged_clear();
        let resized = Rect::new(frame.x, frame.y, frame.width - 50.0, frame.height);
        assert_eq!(snapped().unsnap(ID, cursor, &resized), None);
    }
}
