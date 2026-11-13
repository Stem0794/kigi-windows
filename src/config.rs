//! Named two-window layouts. Schema 1 is migrated in memory; schema 2 writes stay atomic.
use crate::geometry::NormalizedRect;
use serde::{Deserialize, Serialize};
use std::{
    fs::{self, File},
    io::{Read, Write},
    path::{Path, PathBuf},
};
const MAX_BYTES: u64 = 64 * 1024;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SavedWindow {
    pub bundle_id: String,
    pub geometry: NormalizedRect,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SavedLayout {
    pub schema_version: u32,
    pub windows: [SavedWindow; 2],
}
impl SavedLayout {
    pub fn validate(&self) -> Result<(), String> {
        if self.schema_version != 1 {
            return Err("Version de configuration non prise en charge.".into());
        }
        for window in &self.windows {
            let b = &window.bundle_id;
            if b.is_empty()
                || b.len() > 200
                || !b.contains('.')
                || !b
                    .bytes()
                    .all(|c| c.is_ascii_alphanumeric() || b".-_".contains(&c))
                || !window.geometry.valid()
            {
                return Err(
                    "Configuration invalide : application ou géométrie hors limites.".into(),
                );
            }
        }
        Ok(())
    }
}
pub fn path() -> Result<PathBuf, String> {
    let home = std::env::var_os("HOME").ok_or("Dossier utilisateur indisponible.")?;
    Ok(PathBuf::from(home).join("Library/Application Support/io.kigi.windows/config.json"))
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Profile {
    pub id: u32,
    pub name: String,
    pub windows: [SavedWindow; 2],
}
impl Profile {
    pub fn layout(&self) -> SavedLayout {
        SavedLayout {
            schema_version: 1,
            windows: self.windows.clone(),
        }
    }
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Catalog {
    pub schema_version: u32,
    pub next_id: u32,
    pub profiles: Vec<Profile>,
}
impl Default for Catalog {
    fn default() -> Self {
        Self {
            schema_version: 2,
            next_id: 1,
            profiles: Vec::new(),
        }
    }
}
impl Catalog {
    pub fn validate(&self) -> Result<(), String> {
        if self.schema_version != 2 || self.next_id == 0 || self.profiles.len() > 32 {
            return Err("Version ou taille de configuration non prise en charge.".into());
        }
        let mut ids = std::collections::HashSet::new();
        let mut names = std::collections::HashSet::new();
        for profile in &self.profiles {
            Self::valid_name(&profile.name)?;
            if profile.id == 0
                || profile.id >= self.next_id
                || !ids.insert(profile.id)
                || !names.insert(profile.name.to_lowercase())
            {
                return Err("Espaces invalides : noms ou identifiants dupliqués.".into());
            }
            profile.layout().validate()?;
        }
        Ok(())
    }
    pub fn valid_name(name: &str) -> Result<(), String> {
        if name.trim() != name
            || name.is_empty()
            || name.chars().count() > 60
            || name.chars().any(char::is_control)
        {
            Err("Choisissez un nom de 1 à 60 caractères, sans retour à la ligne.".into())
        } else {
            Ok(())
        }
    }
    pub fn profile(&self, id: u32) -> Result<&Profile, String> {
        self.profiles
            .iter()
            .find(|p| p.id == id)
            .ok_or_else(|| "Cet espace n'existe plus. Actualisez la liste.".into())
    }
    pub fn upsert(
        &mut self,
        id: Option<u32>,
        name: &str,
        layout: &SavedLayout,
    ) -> Result<u32, String> {
        Self::valid_name(name)?;
        layout.validate()?;
        if self
            .profiles
            .iter()
            .any(|p| Some(p.id) != id && p.name.to_lowercase() == name.to_lowercase())
        {
            return Err("Ce nom existe déjà. Choisissez un autre nom.".into());
        }
        if let Some(id) = id {
            self.profile(id)?;
            let p = self.profiles.iter_mut().find(|p| p.id == id).unwrap();
            p.name = name.into();
            p.windows = layout.windows.clone();
            Ok(id)
        } else {
            if self.profiles.len() >= 32 {
                return Err("La limite de 32 espaces est atteinte.".into());
            }
            let next = self
                .next_id
                .checked_add(1)
                .ok_or("Identifiants d'espaces épuisés.")?;
            let id = self.next_id;
            self.profiles.push(Profile {
                id,
                name: name.into(),
                windows: layout.windows.clone(),
            });
            self.next_id = next;
            Ok(id)
        }
    }
    pub fn remove(&mut self, id: u32) -> Result<(), String> {
        self.profile(id)?;
        self.profiles.retain(|p| p.id != id);
        Ok(())
    }
}
fn decode(bytes: &[u8]) -> Result<Catalog, String> {
    let value: serde_json::Value = serde_json::from_slice(bytes)
        .map_err(|_| "Configuration JSON invalide ; fichier conservé.".to_owned())?;
    let catalog = match value.get("schema_version").and_then(|v| v.as_u64()) {
        Some(1) => {
            let old: SavedLayout = serde_json::from_value(value)
                .map_err(|_| "Ancienne configuration invalide.".to_owned())?;
            old.validate()?;
            Catalog {
                schema_version: 2,
                next_id: 2,
                profiles: vec![Profile {
                    id: 1,
                    name: "Disposition personnelle".into(),
                    windows: old.windows,
                }],
            }
        }
        Some(2) => serde_json::from_value(value)
            .map_err(|_| "Configuration d'espaces invalide.".to_owned())?,
        _ => return Err("Version de configuration non prise en charge ; fichier conservé.".into()),
    };
    catalog.validate()?;
    Ok(catalog)
}
fn read(path: &Path) -> Result<Option<Vec<u8>>, String> {
    let mut file = match File::open(path) {
        Ok(f) => f,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(_) => return Err("Impossible de lire la configuration locale.".into()),
    };
    let mut bytes = Vec::new();
    (&mut file)
        .take(MAX_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| "Lecture de configuration interrompue.".to_owned())?;
    if bytes.len() as u64 > MAX_BYTES {
        return Err("Configuration trop volumineuse (maximum 64 Ko).".into());
    }
    Ok(Some(bytes))
}
/// Return whether a validated backup was used. Never modify an invalid primary.
pub fn load(path: &Path) -> Result<Option<(Catalog, bool)>, String> {
    let primary = read(path).and_then(|b| b.map(|b| decode(&b)).transpose());
    if let Ok(Some(layout)) = primary {
        return Ok(Some((layout, false)));
    }
    let backup =
        read(&path.with_extension("json.bak")).and_then(|b| b.map(|b| decode(&b)).transpose());
    match backup {
        Ok(Some(layout)) => Ok(Some((layout, true))),
        _ => primary.map(|_| None),
    }
}
fn atomic_write(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let temp = path.with_extension("tmp");
    let result = (|| {
        let mut file = File::create(&temp)?;
        file.write_all(bytes)?;
        file.sync_all()?;
        fs::rename(&temp, path)?;
        if let Some(parent) = path.parent() {
            if let Ok(dir) = File::open(parent) {
                let _ = dir.sync_all();
            }
        }
        Ok::<(), std::io::Error>(())
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temp);
    }
    result.map_err(|_| {
        "Écriture de configuration impossible ; ancienne disposition conservée.".into()
    })
}
/// Exact bytes accompany the validated primary; never silently edit a backup.
pub fn read_primary(path: &Path) -> Result<(Catalog, Option<Vec<u8>>), String> {
    let bytes = read(path)?;
    let catalog = bytes
        .as_deref()
        .map(decode)
        .transpose()?
        .unwrap_or_default();
    Ok((catalog, bytes))
}
pub const CONFLICT: &str = "Le fichier des espaces a changé. Rechargez avant d’enregistrer.";
pub fn save_checked(
    path: &Path,
    layout: &Catalog,
    expected: &Option<Vec<u8>>,
) -> Result<(), String> {
    save_inner(path, layout, Some(expected))
}
pub fn save(path: &Path, layout: &Catalog) -> Result<(), String> {
    save_inner(path, layout, None)
}
fn save_inner(
    path: &Path,
    layout: &Catalog,
    expected: Option<&Option<Vec<u8>>>,
) -> Result<(), String> {
    layout.validate()?;
    let bytes = serde_json::to_vec_pretty(layout)
        .map_err(|_| "Configuration non sérialisable.".to_owned())?;
    let old = read(path)?;
    if expected.is_some_and(|expected| expected != &old) {
        return Err(CONFLICT.into());
    }
    if let Some(old) = old.as_ref() {
        decode(old)?; // refuse to overwrite an invalid/newer user file
        atomic_write(&path.with_extension("json.bak"), old)?;
    }
    fs::create_dir_all(path.parent().ok_or("Dossier de configuration absent.")?)
        .map_err(|_| "Impossible de créer le dossier de configuration.".to_owned())?;
    // Recheck immediately before the atomic replacement, after backup work.
    if expected.is_some() && read(path)? != old {
        return Err(CONFLICT.into());
    }
    atomic_write(path, &bytes)
}
#[cfg(test)]
mod tests {
    use super::*;
    fn layout() -> SavedLayout {
        let w = SavedWindow {
            bundle_id: "com.google.Chrome".into(),
            geometry: NormalizedRect {
                x: 0.0,
                y: 0.0,
                width: 0.5,
                height: 1.0,
            },
        };
        SavedLayout {
            schema_version: 1,
            windows: [w.clone(), w],
        }
    }
    #[test]
    fn migration_backup_roundtrip_and_invalid_primary_are_preserved() {
        let root = std::env::temp_dir().join(format!(
            "kigi-catalog-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&root).unwrap();
        let path = root.join("config.json");
        let old = serde_json::to_vec(&layout()).unwrap();
        fs::write(&path, &old).unwrap();
        let (mut catalog, backup) = load(&path).unwrap().unwrap();
        assert!(!backup);
        assert_eq!(catalog.profile(1).unwrap().name, "Disposition personnelle");
        assert_eq!(fs::read(&path).unwrap(), old); // opening never migrates on disk
        catalog.upsert(None, "Réunion", &layout()).unwrap();
        save(&path, &catalog).unwrap();
        assert_eq!(fs::read(path.with_extension("json.bak")).unwrap(), old);
        assert_eq!(load(&path).unwrap().unwrap().0.profiles.len(), 2);
        fs::write(&path, b"invalid").unwrap();
        assert!(load(&path).unwrap().unwrap().1);
        assert!(save(&path, &catalog).is_err());
        assert_eq!(fs::read(&path).unwrap(), b"invalid");
        let mut extra = serde_json::to_value(&catalog).unwrap();
        extra["script"] = "bad".into();
        assert!(decode(&serde_json::to_vec(&extra).unwrap()).is_err());
        fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn stable_ids_rename_and_delete_do_not_change_other_profiles_or_reuse_ids() {
        let mut c = Catalog::default();
        let l = layout();
        let one = c.upsert(None, "Travail", &l).unwrap();
        let two = c.upsert(None, "Réunion", &l).unwrap();
        assert!(c.upsert(Some(two), "TRAVAIL", &l).is_err());
        assert_eq!(c.upsert(Some(one), "Bureau", &l).unwrap(), one);
        c.remove(two).unwrap();
        let three = c.upsert(None, "Perso", &l).unwrap();
        assert!(three > two);
        assert_eq!(c.profile(one).unwrap().name, "Bureau");
        assert!(c.profile(two).is_err());
        c.validate().unwrap();
        assert!(c.upsert(None, "  ", &l).is_err());
    }
}
