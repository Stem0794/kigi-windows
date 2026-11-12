//! Bounded structural diagnostics, kept in memory until explicit copy/export.
//! Callers must never pass window titles, typed text, touch samples or page URLs.
use std::{cell::RefCell, collections::VecDeque, fmt::Write, time::Instant};

const CAPACITY: usize = 200;
const DETAIL_LIMIT: usize = 400;

struct Log {
    started: Instant,
    sequence: u64,
    entries: VecDeque<String>,
}
impl Log {
    fn new() -> Self {
        Self {
            started: Instant::now(),
            sequence: 0,
            entries: VecDeque::new(),
        }
    }
    fn record(&mut self, event: &'static str, detail: &str) {
        self.sequence += 1;
        let safe: String = detail
            .chars()
            .take(DETAIL_LIMIT)
            .map(|c| if c.is_control() { ' ' } else { c })
            .collect();
        if self.entries.len() == CAPACITY {
            self.entries.pop_front();
        }
        self.entries.push_back(format!(
            "{:04} +{:08}ms {event} {safe}",
            self.sequence,
            self.started.elapsed().as_millis()
        ));
    }
    fn report(&self, metadata: &str) -> String {
        let mut text = format!("Kigi Windows — diagnostic {}\n{}\n\nDerniers événements de cette session ({} / {} conservés) :\n",
            env!("CARGO_PKG_VERSION"), metadata, self.entries.len(), self.sequence);
        for entry in &self.entries {
            let _ = writeln!(text, "{entry}");
        }
        text.push_str("\nPas de frappes, titres, contenu des applications ou coordonnées de doigts.\nLes événements restent en mémoire ; aucun envoi automatique.\n");
        text
    }
}
thread_local! { static LOG: RefCell<Option<Log>> = const { RefCell::new(None) }; }

pub fn record(event: &'static str, detail: impl AsRef<str>) {
    let _ = LOG.try_with(|slot| {
        if let Ok(mut state) = slot.try_borrow_mut() {
            state
                .get_or_insert_with(Log::new)
                .record(event, detail.as_ref());
        }
    });
}
pub fn report(metadata: &str) -> String {
    LOG.try_with(|slot| {
        if let Ok(state) = slot.try_borrow() {
            if let Some(log) = state.as_ref() {
                return log.report(metadata);
            }
        }
        Log::new().report(metadata)
    })
    .unwrap_or_else(|_| "Diagnostic indisponible pendant l'arrêt.".into())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn bounded_recent_history_and_single_line_unicode_details() {
        let mut log = Log::new();
        for _ in 0..CAPACITY + 7 {
            log.record("action", "permission=false");
        }
        assert_eq!(log.entries.len(), CAPACITY);
        assert!(log.entries.front().unwrap().starts_with("0008 "));
        log.record("error", &format!("ligne\n{}", "é".repeat(500)));
        let last = log.entries.back().unwrap();
        assert!(!last.contains('\n'));
        assert_eq!(
            last.split("error ").nth(1).unwrap().chars().count(),
            DETAIL_LIMIT
        );
        assert!(log
            .report("architecture=test")
            .contains("200 / 208 conservés"));
        record("test.status", "permission=false");
        assert!(report("architecture=test").contains("test.status permission=false"));
    }
}
