use crate::action::Action;
use crate::config::Sweep;
use crate::engine::{Engine, Snapshot, WindowId};
use crate::geometry::Rect;

pub struct Session<W> {
    window: W,
    id: WindowId,
    start: (f64, f64),
    snapshot: Snapshot,
    preview: Option<Rect>,
}

impl<W> Session<W> {
    pub fn begin(window: W, id: WindowId, start: (f64, f64), snapshot: Snapshot) -> Self {
        Session {
            window,
            id,
            start,
            snapshot,
            preview: None,
        }
    }

    pub fn window(&self) -> &W {
        &self.window
    }

    pub fn id(&self) -> WindowId {
        self.id
    }

    pub fn snapshot(&self) -> &Snapshot {
        &self.snapshot
    }

    pub fn track(&mut self, cursor: (f64, f64), engine: &Engine, cfg: &Sweep) -> PreviewChange {
        let target = classify(self.start, cursor, cfg.distance)
            .and_then(|action| engine.preview(action, &self.snapshot));
        if target == self.preview {
            return PreviewChange::Unchanged;
        }
        self.preview = target;
        match target {
            Some(rect) => PreviewChange::Show(rect),
            None => PreviewChange::Hide,
        }
    }

    pub fn commit(&self, cursor: (f64, f64), cfg: &Sweep) -> Option<Action> {
        classify(self.start, cursor, cfg.distance)
    }
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum PreviewChange {
    Unchanged,
    Show(Rect),
    Hide,
}

fn classify(start: (f64, f64), end: (f64, f64), min_distance: f64) -> Option<Action> {
    let (dx, dy) = (end.0 - start.0, end.1 - start.1);
    if dx.hypot(dy) < min_distance {
        return None;
    }
    let angle = dy.atan2(dx).to_degrees();
    let sector = (((angle + 22.5).rem_euclid(360.0)) / 45.0).floor() as usize % 8;
    Some(match sector {
        0 => Action::RightHalf,
        1 => Action::BottomRightQuarter,
        2 => Action::BottomHalf,
        3 => Action::BottomLeftQuarter,
        4 => Action::LeftHalf,
        5 => Action::TopLeftQuarter,
        6 => Action::TopHalf,
        _ => Action::TopRightQuarter,
    })
}
