#[derive(Clone, Copy, Debug)]
pub struct Rect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

impl Rect {
    /// AppKit bottom-left -> AX top-left, anchored to the primary display.
    pub fn from_appkit(x: f64, y: f64, width: f64, height: f64, primary_top: f64) -> Self {
        Self {
            x,
            y: primary_top - y - height,
            width,
            height,
        }
    }

    pub fn left_half(self) -> Self {
        Self {
            width: self.width / 2.0,
            ..self
        }
    }

    pub fn right_half(self) -> Self {
        Self {
            x: self.x + self.width / 2.0,
            width: self.width / 2.0,
            ..self
        }
    }

    pub fn overlap(self, other: Self) -> f64 {
        let w = (self.x + self.width).min(other.x + other.width) - self.x.max(other.x);
        let h = (self.y + self.height).min(other.y + other.height) - self.y.max(other.y);
        w.max(0.0) * h.max(0.0)
    }

    pub fn approximately(self, other: Self) -> bool {
        [
            self.x - other.x,
            self.y - other.y,
            self.width - other.width,
            self.height - other.height,
        ]
        .iter()
        .all(|v| v.abs() <= 8.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn negative_origin_and_vertical_display_conversion() {
        let r = Rect::from_appkit(-1440.0, 120.0, 1440.0, 860.0, 1080.0).right_half();
        assert_eq!((r.x, r.y, r.width, r.height), (-720.0, 100.0, 720.0, 860.0));
        let above = Rect::from_appkit(0.0, 1080.0, 1920.0, 1080.0, 1080.0);
        assert_eq!(above.y, -1080.0);
        assert_eq!(r.overlap(above), 0.0);
        assert!(r.approximately(r));
        let full = Rect::from_appkit(-1440.0, 120.0, 1440.0, 860.0, 1080.0);
        assert_eq!(full.left_half().x, -1440.0);
        assert_eq!(full.left_half().width, 720.0);
        assert_eq!(full.left_half().overlap(full.right_half()), 0.0);
    }
}
