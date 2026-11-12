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

/// Relative to one display's usable area, never physical pixels or a Space ID.
#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NormalizedRect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}
impl NormalizedRect {
    pub fn valid(self) -> bool {
        [self.x, self.y, self.width, self.height]
            .iter()
            .all(|n| n.is_finite())
            && self.x >= 0.0
            && self.y >= 0.0
            && self.width > 0.0
            && self.height > 0.0
            && self.x + self.width <= 1.000001
            && self.y + self.height <= 1.000001
    }
    pub fn capture(frame: Rect, display: Rect) -> Option<Self> {
        if display.width <= 0.0 || display.height <= 0.0 {
            return None;
        }
        // Ignore only normal AX rounding/title-bar differences, not off-screen layouts.
        if frame.x < display.x - 8.0
            || frame.y < display.y - 8.0
            || frame.x + frame.width > display.x + display.width + 8.0
            || frame.y + frame.height > display.y + display.height + 8.0
        {
            return None;
        }
        let x = ((frame.x - display.x) / display.width).clamp(0.0, 1.0);
        let y = ((frame.y - display.y) / display.height).clamp(0.0, 1.0);
        let r = Self {
            x,
            y,
            width: (frame.width / display.width).min(1.0 - x),
            height: (frame.height / display.height).min(1.0 - y),
        };
        r.valid().then_some(r)
    }
    pub fn on_display(self, display: Rect) -> Rect {
        Rect {
            x: display.x + self.x * display.width,
            y: display.y + self.y * display.height,
            width: self.width * display.width,
            height: self.height * display.height,
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum RestoreDecision {
    AlreadyRestored,
    Restore,
    UserMoved,
    Unknown,
}
pub fn restore_decision(current: Rect, before: Rect, applied: Option<Rect>) -> RestoreDecision {
    if current.approximately(before) {
        RestoreDecision::AlreadyRestored
    } else if applied.is_some_and(|r| current.approximately(r)) {
        RestoreDecision::Restore
    } else if applied.is_some() {
        RestoreDecision::UserMoved
    } else {
        RestoreDecision::Unknown
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
    #[test]
    fn saved_geometry_scales_and_restore_respects_manual_changes() {
        let display = Rect {
            x: -1440.0,
            y: 24.0,
            width: 1440.0,
            height: 860.0,
        };
        let half = NormalizedRect::capture(display.right_half(), display).unwrap();
        let bigger = Rect {
            x: 0.0,
            y: 24.0,
            width: 1920.0,
            height: 1080.0,
        };
        assert!(half.on_display(bigger).approximately(bigger.right_half()));
        assert!(!NormalizedRect {
            x: f64::NAN,
            ..half
        }
        .valid());
        assert!(NormalizedRect::capture(
            Rect {
                x: -1500.0,
                ..display
            },
            display
        )
        .is_none());
        assert_eq!(
            restore_decision(display, display, None),
            RestoreDecision::AlreadyRestored
        );
        assert_eq!(
            restore_decision(display.right_half(), display, Some(display.right_half())),
            RestoreDecision::Restore
        );
        assert_eq!(
            restore_decision(display.left_half(), display, Some(display.right_half())),
            RestoreDecision::UserMoved
        );
        assert_eq!(
            restore_decision(display.left_half(), display, Some(display.left_half())),
            RestoreDecision::Restore
        );
        assert_eq!(
            restore_decision(display.left_half(), display, None),
            RestoreDecision::Unknown
        );
    }
}
