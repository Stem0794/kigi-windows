//! Local P0 diagnostic only. No global gesture actions.
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
            (fingers == 1 && c.x >= 0.0 && c.x <= 0.2 && c.y >= 0.8 && c.y <= 1.0).then_some(c);
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
        for start in [contact(0.45, 0.9, 0), contact(0.1, 0.4, 0)] {
            p.begin(start, 1);
            assert!(!p.moved(contact(start.x, start.y - 0.3, 200), 1));
        }
        p.begin(contact(0.1, 0.9, 0), 1);
        assert!(!p.moved(contact(0.1, 0.6, 200), 2));
        assert!(!p.moved(contact(0.1, 0.5, 300), 1));
        p.begin(contact(0.1, 0.9, 0), 1);
        assert!(!p.moved(contact(0.1, 0.6, 1501), 1));
    }
}
