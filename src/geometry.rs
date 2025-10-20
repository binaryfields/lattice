#[derive(Clone, Copy, PartialEq, Debug, Default)]
pub struct Rect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

impl Rect {
    pub const fn new(x: f64, y: f64, width: f64, height: f64) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    pub fn max_x(&self) -> f64 {
        self.x + self.width
    }

    pub fn max_y(&self) -> f64 {
        self.y + self.height
    }

    pub fn center(&self) -> (f64, f64) {
        (self.x + self.width / 2.0, self.y + self.height / 2.0)
    }

    pub fn contains(&self, x: f64, y: f64) -> bool {
        x >= self.x && x < self.max_x() && y >= self.y && y < self.max_y()
    }

    pub fn intersection_area(&self, other: &Rect) -> f64 {
        let w = self.max_x().min(other.max_x()) - self.x.max(other.x);
        let h = self.max_y().min(other.max_y()) - self.y.max(other.y);
        if w > 0.0 && h > 0.0 { w * h } else { 0.0 }
    }

    pub fn inset(&self, d: f64) -> Rect {
        Rect::new(
            self.x + d,
            self.y + d,
            (self.width - 2.0 * d).max(0.0),
            (self.height - 2.0 * d).max(0.0),
        )
    }

    pub fn centered_in(&self, container: &Rect) -> Rect {
        Rect::new(
            container.x + (container.width - self.width) / 2.0,
            container.y + (container.height - self.height) / 2.0,
            self.width,
            self.height,
        )
    }

    pub fn column(&self, gap: f64, n: u32, col: u32, span: u32) -> Rect {
        let (offset, width) = track_span(self.width, gap, n, col, span);
        Rect::new(self.x + offset, self.y, width, self.height)
    }

    pub fn row(&self, gap: f64, n: u32, row: u32, span: u32) -> Rect {
        let (offset, height) = track_span(self.height, gap, n, row, span);
        Rect::new(self.x, self.y + offset, self.width, height)
    }

    pub fn flip_vertical(&self, primary_display_height: f64) -> Rect {
        Rect::new(
            self.x,
            primary_display_height - (self.y + self.height),
            self.width,
            self.height,
        )
    }
}

fn track_span(length: f64, gap: f64, n: u32, start: u32, span: u32) -> (f64, f64) {
    let n = f64::from(n);
    let track = (length - gap * (n - 1.0)) / n;
    let offset = f64::from(start) * (track + gap);
    let length = f64::from(span) * track + (f64::from(span) - 1.0) * gap;
    (offset, length)
}
