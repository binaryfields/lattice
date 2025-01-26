use crate::action::Action;
use crate::config::Sweep;
use crate::engine::{Snapshot, WindowId};

pub struct Session<W> {
    window: W,
    id: WindowId,
    start: (f64, f64),
    snapshot: Snapshot,
}

impl<W> Session<W> {
    pub fn begin(window: W, id: WindowId, start: (f64, f64), snapshot: Snapshot) -> Self {
        Session {
            window,
            id,
            start,
            snapshot,
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

    pub fn commit(&self, cursor: (f64, f64), cfg: &Sweep) -> Option<Action> {
        classify(self.start, cursor, cfg.distance)
    }
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
