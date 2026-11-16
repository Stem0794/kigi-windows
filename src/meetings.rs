//! Explicit, local Teams profile. No universal meeting-title heuristic.
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Settings {
    pub enabled: bool,
    #[serde(default)]
    pub distinguish_chat: bool,
    #[serde(default)]
    pub end_on_title_change: bool,
    pub chat_title: String,
    pub meeting_title: String,
    pub meeting_right: bool,
    pub chat_percent: u8,
    pub maximize_after: bool,
}
impl Default for Settings {
    fn default() -> Self {
        Self {
            enabled: false,
            distinguish_chat: false,
            end_on_title_change: false,
            chat_title: String::new(),
            meeting_title: String::new(),
            meeting_right: true,
            chat_percent: 50,
            maximize_after: true,
        }
    }
}
impl Settings {
    pub fn validate(&self) -> Result<(), String> {
        if !(20..=80).contains(&self.chat_percent) {
            return Err("La largeur du chat doit être comprise entre 20 et 80 %.".into());
        }
        for text in [&self.chat_title, &self.meeting_title] {
            if text.chars().count() > 256 || text.chars().any(char::is_control) {
                return Err("Les repères doivent contenir au maximum 256 caractères, sans retour à la ligne.".into());
            }
        }
        if self.enabled
            && (self.chat_title.trim().is_empty()
                || self.meeting_title.trim().is_empty()
                || self
                    .chat_title
                    .trim()
                    .eq_ignore_ascii_case(self.meeting_title.trim()))
        {
            return Err("Identifiez le chat et la réunion avec deux repères de titre distincts avant d’activer.".into());
        }
        Ok(())
    }
    pub fn meeting_matches(&self, title: &str) -> bool {
        if !Self::matches(title, &self.meeting_title) {
            return false;
        }
        if !self.distinguish_chat {
            return true;
        }
        if Self::matches(title, &self.chat_title) {
            return false;
        }
        let prefix = title
            .split('|')
            .next()
            .unwrap_or(title)
            .trim()
            .to_lowercase();
        ![
            "chat",
            "conversation",
            "activity",
            "activité",
            "calendar",
            "calendrier",
            "calls",
            "appels",
            "teams",
            "équipes",
            "files",
            "fichiers",
            "settings",
            "paramètres",
            "window",
            "fenêtre",
        ]
        .contains(&prefix.as_str())
    }
    pub fn matches(title: &str, marker: &str) -> bool {
        !marker.trim().is_empty() && title.to_lowercase().contains(&marker.trim().to_lowercase())
    }
}
/// Exactly one non-overlapping pair; missing meeting is a normal idle state.
pub fn classify(titles: &[&str], settings: &Settings) -> Result<Option<(usize, usize)>, String> {
    let chat: Vec<_> = titles
        .iter()
        .enumerate()
        .filter(|(_, title)| Settings::matches(title, &settings.chat_title))
        .map(|(i, _)| i)
        .collect();
    let meeting: Vec<_> = titles
        .iter()
        .enumerate()
        .filter(|(_, title)| settings.meeting_matches(title))
        .map(|(i, _)| i)
        .collect();
    if meeting.is_empty() {
        return Ok(None);
    }
    if chat.len() != 1 || meeting.len() != 1 || chat[0] == meeting[0] {
        return Err("Détection ambiguë : il faut exactement un chat et une réunion distincts. Affinez les repères.".into());
    }
    Ok(Some((chat[0], meeting[0])))
}
#[derive(Clone, Copy, Debug, Default, PartialEq)]
enum Phase {
    #[default]
    Idle,
    Candidate(u64),
    Active,
    Ending(u64),
    Inhibited,
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Effect {
    None,
    Start,
    End,
}
#[derive(Default)]
pub struct Gate {
    phase: Phase,
}
impl Gate {
    pub fn activate(&mut self) {
        self.phase = Phase::Active;
    }
    pub fn inhibit(&mut self) {
        self.phase = Phase::Inhibited;
    }
    pub fn reset_candidate(&mut self) {
        if matches!(self.phase, Phase::Candidate(_)) {
            self.phase = Phase::Idle;
        }
    }
    pub fn step(&mut self, present: Option<bool>, now: u64) -> Effect {
        use Phase::*;
        let mut effect = Effect::None;
        self.phase = match (self.phase, present) {
            (Idle, Some(true)) => Candidate(now),
            (Candidate(since), Some(true)) if now.saturating_sub(since) >= 1000 => {
                effect = Effect::Start;
                Active
            }
            (Candidate(_), Some(false) | None) => Idle,
            (Active, Some(false)) => Ending(now),
            (Ending(since), Some(false)) if now.saturating_sub(since) >= 2000 => {
                effect = Effect::End;
                Idle
            }
            (Ending(_), Some(true) | None) => Active,
            (Inhibited, Some(false)) => Idle,
            (phase, _) => phase,
        };
        effect
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn calibrated_shared_suffix_excludes_chat_and_ordinary_pages() {
        let settings = Settings {
            chat_title: "Chat |".into(),
            meeting_title: "Company".into(),
            distinguish_chat: true,
            ..Settings::default()
        };
        assert_eq!(
            classify(
                &[
                    "Chat | Company",
                    "Calendar | Company",
                    "Customer call | Company"
                ],
                &settings
            )
            .unwrap(),
            Some((0, 2))
        );
        assert!(classify(
            &["Chat | Company", "Call one | Company", "Call two | Company"],
            &settings
        )
        .is_err());
        assert!(!settings.meeting_matches("Settings | Company"));
    }
    #[test]
    fn debounce_unknown_and_reappearance_never_restore_early() {
        let mut gate = Gate::default();
        assert_eq!(gate.step(Some(true), 0), Effect::None);
        assert_eq!(gate.step(Some(true), 999), Effect::None);
        assert_eq!(gate.step(None, 1000), Effect::None);
        gate.step(Some(true), 2000);
        assert_eq!(gate.step(Some(true), 3000), Effect::Start);
        gate.step(Some(false), 4000);
        gate.step(None, 6000);
        assert_eq!(gate.step(Some(false), 7000), Effect::None);
        gate.step(Some(true), 8000);
        gate.step(Some(false), 9000);
        assert_eq!(gate.step(Some(false), 10999), Effect::None);
        assert_eq!(gate.step(Some(false), 11000), Effect::End);
    }
    #[test]
    fn manual_override_holds_until_confirmed_absence() {
        let mut gate = Gate::default();
        gate.step(Some(true), 0);
        gate.step(Some(true), 1000);
        gate.inhibit();
        for value in [Some(true), None, Some(true)] {
            assert_eq!(gate.step(value, 5000), Effect::None);
        }
        gate.step(Some(false), 6000);
        gate.step(Some(true), 7000);
        assert_eq!(gate.step(Some(true), 8000), Effect::Start);
    }
    #[test]
    fn ambiguous_or_shared_windows_never_form_a_pair() {
        let settings = Settings {
            chat_title: "Chat".into(),
            meeting_title: "Call".into(),
            ..Settings::default()
        };
        assert_eq!(
            classify(&["Chat", "Call"], &settings).unwrap(),
            Some((0, 1))
        );
        assert_eq!(classify(&["Chat"], &settings).unwrap(), None);
        assert!(classify(&["Chat", "Call 1", "Call 2"], &settings).is_err());
        assert!(classify(&["Chat 1", "Chat 2", "Call"], &settings).is_err());
        assert!(classify(&["Chat Call"], &settings).is_err());
        assert!(classify(&["Call"], &settings).is_err());
    }
    #[test]
    fn profile_requires_explicit_signatures_and_safe_proportions() {
        let mut settings = Settings::default();
        assert!(settings.validate().is_ok());
        settings.enabled = true;
        assert!(settings.validate().is_err());
        settings.chat_title = "Chat | Teams".into();
        settings.meeting_title = "Réunion | Teams".into();
        assert!(settings.validate().is_ok());
        assert!(!Settings::matches("Teams", ""));
        assert!(Settings::matches("RÉUNION | Teams", "réunion | teams"));
        settings.chat_percent = 0;
        assert!(settings.validate().is_err());
    }
}
