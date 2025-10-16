use std::collections::HashMap;

use crate::action::Action;
use crate::geometry::Rect;
use crate::layout::{self, Gaps};

pub struct Engine {
    gaps: Gaps,
    windows: HashMap<WindowId, WindowState>,
}

#[derive(Clone, Debug)]
struct WindowState {
    original: Rect,
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
            _ => self.place_in_display(id, action, snapshot),
        }
    }

    fn place_in_display(
        &mut self,
        id: WindowId,
        action: Action,
        snapshot: &Snapshot,
    ) -> Option<Rect> {
        let display = layout::display_for(&snapshot.window, &snapshot.visible_frames)?;
        let visible_frame = snapshot.visible_frames[display];
        let target = layout::place(action, 0.5, &snapshot.window, &visible_frame, self.gaps)?;
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
