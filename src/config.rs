//! One saved two-window layout, versioned and written atomically. No titles/PIDs.
use crate::geometry::NormalizedRect;
use serde::{Deserialize, Serialize};
use std::{
    fs::{self, File},
    io::{Read, Write},
    path::{Path, PathBuf},
};
const MAX_BYTES: u64 = 64 * 1024;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SavedWindow {
    pub bundle_id: String,
    pub geometry: NormalizedRect,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
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
fn decode(bytes: &[u8]) -> Result<SavedLayout, String> {
    let layout: SavedLayout = serde_json::from_slice(bytes)
        .map_err(|_| "Configuration JSON invalide ; fichier conservé.".to_owned())?;
    layout.validate()?;
    Ok(layout)
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
pub fn load(path: &Path) -> Result<Option<(SavedLayout, bool)>, String> {
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
pub fn save(path: &Path, layout: &SavedLayout) -> Result<(), String> {
    layout.validate()?;
    let bytes = serde_json::to_vec_pretty(layout)
        .map_err(|_| "Configuration non sérialisable.".to_owned())?;
    if let Some(old) = read(path)? {
        decode(&old)?; // refuse to overwrite an invalid/newer user file
        atomic_write(&path.with_extension("json.bak"), &old)?;
    }
    fs::create_dir_all(path.parent().ok_or("Dossier de configuration absent.")?)
        .map_err(|_| "Impossible de créer le dossier de configuration.".to_owned())?;
    atomic_write(path, &bytes)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn roundtrip_backup_recovery_and_failed_save_preserve_files() {
        let root = std::env::temp_dir().join(format!(
            "kigi-config-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let path = root.join("config.json");
        let window = SavedWindow {
            bundle_id: "com.google.Chrome".into(),
            geometry: NormalizedRect {
                x: 0.0,
                y: 0.0,
                width: 0.5,
                height: 1.0,
            },
        };
        let layout = SavedLayout {
            schema_version: 1,
            windows: [window.clone(), window],
        };
        save(&path, &layout).unwrap();
        let first = fs::read(&path).unwrap();
        assert!(!load(&path).unwrap().unwrap().1);
        save(&path, &layout).unwrap();
        assert_eq!(fs::read(path.with_extension("json.bak")).unwrap(), first);
        fs::write(&path, b"invalid").unwrap();
        assert!(load(&path).unwrap().unwrap().1);
        assert!(save(&path, &layout).is_err());
        assert_eq!(fs::read(&path).unwrap(), b"invalid");
        let mut extra = serde_json::to_value(&layout).unwrap();
        extra["script"] = serde_json::Value::String("bad".into());
        assert!(serde_json::from_value::<SavedLayout>(extra).is_err());
        let mut bad = layout;
        bad.windows[0].bundle_id = "/bin/sh".into();
        assert!(bad.validate().is_err());
        fs::remove_dir_all(root).unwrap();
    }
}
