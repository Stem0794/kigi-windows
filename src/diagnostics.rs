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

// Successful samples only; failed/cancelled operations never improve the p95.
const SAMPLE_CAPACITY: usize = 64;
#[derive(Clone, Copy)]
pub enum Metric {
    Geometry,
    Activation,
    Maximize,
}
impl Metric {
    fn label(self) -> &'static str {
        match self {
            Self::Geometry => "Écriture géométrie AX",
            Self::Activation => "Activation d’app (avec placement si demandé)",
            Self::Maximize => "Maximisation (transition comprise)",
        }
    }
}
#[derive(Default)]
struct Timing {
    samples: VecDeque<u64>,
    failures: u64,
}
impl Timing {
    fn add(&mut self, microseconds: u64, success: bool) {
        if !success {
            self.failures = self.failures.saturating_add(1);
            return;
        }
        if self.samples.len() == SAMPLE_CAPACITY {
            self.samples.pop_front();
        }
        self.samples.push_back(microseconds);
    }
    fn p95(&self) -> Option<u64> {
        let mut values: Vec<_> = self.samples.iter().copied().collect();
        values.sort_unstable();
        if values.is_empty() {
            None
        } else {
            Some(values[(values.len() * 95).div_ceil(100) - 1])
        }
    }
}
thread_local! {static TIMINGS: RefCell<[Timing;3]> = RefCell::new(std::array::from_fn(|_|Timing::default()));}
pub fn timing(kind: Metric, elapsed: std::time::Duration, success: bool) {
    TIMINGS.with(|t| {
        t.borrow_mut()[kind as usize].add(elapsed.as_micros().min(u64::MAX as u128) as u64, success)
    });
}
#[cfg(target_os = "macos")]
pub fn rss_bytes() -> Option<u64> {
    let mut info = std::mem::MaybeUninit::<libc::proc_taskinfo>::uninit();
    let size = std::mem::size_of::<libc::proc_taskinfo>() as i32;
    let read = unsafe {
        libc::proc_pidinfo(
            std::process::id() as i32,
            libc::PROC_PIDTASKINFO,
            0,
            info.as_mut_ptr().cast(),
            size,
        )
    };
    if read == size {
        Some(unsafe { info.assume_init() }.pti_resident_size)
    } else {
        None
    }
}
#[cfg(target_os = "macos")]
pub fn peak_rss_bytes() -> Option<u64> {
    let mut usage = std::mem::MaybeUninit::<libc::rusage>::uninit();
    if unsafe { libc::getrusage(libc::RUSAGE_SELF, usage.as_mut_ptr()) } != 0 {
        return None;
    }
    // Darwin ru_maxrss is bytes, unlike Linux. This is a peak, not current RSS.
    u64::try_from(unsafe { usage.assume_init() }.ru_maxrss).ok()
}
pub fn performance_report() -> String {
    let mut out = String::new();
    #[cfg(target_os = "macos")]
    match rss_bytes() {
        Some(bytes) => {
            let _ = writeln!(
                out,
                "RSS actuelle : {:.1} Mio ({bytes} octets).",
                bytes as f64 / 1048576.0
            );
        }
        None => out.push_str("RSS actuelle : indisponible.\n"),
    }
    #[cfg(target_os = "macos")]
    match peak_rss_bytes() {
        Some(bytes) => {
            let _ = writeln!(
                out,
                "RSS maximale depuis le lancement : {:.1} Mio ({bytes} octets).",
                bytes as f64 / 1048576.0
            );
        }
        None => out.push_str("RSS maximale : indisponible.\n"),
    }
    TIMINGS.with(|t| {
        for kind in [Metric::Geometry, Metric::Activation, Metric::Maximize] {
            let t = t.borrow();
            let series = &t[kind as usize];
            match series.p95() {
                Some(us) => {
                    let _ = writeln!(
                        out,
                        "{} : p95 {:.2} ms ; {} succès conservés, {} échec(s).",
                        kind.label(),
                        us as f64 / 1000.0,
                        series.samples.len(),
                        series.failures
                    );
                }
                None => {
                    let _ = writeln!(
                        out,
                        "{} : non mesuré (aucun succès), {} échec(s).",
                        kind.label(),
                        series.failures
                    );
                }
            }
        }
    });
    out.push_str("64 derniers succès par mesure, pour cette session uniquement. Le p95 sur peu d’échantillons reste indicatif. La géométrie exclut les dialogues et le lancement d’app ; les autres mesures incluent l’attente de l’action. Aucune mesure CPU/latence trackpad n’est déduite de ces valeurs.");
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn bounded_success_percentile_excludes_failures_and_empty_is_not_zero() {
        let mut timing = Timing::default();
        assert_eq!(timing.p95(), None);
        for value in 1..=100 {
            timing.add(value, true);
        }
        assert_eq!(timing.samples.len(), 64);
        assert_eq!(timing.p95(), Some(97));
        timing.add(0, false);
        assert_eq!(timing.failures, 1);
        assert_eq!(timing.p95(), Some(97));
        super::timing(Metric::Geometry, std::time::Duration::ZERO, false);
        assert!(performance_report().contains("non mesuré"));
    }
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
