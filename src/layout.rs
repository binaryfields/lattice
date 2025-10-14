use crate::action::Action;
use crate::geometry::Rect;

const DEFAULT_GAP: f64 = 5.0;

#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Gaps {
    pub outer: f64,
    pub inner: f64,
}

impl Default for Gaps {
    fn default() -> Self {
        Gaps {
            outer: DEFAULT_GAP,
            inner: DEFAULT_GAP,
        }
    }
}

pub fn place(action: Action, _window: &Rect, visible_frame: &Rect, gaps: Gaps) -> Option<Rect> {
    let area = visible_frame.inset(gaps.outer);
    let half = |full: f64| full / 2.0 - gaps.inner / 2.0;

    let rect = match action {
        Action::LeftHalf => {
            let w = half(area.width);
            Rect::new(area.x, area.y, w, area.height)
        }
        Action::RightHalf => {
            let w = half(area.width);
            Rect::new(area.max_x() - w, area.y, w, area.height)
        }
        Action::TopHalf => {
            let h = half(area.height);
            Rect::new(area.x, area.y, area.width, h)
        }
        Action::BottomHalf => {
            let h = half(area.height);
            Rect::new(area.x, area.max_y() - h, area.width, h)
        }
        Action::Maximize => area,
    };
    Some(rect)
}
