use std::collections::HashMap;

use crate::action::Action;
use crate::display;
use crate::geometry::Rect;
use crate::layout::{self, Gaps};

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

    pub fn place(&mut self, id: WindowId, action: Action, snapshot: &Snapshot) -> Option<Rect> {
        match action {
            Action::Restore => self.windows.remove(&id).map(|state| state.original),
            Action::NextDisplay | Action::PrevDisplay => self.move_display(id, action, snapshot),
            _ => {
                let display = display::display_for(&snapshot.window, &snapshot.visible_frames)?;
                let visible_frame = snapshot.visible_frames[display];
                let target = layout::place(action, &snapshot.window, &visible_frame, self.gaps)?;
                self.windows.entry(id).or_insert(WindowState {
                    original: snapshot.window,
                });
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
        self.windows.entry(id).or_insert(WindowState {
            original: snapshot.window,
        });
        Some(target)
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct WindowId(pub u64);

#[derive(Clone, Debug)]
pub struct Snapshot {
    pub window: Rect,
    pub visible_frames: Vec<Rect>,
}

#[derive(Clone, Debug)]
struct WindowState {
    original: Rect,
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
    const ID: WindowId = WindowId(1);

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
    fn place_next_display_with_frames_out_of_order() {
        assert_cycles(Action::NextDisplay, [WINDOW_ON_RIGHT, WINDOW_ON_LEFT]);
    }

    #[test]
    fn place_prev_display_with_frames_out_of_order() {
        assert_cycles(Action::PrevDisplay, [WINDOW_ON_LEFT, WINDOW_ON_RIGHT]);
    }
}
