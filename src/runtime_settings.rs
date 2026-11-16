//! Local personal-workflow preferences, separate from saved layouts and shortcuts.
use crate::shortcuts::{Command, Shortcuts};
use serde::{Deserialize, Serialize};
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", deny_unknown_fields)]
pub enum GestureTarget {
    Shortcut { key: u32, modifiers: u32 },
    Profile { profile_id: u32 },
    Disabled,
}
impl GestureTarget {
    pub fn resolve(&self, shortcuts: &Shortcuts) -> Result<Option<Command>, String> {
        match self {
            Self::Disabled=>Ok(None),
            Self::Profile{profile_id}=>Ok(Some(Command::ApplyProfile{profile_id:*profile_id})),
            Self::Shortcut{key,modifiers}=>shortcuts.entries.iter().find(|b|b.enabled&&b.key==*key&&b.modifiers==*modifiers).map(|b|Some(b.command.clone())).ok_or("Le raccourci associé au geste est absent ou désactivé. Choisissez sa nouvelle combinaison dans Trackpad.".into()),
        }
    }
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Settings {
    pub gestures_enabled: bool,
    pub left: GestureTarget,
    pub right: GestureTarget,
    pub edge_percent: u8,
    pub travel_percent: u8,
    pub duration_ms: u32,
    pub startup_work: bool,
    pub launch_missing_apps: bool,
    pub window_switcher: bool,
}
impl Default for Settings {
    fn default() -> Self {
        Self {
            gestures_enabled: false,
            left: GestureTarget::Shortcut {
                key: 17,
                modifiers: 2048,
            },
            right: GestureTarget::Shortcut {
                key: 11,
                modifiers: 2048,
            },
            edge_percent: 10,
            travel_percent: 10,
            duration_ms: 1500,
            startup_work: false,
            launch_missing_apps: true,
            window_switcher: false,
        }
    }
}
impl Settings {
    pub fn validate(&self) -> Result<(), String> {
        if !(3..=25).contains(&self.edge_percent)
            || !(5..=40).contains(&self.travel_percent)
            || !(500..=3000).contains(&self.duration_ms)
        {
            return Err("Bord3–25 %, déplacement5–40 %, durée500–3000ms.".into());
        }
        for target in [&self.left, &self.right] {
            match target {
                GestureTarget::Profile { profile_id: 0 } => {
                    return Err("Choisissez un espace existant.".into())
                }
                GestureTarget::Shortcut { key, modifiers }
                    if !crate::shortcuts::valid_combination(*key, *modifiers) =>
                {
                    return Err("Combinaison de geste invalide.".into())
                }
                _ => {}
            }
        }
        Ok(())
    }
}
/// Only the explicitly enabled system combination; other keyboard events pass through.
pub fn matches_window_tab(key: i64, flags: u64) -> bool {
    key == 48 && flags & (1 << 20) != 0 && flags & ((1 << 18) | (1 << 19) | (1 << 23)) == 0
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn system_switcher_matches_only_cmd_tab_and_reverse() {
        for key in 0..128 {
            assert_eq!(matches_window_tab(key, 1 << 20), key == 48);
        }
        assert!(matches_window_tab(48, (1 << 20) | (1 << 17)));
        for extra in [0, 1 << 18, 1 << 19, 1 << 23] {
            assert!(!matches_window_tab(48, extra));
            if extra != 0 {
                assert!(!matches_window_tab(48, (1 << 20) | extra));
            }
        }
    }
    #[test]
    fn gesture_follows_latest_shortcut_command_and_never_another_index() {
        let settings = Settings::default();
        let mut shortcuts = Shortcuts::default();
        shortcuts.entries[1].command = Command::Maximize;
        assert_eq!(
            settings.left.resolve(&shortcuts).unwrap(),
            Some(Command::Maximize)
        );
        shortcuts.entries.remove(1);
        assert!(settings.left.resolve(&shortcuts).is_err());
        assert!(settings.right.resolve(&shortcuts).is_ok());
        let mut bad = settings.clone();
        bad.edge_percent = 90;
        assert!(bad.validate().is_err());
        assert_eq!(
            serde_json::from_str::<Settings>(&serde_json::to_string(&settings).unwrap()).unwrap(),
            settings
        );
    }
}
