use crate::action::Action;
use crate::geometry::Rect;

const DEFAULT_GAP: f64 = 5.0;

pub fn place(
    action: Action,
    ratio: f64,
    _window: &Rect,
    visible_frame: &Rect,
    gaps: Gaps,
) -> Option<Rect> {
    let area = visible_frame.inset(gaps.outer);
    let half_gap = |shared: bool| if shared { gaps.inner / 2.0 } else { 0.0 };

    let rect = match action {
        Action::LeftHalf => {
            let w = area.width * ratio - half_gap(ratio < 1.0);
            Rect::new(area.x, area.y, w, area.height)
        }
        Action::RightHalf => {
            let w = area.width * ratio - half_gap(ratio < 1.0);
            Rect::new(area.max_x() - w, area.y, w, area.height)
        }
        Action::TopHalf => {
            let h = area.height * ratio - half_gap(ratio < 1.0);
            Rect::new(area.x, area.y, area.width, h)
        }
        Action::BottomHalf => {
            let h = area.height * ratio - half_gap(ratio < 1.0);
            Rect::new(area.x, area.max_y() - h, area.width, h)
        }
        Action::TopLeftQuarter => area.column(gaps.inner, 2, 0, 1).row(gaps.inner, 2, 0, 1),
        Action::TopRightQuarter => area.column(gaps.inner, 2, 1, 1).row(gaps.inner, 2, 0, 1),
        Action::BottomLeftQuarter => area.column(gaps.inner, 2, 0, 1).row(gaps.inner, 2, 1, 1),
        Action::BottomRightQuarter => area.column(gaps.inner, 2, 1, 1).row(gaps.inner, 2, 1, 1),
        Action::Maximize => area,
        Action::Restore => return None,
    };
    Some(rect)
}

pub fn display_for(window: &Rect, visible_frames: &[Rect]) -> Option<usize> {
    if visible_frames.is_empty() {
        return None;
    }
    let best = visible_frames
        .iter()
        .enumerate()
        .max_by(|(_, a), (_, b)| {
            window
                .intersection_area(a)
                .total_cmp(&window.intersection_area(b))
        })
        .map(|(i, _)| i)?;
    if window.intersection_area(&visible_frames[best]) > 0.0 {
        return Some(best);
    }
    let (wx, wy) = window.center();
    visible_frames
        .iter()
        .enumerate()
        .min_by(|(_, a), (_, b)| {
            let (ax, ay) = a.center();
            let (bx, by) = b.center();
            let da = (ax - wx).powi(2) + (ay - wy).powi(2);
            let db = (bx - wx).powi(2) + (by - wy).powi(2);
            da.total_cmp(&db)
        })
        .map(|(i, _)| i)
}

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
