//! Pure contact classifiers; coordinates never leave transient recognition state.
#[derive(Clone, Copy)]
pub struct Contact {
    pub identity: usize,
    pub device: usize,
    pub x: f64,
    pub y: f64,
    pub ms: u64,
}

#[derive(Default)]
pub struct CornerProbe {
    start: Option<Contact>,
}

impl CornerProbe {
    pub fn cancel(&mut self) {
        self.start = None;
    }

    pub fn begin(&mut self, c: Contact, fingers: usize) {
        self.start =
            (fingers == 1 && c.x >= 0.0 && c.x <= 0.1 && c.y >= 0.8 && c.y <= 1.0).then_some(c);
    }

    pub fn moved(&mut self, c: Contact, fingers: usize) -> bool {
        let Some(start) = self.start else {
            return false;
        };
        if fingers != 1
            || c.identity != start.identity
            || c.device != start.device
            || !c.x.is_finite()
            || !c.y.is_finite()
            || !(0.0..=1.0).contains(&c.y)
            || !(0.0..=1.0).contains(&c.x)
            || c.ms < start.ms
            || c.ms - start.ms > 1500
            || (c.x - start.x).abs() > 0.12
            || c.y > start.y + 0.03
        {
            self.cancel();
            return false;
        }
        if start.y - c.y >= 0.22 {
            self.cancel(); // Cannot rearm until a new began event.
            return true;
        }
        false
    }
}

/// Personal profile from the supplied Hammerspoon reference: whole-height edges.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Edge {
    Left,
    Right,
}
#[derive(Default)]
pub struct EdgeProbe {
    start: Option<(Contact, Edge)>,
    blocked: bool,
    cooldown_until: u64,
}
impl EdgeProbe {
    pub fn cancel(&mut self) {
        self.start = None;
        self.blocked = true;
    }
    /// Only an observed all-fingers-up boundary rearms a cancelled/fired sequence.
    pub fn release(&mut self) {
        self.start = None;
        self.blocked = false;
    }
    pub fn begin(&mut self, c: Contact, fingers: usize) {
        if self.blocked {
            return;
        }
        if self.start.is_some()
            || fingers != 1
            || c.ms < self.cooldown_until
            || !c.x.is_finite()
            || !c.y.is_finite()
            || !(0.0..=1.0).contains(&c.y)
        {
            self.cancel();
            return;
        }
        let edge = if (0.0..=0.1).contains(&c.x) {
            Some(Edge::Left)
        } else if (0.8..=1.0).contains(&c.x) {
            Some(Edge::Right)
        } else {
            None
        };
        if let Some(edge) = edge {
            self.start = Some((c, edge));
        } else {
            self.cancel();
        }
    }
    pub fn moved(&mut self, c: Contact, fingers: usize) -> Option<Edge> {
        let (start, edge) = self.start?;
        if fingers != 1
            || c.identity != start.identity
            || c.device != start.device
            || !c.x.is_finite()
            || !c.y.is_finite()
            || !(0.0..=1.0).contains(&c.x)
            || !(0.0..=1.0).contains(&c.y)
            || c.ms < start.ms
            || c.ms - start.ms > 1500
            || (c.x - start.x).abs() > 0.12
            || c.y > start.y + 0.03
        {
            self.cancel();
            return None;
        }
        if start.y - c.y >= 0.22 {
            self.cancel();
            self.cooldown_until = c.ms.saturating_add(1000);
            Some(edge)
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn contact(x: f64, y: f64, ms: u64) -> Contact {
        Contact {
            identity: 1,
            device: 1,
            x,
            y,
            ms,
        }
    }
    #[test]
    fn downward_only_once_and_requires_begin() {
        let mut p = CornerProbe::default();
        assert!(!p.moved(contact(0.1, 0.5, 200), 1));
        p.begin(contact(0.1, 0.9, 0), 1);
        assert!(!p.moved(contact(0.1, 0.71, 100), 1));
        assert!(p.moved(contact(0.15, 0.6, 200), 1));
        assert!(!p.moved(contact(0.15, 0.5, 300), 1));
    }
    #[test]
    fn rejects_wrong_zone_multitouch_and_timeout() {
        let mut p = CornerProbe::default();
        for start in [
            contact(0.1001, 0.9, 0),
            contact(0.45, 0.9, 0),
            contact(0.1, 0.4, 0),
        ] {
            p.begin(start, 1);
            assert!(!p.moved(contact(start.x, start.y - 0.3, 200), 1));
        }
        p.begin(contact(0.1, 0.9, 0), 1);
        assert!(!p.moved(contact(0.1, 0.6, 200), 2));
        assert!(!p.moved(contact(0.1, 0.5, 300), 1));
        p.begin(contact(0.1, 0.9, 0), 1);
        assert!(!p.moved(contact(0.1, 0.6, 1501), 1));
    }
    #[test]
    fn edges_fire_once_with_shared_cooldown_and_accept_middle_height() {
        let mut p = EdgeProbe::default();
        p.begin(contact(0.1, 0.5, 0), 1);
        assert_eq!(p.moved(contact(0.12, 0.2, 100), 1), Some(Edge::Left));
        p.begin(contact(0.1, 0.5, 200), 1); // duplicate begin cannot rearm
        assert_eq!(p.moved(contact(0.1, 0.2, 300), 1), None);
        p.release();
        p.begin(contact(0.8, 0.9, 400), 1); // other edge shares cooldown
        assert_eq!(p.moved(contact(0.8, 0.6, 500), 1), None);
        p.release();
        p.begin(contact(0.8, 0.9, 1100), 1);
        assert_eq!(p.moved(contact(0.8, 0.6, 1200), 1), Some(Edge::Right));
    }
    #[test]
    fn edges_cancel_until_lift_and_reject_invalid_or_interrupted_sequences() {
        let mut p = EdgeProbe::default();
        assert_eq!(p.moved(contact(0.05, 0.3, 100), 1), None);
        p.begin(contact(0.05, 0.8, 200), 1);
        assert_eq!(p.moved(contact(0.05, 0.7, 250), 2), None);
        p.begin(contact(0.05, 0.8, 300), 1);
        assert_eq!(p.moved(contact(0.05, 0.4, 400), 1), None);
        for start in [contact(0.1001, 0.8, 500), contact(f64::NAN, 0.8, 500)] {
            p.release();
            p.begin(start, 1);
            assert_eq!(p.moved(contact(0.05, 0.4, 600), 1), None);
        }
        for bad in [
            Contact {
                device: 2,
                ..contact(0.05, 0.4, 600)
            },
            contact(0.05, 0.9, 600),
            contact(0.3, 0.4, 600),
            contact(0.05, 0.4, 2001),
        ] {
            p.release();
            p.begin(contact(0.05, 0.8, 500), 1);
            assert_eq!(p.moved(bad, 1), None);
        }
    }
}
