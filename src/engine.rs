use std::collections::HashMap;

use crate::action::Action;
use crate::config::Config;
use crate::geometry::Rect;
use crate::layout;

pub struct Engine {
    config: Config,
    windows: HashMap<WindowId, WindowState>,
}

#[derive(Clone, Debug)]
struct WindowState {
    original: Rect,
}

impl Engine {
    pub fn new(config: Config) -> Self {
        Self {
            config,
            windows: HashMap::new(),
        }
    }

    pub fn config(&self) -> &Config {
        &self.config
    }

    pub fn set_config(&mut self, config: Config) {
        self.config = config;
    }

    pub fn place(&mut self, id: WindowId, action: Action, snapshot: &Snapshot) -> Option<Rect> {
        match action {
            Action::Restore => self.windows.remove(&id).map(|state| state.original),
            Action::NextDisplay | Action::PrevDisplay => self.move_display(id, action, snapshot),
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
        let target = layout::place(
            action,
            0.5,
            &snapshot.window,
            &visible_frame,
            self.config.gaps,
        )?;
        self.windows.entry(id).or_insert(WindowState {
            original: snapshot.window,
        });
        Some(target)
    }

    fn move_display(&mut self, id: WindowId, action: Action, snapshot: &Snapshot) -> Option<Rect> {
        if snapshot.visible_frames.len() < 2 {
            return None;
        }
        let current = layout::display_for(&snapshot.window, &snapshot.visible_frames)?;
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

        let target = layout::remap_between(
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
