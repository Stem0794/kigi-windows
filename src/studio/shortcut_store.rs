use super::bridge::Failure;
use crate::shortcuts::{self, Shortcuts};
use serde_json::{json, Value};
use std::path::Path;
#[derive(Default)]
pub struct ShortcutStore {
    serial: u64,
    seen: Option<(String, Option<Vec<u8>>, Shortcuts)>,
}
impl ShortcutStore {
    pub fn list(&mut self, path: &Path, runtime: &Shortcuts) -> Value {
        match shortcuts::read_bytes(path)
            .and_then(|bytes| shortcuts::decode(bytes.as_deref()).map(|settings| (bytes, settings)))
        {
            Ok((bytes, disk)) => {
                let changed = disk != *runtime;
                if changed {
                    self.seen = None;
                } else if !self
                    .seen
                    .as_ref()
                    .is_some_and(|(_, old, settings)| old == &bytes && settings == runtime)
                {
                    self.serial += 1;
                    self.seen =
                        Some((format!("shortcuts-{}", self.serial), bytes, runtime.clone()));
                }
                json!({"settings":runtime,"combinations":runtime.entries.iter().map(|b|b.combination()).collect::<Vec<_>>(),"revision":self.seen.as_ref().map(|s|&s.0),"canWrite":!changed,"externalChanged":changed,"error":if changed{Some("Le fichier des raccourcis a changé. Rechargez-le pour utiliser sa nouvelle version.")}else{None}})
            }
            Err(message) => {
                self.seen = None;
                json!({"settings":runtime,"combinations":runtime.entries.iter().map(|b|b.combination()).collect::<Vec<_>>(),"revision":null,"canWrite":false,"externalChanged":true,"error":message})
            }
        }
    }
    pub fn prepare(
        &self,
        path: &Path,
        revision: &str,
        runtime: &Shortcuts,
    ) -> Result<Option<Vec<u8>>, Failure> {
        let (_, before, settings) = self
            .seen
            .as_ref()
            .filter(|s| s.0 == revision && s.2 == *runtime)
            .ok_or_else(|| {
                Failure::new(
                    "CONFIG_CONFLICT",
                    "Les raccourcis ont changé. Actualisez ou rechargez-les.",
                )
            })?;
        let bytes = shortcuts::read_bytes(path).map_err(Failure::invalid)?;
        if bytes != *before
            || shortcuts::decode(bytes.as_deref()).map_err(Failure::invalid)? != *settings
        {
            return Err(Failure::new(
                "CONFIG_CONFLICT",
                "Le fichier des raccourcis a changé. Rechargez avant d’enregistrer.",
            ));
        }
        Ok(bytes)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn revisions_protect_runtime_and_exact_disk_bytes() {
        let dir = std::env::temp_dir().join(format!("kigi-shortcut-store-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("shortcuts.json");
        let runtime = Shortcuts::default();
        let mut session = ShortcutStore::default();
        let listing = session.list(&path, &runtime);
        let revision = listing["revision"].as_str().unwrap();
        let bytes = session.prepare(&path, revision, &runtime).unwrap();
        let mut edited = runtime.clone();
        edited.entries[1].enabled = false;
        shortcuts::save_checked(&path, &edited, &runtime, &bytes).unwrap();
        assert!(session.prepare(&path, revision, &runtime).is_err());
        let external = session.list(&path, &runtime);
        assert_eq!(external["canWrite"], false);
        let fresh = session.list(&path, &edited);
        let revision = fresh["revision"].as_str().unwrap();
        let original = std::fs::read(&path).unwrap();
        std::fs::write(&path, [original.clone(), b"\n".to_vec()].concat()).unwrap();
        assert!(session.prepare(&path, revision, &edited).is_err());
        std::fs::write(&path, b"invalid").unwrap();
        assert_eq!(session.list(&path, &edited)["canWrite"], false);
        assert_eq!(std::fs::read(&path).unwrap(), b"invalid");
        std::fs::remove_dir_all(dir).unwrap();
    }
}
