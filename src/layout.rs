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
        Action::TopLeftQuarter
        | Action::TopRightQuarter
        | Action::BottomLeftQuarter
        | Action::BottomRightQuarter => {
            let cw = (area.width - gaps.inner) / 2.0;
            let ch = (area.height - gaps.inner) / 2.0;
            let (col, row) = match action {
                Action::TopLeftQuarter => (0.0, 0.0),
                Action::TopRightQuarter => (1.0, 0.0),
                Action::BottomLeftQuarter => (0.0, 1.0),
                _ => (1.0, 1.0),
            };
            Rect::new(
                area.x + col * (cw + gaps.inner),
                area.y + row * (ch + gaps.inner),
                cw,
                ch,
            )
        }
        Action::Maximize => area,
        Action::Restore => return None,
    };
    Some(rect)
}
