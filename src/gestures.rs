//! Pure contact classifiers; coordinates never leave transient recognition state.
#[derive(Clone, Copy)]
pub struct Contact {
    pub identity: usize,
    pub device: usize,
    pub x: f64,
    pub y: f64,
    pub ms: u64,
}

#[cfg(test)]
#[derive(Default)]
pub struct CornerProbe {
    start: Option<Contact>,
}

#[cfg(test)]
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

#[derive(Default, Clone, Copy)]
pub struct TrialEvidence {
    pub events: u64,
    pub contacts: u64,
    pub samples: u64,
    pub began: u64,
    pub edge_starts: u64,
    pub recognized: u64,
}
impl TrialEvidence {
    pub fn diagnosis(self) -> &'static str {
        if self.events == 0 {
            "Aucun événement global reçu. Testez d’abord en local, puis vérifiez la surveillance de l’entrée."
        } else if self.contacts == 0 {
            "Événements reçus sans contacts tactiles. Élargir la zone ne résout pas ce transport."
        } else if self.samples == 0 {
            "Contacts reçus, mais aucun échantillon à un seul doigt sans clic. Levez les autres doigts et recommencez."
        } else if self.began == 0 {
            "Contacts reçus sans début de contact à un doigt. Levez tous les doigts avant l’essai."
        } else if self.edge_starts == 0 {
            "Débuts de contacts reçus hors des bords à 10 %. Essayez le test local pour voir le point de départ."
        } else if self.recognized == 0 {
            "Départ dans un bord reçu, mais aucun geste complet. Glissez vers l’intérieur sur 10 % de la largeur, en 1,5 s, horizontalement, à un doigt."
        } else {
            "Au moins un geste global a été observé. La fiabilité et les faux positifs restent à valider sur ce Mac."
        }
    }
}

/// Short inward swipe: outermost 10%, minimum travel 10% of trackpad width.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Edge {
    Left,
    Right,
}
#[derive(Default)]
pub struct EdgeProbe {
    start: Option<(Contact, Edge)>,
    blocked: bool,
    returning: Option<(Contact, Option<Edge>)>,
}
impl EdgeProbe {
    pub fn cancel(&mut self) {
        self.start = None;
        self.returning = None;
        self.blocked = true;
    }
    /// Lift always resets; a successful swipe can also rearm by returning to an edge.
    pub fn release(&mut self) {
        self.start = None;
        self.returning = None;
        self.blocked = false;
    }
    pub fn armed(&self) -> bool {
        self.start.is_some()
    }
    pub fn begin(&mut self, c: Contact, fingers: usize) {
        if self.blocked {
            return;
        }
        if self.start.is_some()
            || fingers != 1
            || !c.x.is_finite()
            || !c.y.is_finite()
            || !(0.0..=1.0).contains(&c.x)
            || !(0.0..=1.0).contains(&c.y)
        {
            self.cancel();
            return;
        }
        let edge = if (0.0..=0.10).contains(&c.x) {
            Some(Edge::Left)
        } else if (0.90..=1.0).contains(&c.x) {
            Some(Edge::Right)
        } else {
            None
        };
        if let Some(edge) = edge {
            self.start = Some((c, edge));
        } else {
            // A finger may land in the center, then move to a real edge before swiping.
            self.returning = Some((c, None));
        }
    }
    pub fn moved(&mut self, c: Contact, fingers: usize) -> Option<Edge> {
        if let Some((last, fired_edge)) = self.returning {
            if fingers != 1
                || c.identity != last.identity
                || c.device != last.device
                || !c.x.is_finite()
                || !c.y.is_finite()
                || !(0.0..=1.0).contains(&c.x)
                || !(0.0..=1.0).contains(&c.y)
                || c.ms < last.ms
            {
                self.cancel();
                return None;
            }
            let edge = if c.x <= 0.10 {
                Some(Edge::Left)
            } else if c.x >= 0.90 {
                Some(Edge::Right)
            } else {
                None
            };
            if let Some(edge) = edge {
                // Returning to the same edge needs real reversal, not boundary jitter.
                let reversed = match edge {
                    Edge::Left => last.x - c.x,
                    Edge::Right => c.x - last.x,
                };
                if Some(edge) != fired_edge || reversed + 1e-9 >= 0.06 {
                    self.returning = None;
                    self.blocked = false;
                    self.start = Some((c, edge));
                }
            }
            return None;
        }
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
            || (c.y - start.y).abs() > 0.12
            || match edge {
                Edge::Left => c.x < start.x - 0.03,
                Edge::Right => c.x > start.x + 0.03,
            }
        {
            self.cancel();
            return None;
        }
        let inward = match edge {
            Edge::Left => c.x - start.x,
            Edge::Right => start.x - c.x,
        };
        let horizontal = inward + 1e-9 >= (c.y - start.y).abs() * 1.5;
        if inward + 1e-9 >= 0.10 && horizontal {
            self.cancel();
            self.returning = Some((c, Some(edge)));
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
    fn trial_distinguishes_transport_phases_zones_and_recognition() {
        let mut e = TrialEvidence::default();
        assert!(e.diagnosis().starts_with("Aucun événement"));
        e.events = 1;
        assert!(e.diagnosis().contains("sans contacts"));
        e.contacts = 1;
        assert!(e.diagnosis().contains("aucun échantillon"));
        e.samples = 1;
        assert!(e.diagnosis().contains("sans début"));
        e.began = 1;
        assert!(e.diagnosis().contains("hors des bords"));
        e.edge_starts = 1;
        assert!(e.diagnosis().contains("aucun geste complet"));
        e.recognized = 1;
        assert!(e.diagnosis().contains("fiabilité"));
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
    fn edges_short_inward_swipe_fires_once_until_rearmed() {
        let mut p = EdgeProbe::default();
        p.begin(contact(0.10, 0.5, 0), 1);
        assert_eq!(p.moved(contact(0.19, 0.55, 100), 1), None);
        assert_eq!(p.moved(contact(0.20, 0.56, 140), 1), Some(Edge::Left));
        assert_eq!(p.moved(contact(0.7, 0.56, 200), 1), None);
        p.begin(contact(0.10, 0.5, 210), 1);
        assert_eq!(p.moved(contact(0.5, 0.5, 240), 1), None);
        p.release();
        p.begin(contact(0.90, 0.6, 400), 1);
        assert_eq!(p.moved(contact(0.5, 0.6, 500), 1), Some(Edge::Right));
        p.release();
        p.begin(contact(0.90, 0.4, 1200), 1);
        assert_eq!(p.moved(contact(0.80, 0.42, 1300), 1), Some(Edge::Right));
    }
    #[test]
    fn continuous_contact_rearms_at_edges_but_never_in_the_center() {
        let mut p = EdgeProbe::default();
        p.begin(contact(0.5, 0.5, 0), 1);
        assert!(!p.armed());
        assert_eq!(p.moved(contact(0.05, 0.5, 50), 1), None);
        assert!(p.armed());
        assert_eq!(p.moved(contact(0.15, 0.5, 100), 1), Some(Edge::Left));
        for x in [0.4, 0.3, 0.6, 0.2] {
            assert_eq!(p.moved(contact(x, 0.5, 200), 1), None);
        }
        // Same finger can rest, return to its edge, then perform another short swipe.
        assert_eq!(p.moved(contact(0.08, 0.8, 5000), 1), None);
        assert_eq!(p.moved(contact(0.18, 0.8, 5100), 1), Some(Edge::Left));
        // Going to the opposite edge arms that side without lifting, at any height.
        assert_eq!(p.moved(contact(0.93, 0.2, 5200), 1), None);
        assert_eq!(p.moved(contact(0.83, 0.2, 5300), 1), Some(Edge::Right));
        assert_eq!(p.moved(contact(0.92, 0.2, 5400), 1), None);
        assert_eq!(p.moved(contact(0.82, 0.2, 5500), 1), Some(Edge::Right));
    }
    #[test]
    fn continuous_rearm_rejects_boundary_jitter_and_contact_changes() {
        let mut p = EdgeProbe::default();
        p.begin(contact(0.0, 0.5, 0), 1);
        assert_eq!(p.moved(contact(0.10, 0.5, 100), 1), Some(Edge::Left));
        assert_eq!(p.moved(contact(0.09, 0.5, 120), 1), None);
        assert_eq!(p.moved(contact(0.20, 0.5, 140), 1), None);
        assert_eq!(p.moved(contact(0.04, 0.5, 160), 1), None);
        assert_eq!(p.moved(contact(0.14, 0.5, 180), 1), Some(Edge::Left));
        assert_eq!(p.moved(contact(0.04, 0.5, 200), 2), None);
        assert_eq!(p.moved(contact(0.14, 0.5, 220), 1), None);
        p.release();
        p.begin(contact(0.05, 0.5, 300), 1);
        assert_eq!(p.moved(contact(0.15, 0.5, 400), 1), Some(Edge::Left));
        assert_eq!(
            p.moved(
                Contact {
                    identity: 2,
                    ..contact(0.05, 0.5, 500)
                },
                1
            ),
            None
        );
        assert_eq!(p.moved(contact(0.15, 0.5, 600), 1), None);
    }
    #[test]
    fn short_swipe_uses_distance_from_start_and_rejects_jitter_and_diagonals() {
        let mut p = EdgeProbe::default();
        p.begin(contact(0.10, 0.5, 0), 1);
        assert_eq!(p.moved(contact(0.1999, 0.5, 100), 1), None);
        assert_eq!(p.moved(contact(0.20, 0.5, 110), 1), Some(Edge::Left));
        p.release();
        p.begin(contact(0.90, 0.5, 1200), 1);
        assert_eq!(p.moved(contact(0.8001, 0.5, 1250), 1), None);
        assert_eq!(p.moved(contact(0.80, 0.5, 1300), 1), Some(Edge::Right));
        p.release();
        p.begin(contact(0.05, 0.5, 2500), 1);
        assert_eq!(p.moved(contact(0.15, 0.61, 2600), 1), None);
        assert_eq!(p.moved(contact(0.25, 0.61, 2700), 1), Some(Edge::Left));
    }
    #[test]
    fn edges_ten_percent_bounds_and_direction() {
        let mut p = EdgeProbe::default();
        for x in [0.10001, 0.5, 0.89999] {
            p.release();
            p.begin(contact(x, 0.5, 0), 1);
            assert_eq!(p.moved(contact(0.5, 0.5, 100), 1), None);
        }
        p.release();
        p.begin(contact(0.04, 0.8, 0), 1);
        assert_eq!(p.moved(contact(0.3, 0.6, 100), 1), None); // vertical drift
        assert_eq!(p.moved(contact(0.5, 0.8, 200), 1), None);
        p.release();
        p.begin(contact(0.04, 0.5, 0), 1);
        assert_eq!(p.moved(contact(0.0, 0.5, 100), 1), None); // outward motion
        assert_eq!(p.moved(contact(0.5, 0.5, 200), 1), None);
        p.release();
        p.begin(contact(0.90, 0.9, 0), 1);
        assert_eq!(p.moved(contact(0.80, 0.9, 200), 1), Some(Edge::Right));
        p.release();
        p.begin(contact(0.10, 0.1, 2000), 1);
        assert_eq!(p.moved(contact(0.20, 0.1, 2100), 1), Some(Edge::Left));
    }
    #[test]
    fn edges_block_until_lift_after_invalid_contact() {
        let mut p = EdgeProbe::default();
        assert_eq!(p.moved(contact(0.5, 0.5, 100), 1), None);
        p.begin(contact(0.10, 0.5, 0), 1);
        assert_eq!(p.moved(contact(0.5, 0.5, 100), 2), None);
        p.begin(contact(0.10, 0.5, 200), 1);
        assert_eq!(p.moved(contact(0.5, 0.5, 300), 1), None);
        for bad in [
            Contact {
                device: 2,
                ..contact(0.5, 0.5, 600)
            },
            Contact {
                identity: 2,
                ..contact(0.5, 0.5, 600)
            },
            contact(0.5, 0.5, 2001),
            contact(0.5, f64::NAN, 600),
        ] {
            p.release();
            p.begin(contact(0.10, 0.5, 500), 1);
            assert_eq!(p.moved(bad, 1), None);
            assert_eq!(p.moved(contact(0.5, 0.5, 700), 1), None);
        }
    }
}
