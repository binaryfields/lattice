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

pub fn place(action: Action, window: &Rect, visible_frame: &Rect, gaps: Gaps) -> Option<Rect> {
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
        Action::TopLeftQuarter => area.column(gaps.inner, 2, 0, 1).row(gaps.inner, 2, 0, 1),
        Action::TopRightQuarter => area.column(gaps.inner, 2, 1, 1).row(gaps.inner, 2, 0, 1),
        Action::BottomLeftQuarter => area.column(gaps.inner, 2, 0, 1).row(gaps.inner, 2, 1, 1),
        Action::BottomRightQuarter => area.column(gaps.inner, 2, 1, 1).row(gaps.inner, 2, 1, 1),
        Action::FirstThird => area.column(gaps.inner, 3, 0, 1),
        Action::CenterThird => area.column(gaps.inner, 3, 1, 1),
        Action::LastThird => area.column(gaps.inner, 3, 2, 1),
        Action::FirstTwoThirds => area.column(gaps.inner, 3, 0, 2),
        Action::LastTwoThirds => area.column(gaps.inner, 3, 1, 2),
        Action::Maximize => area,
        Action::AlmostMaximize => {
            Rect::new(0.0, 0.0, area.width * 0.9, area.height * 0.9).centered_in(&area)
        }
        Action::Center => window.centered_in(&area),
        Action::Restore => return None,
    };
    Some(rect)
}

#[cfg(test)]
mod tests {
    use strum::IntoEnumIterator;

    use super::*;

    const SCREEN: Rect = Rect::new(0.0, 0.0, 1000.0, 800.0);
    const WINDOW: Rect = Rect::new(120.0, 90.0, 400.0, 300.0);
    const EPS: f64 = 1e-9;

    fn gaps(outer: f64, inner: f64) -> Gaps {
        Gaps { outer, inner }
    }

    fn placed(action: Action, gaps: Gaps) -> Rect {
        place(action, &WINDOW, &SCREEN, gaps).expect("action places a rect")
    }

    #[track_caller]
    fn assert_close(actual: f64, expected: f64) {
        assert!(
            (actual - expected).abs() < EPS,
            "expected {expected}, got {actual}"
        );
    }

    #[track_caller]
    fn assert_places(expected: [(Action, Rect); 4]) {
        for (action, rect) in expected {
            assert_eq!(placed(action, gaps(5.0, 5.0)), rect, "{action:?}");
        }
    }

    #[test]
    fn place_halves() {
        assert_places([
            (Action::LeftHalf, Rect::new(5.0, 5.0, 492.5, 790.0)),
            (Action::RightHalf, Rect::new(502.5, 5.0, 492.5, 790.0)),
            (Action::TopHalf, Rect::new(5.0, 5.0, 990.0, 392.5)),
            (Action::BottomHalf, Rect::new(5.0, 402.5, 990.0, 392.5)),
        ]);
    }

    #[test]
    fn place_quarters() {
        assert_places([
            (Action::TopLeftQuarter, Rect::new(5.0, 5.0, 492.5, 392.5)),
            (Action::TopRightQuarter, Rect::new(502.5, 5.0, 492.5, 392.5)),
            (
                Action::BottomLeftQuarter,
                Rect::new(5.0, 402.5, 492.5, 392.5),
            ),
            (
                Action::BottomRightQuarter,
                Rect::new(502.5, 402.5, 492.5, 392.5),
            ),
        ]);
    }

    #[test]
    fn place_every_action() {
        let gaps = gaps(8.0, 6.0);
        let area = SCREEN.inset(gaps.outer);
        for action in Action::iter() {
            let Some(rect) = place(action, &WINDOW, &SCREEN, gaps) else {
                continue;
            };
            assert!(
                rect.width >= 0.0 && rect.height >= 0.0,
                "{action:?}: {rect:?}"
            );
            assert!(
                rect.x >= area.x - EPS && rect.max_x() <= area.max_x() + EPS,
                "{action:?}: {rect:?}"
            );
            assert!(
                rect.y >= area.y - EPS && rect.max_y() <= area.max_y() + EPS,
                "{action:?}: {rect:?}"
            );
        }
    }

    #[test]
    fn place_thirds() {
        let gaps = gaps(5.0, 5.0);
        let area = SCREEN.inset(gaps.outer);
        let first = placed(Action::FirstThird, gaps);
        let center = placed(Action::CenterThird, gaps);
        let last = placed(Action::LastThird, gaps);

        assert_close(first.x, area.x);
        assert_close(center.x - first.max_x(), gaps.inner);
        assert_close(last.x - center.max_x(), gaps.inner);
        assert_close(last.max_x(), area.max_x());
        assert_close(center.width, first.width);
        assert_close(last.width, first.width);
    }

    #[test]
    fn place_two_thirds() {
        let gaps = gaps(5.0, 5.0);
        let area = SCREEN.inset(gaps.outer);
        let first = placed(Action::FirstThird, gaps);
        let center = placed(Action::CenterThird, gaps);
        let first_two = placed(Action::FirstTwoThirds, gaps);
        let last_two = placed(Action::LastTwoThirds, gaps);

        assert_close(first_two.x, area.x);
        assert_close(first_two.width, first.width * 2.0 + gaps.inner);
        assert_close(last_two.x, center.x);
        assert_close(last_two.max_x(), area.max_x());
    }

    #[test]
    fn place_center() {
        assert_eq!(
            placed(Action::Center, gaps(5.0, 5.0)),
            Rect::new(300.0, 250.0, 400.0, 300.0)
        );
    }
}
