use crate::geometry::Rect;

pub fn display_for(window: &Rect, visible_frames: &[Rect]) -> Option<usize> {
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
    let (x, y) = window.center();
    display_at(x, y, visible_frames)
}

pub fn display_at(x: f64, y: f64, visible_frames: &[Rect]) -> Option<usize> {
    visible_frames
        .iter()
        .enumerate()
        .min_by(|(_, a), (_, b)| distance_squared(x, y, a).total_cmp(&distance_squared(x, y, b)))
        .map(|(i, _)| i)
}

fn distance_squared(x: f64, y: f64, frame: &Rect) -> f64 {
    let dx = (frame.x - x).max(x - frame.max_x()).max(0.0);
    let dy = (frame.y - y).max(y - frame.max_y()).max(0.0);
    dx * dx + dy * dy
}

pub fn remap_between(window: &Rect, from: &Rect, to: &Rect) -> Rect {
    let sx = to.width / from.width;
    let sy = to.height / from.height;
    Rect::new(
        to.x + (window.x - from.x) * sx,
        to.y + (window.y - from.y) * sy,
        window.width * sx,
        window.height * sy,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    const LEFT: Rect = Rect::new(0.0, 0.0, 1000.0, 800.0);
    const RIGHT: Rect = Rect::new(1000.0, 0.0, 1000.0, 800.0);

    #[test]
    fn display_for_with_overlapping_window() {
        let mostly_left = Rect::new(700.0, 100.0, 400.0, 300.0);
        let mostly_right = Rect::new(900.0, 100.0, 400.0, 300.0);
        assert_eq!(display_for(&mostly_left, &[LEFT, RIGHT]), Some(0));
        assert_eq!(display_for(&mostly_right, &[LEFT, RIGHT]), Some(1));
    }

    #[test]
    fn display_for_with_offscreen_window() {
        let above_right = Rect::new(1400.0, -5000.0, 200.0, 150.0);
        assert_eq!(display_for(&above_right, &[LEFT, RIGHT]), Some(1));
    }

    #[test]
    fn display_at_inside_frame() {
        assert_eq!(display_at(500.0, 400.0, &[LEFT, RIGHT]), Some(0));
        assert_eq!(display_at(1500.0, 400.0, &[LEFT, RIGHT]), Some(1));
    }

    #[test]
    fn display_at_off_every_frame() {
        let narrow_right = Rect::new(1000.0, 0.0, 400.0, 300.0);
        assert_eq!(display_at(1200.0, -40.0, &[LEFT, RIGHT]), Some(1));
        assert_eq!(display_at(-30.0, 900.0, &[LEFT, RIGHT]), Some(0));
        assert_eq!(display_at(990.0, -40.0, &[LEFT, narrow_right]), Some(0));
    }

    #[test]
    fn remap_between_across_different_sizes() {
        let from = Rect::new(0.0, 0.0, 1000.0, 800.0);
        let to = Rect::new(1000.0, 0.0, 2000.0, 1200.0);
        let window = Rect::new(100.0, 200.0, 400.0, 300.0);
        assert_eq!(
            remap_between(&window, &from, &to),
            Rect::new(1200.0, 300.0, 800.0, 450.0)
        );
    }
}
