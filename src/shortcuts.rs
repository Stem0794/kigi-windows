//! User shortcuts, independent from the migrated workspace catalogue.
use crate::actions::Action;
use serde::{Deserialize, Serialize};
use std::{
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    path::Path,
};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", deny_unknown_fields)]
pub enum Command {
    Application {
        bundle: String,
        name: String,
        maximize: bool,
    },
    ChatGpt,
    Maximize,
    PairLeft,
    PairRight,
    Test,
}
impl Command {
    pub fn label(&self) -> String {
        match self {
            Self::Application { name, maximize, .. } => {
                format!("{}{}", name, if *maximize { " · maximiser" } else { "" })
            }
            Self::ChatGpt => "Retrouver ChatGPT dans Chrome".into(),
            Self::Maximize => "Maximiser la fenêtre".into(),
            Self::PairLeft => "Partager : gauche puis droite".into(),
            Self::PairRight => "Partager : droite puis gauche".into(),
            Self::Test => "Tester / restaurer la fenêtre".into(),
        }
    }
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Binding {
    pub enabled: bool,
    pub key: u32,
    pub modifiers: u32,
    pub key_label: String,
    pub command: Command,
}
impl Binding {
    pub fn combination(&self) -> String {
        let mut text = String::new();
        for (flag, glyph) in [(1 << 12, "⌃"), (1 << 11, "⌥"), (1 << 9, "⇧"), (1 << 8, "⌘")]
        {
            if self.modifiers & flag != 0 {
                text.push_str(glyph);
            }
        }
        text.push_str(&self.key_label);
        text
    }
    pub fn caption(&self) -> String {
        format!(
            "{}{} — {}",
            if self.enabled { "" } else { "Désactivé · " },
            self.combination(),
            self.command.label()
        )
    }
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Shortcuts {
    pub schema_version: u32,
    pub entries: Vec<Binding>,
}
impl Default for Shortcuts {
    fn default() -> Self {
        let commands = [
            Command::Test,
            Command::Application {
                bundle: "com.microsoft.teams2".into(),
                name: "Teams".into(),
                maximize: false,
            },
            Command::Application {
                bundle: "com.google.Chrome".into(),
                name: "Google Chrome".into(),
                maximize: true,
            },
            Command::ChatGpt,
            Command::Maximize,
            Command::PairLeft,
            Command::PairRight,
        ];
        Self {
            schema_version: 1,
            entries: Action::ALL
                .into_iter()
                .zip(commands)
                .map(|(a, command)| {
                    let (key, modifiers) = a.binding();
                    Binding {
                        enabled: true,
                        key,
                        modifiers,
                        key_label: match a {
                            Action::Test | Action::Teams => "T",
                            Action::Browser => "B",
                            Action::ChatGpt => "C",
                            Action::Maximize => "↑",
                            Action::PairLeft => "←",
                            Action::PairRight => "→",
                        }
                        .into(),
                        command,
                    }
                })
                .collect(),
        }
    }
}
pub fn valid_combination(key: u32, modifiers: u32) -> bool {
    key <= 127
        && !(54..=63).contains(&key)
        && modifiers & !((1 << 8) | (1 << 9) | (1 << 11) | (1 << 12)) == 0
        && modifiers & ((1 << 8) | (1 << 11) | (1 << 12)) != 0
}
impl Shortcuts {
    pub fn validate(&self) -> Result<(), String> {
        if self.schema_version != 1 || self.entries.len() > 32 {
            return Err("Configuration des raccourcis non prise en charge (32 maximum).".into());
        }
        let mut seen = std::collections::HashSet::new();
        for b in &self.entries {
            if !valid_combination(b.key, b.modifiers)
                || b.key_label.is_empty()
                || b.key_label.chars().count() > 16
                || b.key_label.chars().any(char::is_control)
            {
                return Err("Choisissez une touche avec ⌘, ⌥ ou ⌃ (⇧ facultatif).".into());
            }
            if b.enabled && !seen.insert((b.key, b.modifiers)) {
                return Err(format!(
                    "{} est déjà utilisé. Modifiez ou désactivez l’autre raccourci.",
                    b.combination()
                ));
            }
            if let Command::Application { bundle, name, .. } = &b.command {
                if bundle.len() > 200
                    || !bundle.contains('.')
                    || !bundle
                        .bytes()
                        .all(|c| c.is_ascii_alphanumeric() || b".-_".contains(&c))
                    || name.trim().is_empty()
                    || name.chars().count() > 100
                    || name.chars().any(char::is_control)
                {
                    return Err("Choisissez une application macOS valide.".into());
                }
            }
        }
        Ok(())
    }
}
pub fn load(path: &Path) -> Result<Shortcuts, String> {
    let mut f = match File::open(path) {
        Ok(f) => f,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Shortcuts::default()),
        Err(_) => return Err("Impossible de lire les raccourcis.".into()),
    };
    let mut bytes = Vec::new();
    Read::by_ref(&mut f)
        .take(65537)
        .read_to_end(&mut bytes)
        .map_err(|_| "Lecture des raccourcis impossible.")?;
    if bytes.len() > 65536 {
        return Err("Fichier de raccourcis trop volumineux ; conservé.".into());
    }
    let settings: Shortcuts =
        serde_json::from_slice(&bytes).map_err(|_| "Raccourcis illisibles ; fichier conservé.")?;
    settings.validate()?;
    Ok(settings)
}
pub fn save(path: &Path, settings: &Shortcuts, expected: &Shortcuts) -> Result<(), String> {
    settings.validate()?;
    if &load(path)? != expected {
        return Err(
            "Les raccourcis ont changé sur disque. Rouvrez Kigi avant d’enregistrer.".into(),
        );
    }
    let parent = path.parent().ok_or("Dossier de raccourcis absent.")?;
    fs::create_dir_all(parent).map_err(|_| "Création du dossier impossible.")?;
    let temp = path.with_extension(format!("{}.tmp", std::process::id()));
    let result = (|| {
        let mut f = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temp)
            .map_err(|_| "Écriture temporaire impossible.")?;
        let bytes = serde_json::to_vec_pretty(settings).map_err(|_| "Sérialisation impossible.")?;
        f.write_all(&bytes)
            .and_then(|_| f.sync_all())
            .map_err(|_| "Écriture des raccourcis impossible.")?;
        fs::rename(&temp, path).map_err(|_| "Enregistrement des raccourcis impossible.")
    })();
    if result.is_err() {
        let _ = fs::remove_file(temp);
    }
    result.map_err(str::to_owned)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn defaults_reassignment_duplicates_and_modifiers() {
        let mut s = Shortcuts::default();
        s.validate().unwrap();
        s.entries[1].command = Command::Application {
            bundle: "com.apple.Safari".into(),
            name: "Safari".into(),
            maximize: false,
        };
        s.validate().unwrap();
        assert!(s.entries[1].caption().contains("Safari"));
        s.entries[2].key = s.entries[1].key;
        s.entries[2].modifiers = s.entries[1].modifiers;
        assert!(s.validate().is_err());
        s.entries[2].enabled = false;
        s.validate().unwrap();
        assert!(!valid_combination(17, 0));
        assert!(!valid_combination(17, 1 << 9));
        assert!(!valid_combination(55, 1 << 11));
        assert!(valid_combination(17, (1 << 11) | (1 << 9)));
    }
    #[test]
    fn persistence_restart_invalid_and_external_change_are_preserved() {
        let dir = std::env::temp_dir().join(format!("kigi-shortcuts-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let p = dir.join("shortcuts.json");
        let defaults = load(&p).unwrap();
        let mut s = defaults.clone();
        s.entries[1].key = 15;
        s.entries[1].key_label = "R".into();
        save(&p, &s, &defaults).unwrap();
        assert_eq!(load(&p).unwrap(), s);
        assert!(save(&p, &defaults, &defaults).is_err());
        assert_eq!(load(&p).unwrap(), s);
        fs::write(&p, b"invalid").unwrap();
        assert!(save(&p, &defaults, &s).is_err());
        assert_eq!(fs::read(&p).unwrap(), b"invalid");
        fs::remove_dir_all(dir).unwrap();
    }
}
