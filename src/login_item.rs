//! One owned per-user RunAtLoad item launching this same application, no helper/daemon.
use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
};
const MARKER: &str = "<!-- Kigi managed login item v1 -->";
pub fn path() -> Result<PathBuf, String> {
    Ok(
        PathBuf::from(std::env::var_os("HOME").ok_or("Dossier utilisateur indisponible.")?)
            .join("Library/LaunchAgents/io.kigi.windows.login.plist"),
    )
}
fn owned(bytes: &[u8]) -> bool {
    std::str::from_utf8(bytes)
        .is_ok_and(|s| s.contains(MARKER) && s.contains("<string>io.kigi.windows.login</string>"))
}
fn escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}
pub fn enabled(path: &Path) -> bool {
    fs::read(path).is_ok_and(|bytes| owned(&bytes))
}
pub fn set(path: &Path, exe: &Path, on: bool) -> Result<(), String> {
    if fs::symlink_metadata(path).is_ok_and(|m| m.file_type().is_symlink()) {
        return Err("Le fichier de connexion est un lien symbolique ; aucune modification.".into());
    }
    let before = match fs::read(path) {
        Ok(bytes) => Some(bytes),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
        Err(_) => return Err("Lecture du lancement à la connexion impossible.".into()),
    };
    if before.as_ref().is_some_and(|bytes| !owned(bytes)) {
        return Err(
            "Un fichier non géré par Kigi utilise déjà cet emplacement. Il est conservé.".into(),
        );
    }
    if !on {
        if before.is_some() {
            fs::remove_file(path)
                .map_err(|_| "Impossible de désactiver le lancement à la connexion.")?;
        }
        return Ok(());
    }
    let exe = exe
        .to_str()
        .ok_or("Chemin d’application non pris en charge.")?;
    if exe.contains("/AppTranslocation/") || !exe.ends_with(".app/Contents/MacOS/kigi-windows") {
        return Err(
            "Déplacez Kigi dans Applications, puis relancez-le avant d’activer cette option."
                .into(),
        );
    }
    let text=format!("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<!DOCTYPE plist PUBLIC \"-//Apple//DTD PLIST 1.0//EN\" \"http://www.apple.com/DTDs/PropertyList-1.0.dtd\">\n<plist version=\"1.0\">{MARKER}<dict><key>Label</key><string>io.kigi.windows.login</string><key>ProgramArguments</key><array><string>{}</string><string>--login</string></array><key>RunAtLoad</key><true/><key>LimitLoadToSessionType</key><string>Aqua</string></dict></plist>\n",escape(exe));
    fs::create_dir_all(path.parent().ok_or("Dossier de connexion absent.")?)
        .map_err(|_| "Impossible de créer le dossier de connexion.")?;
    let temp = path.with_extension(format!("{}.tmp", std::process::id()));
    let result = (|| {
        let mut file = fs::OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&temp)
            .map_err(|_| "Création du fichier temporaire impossible.")?;
        file.write_all(text.as_bytes())
            .and_then(|_| file.sync_all())
            .map_err(|_| "Écriture du lancement impossible.")?;
        if fs::read(path).ok() != before {
            return Err("Le lancement à la connexion a changé pendant l’enregistrement.");
        }
        fs::rename(&temp, path).map_err(|_| "Activation du lancement à la connexion impossible.")
    })();
    if result.is_err() {
        let _ = fs::remove_file(temp);
    }
    result.map_err(str::to_string)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn owned_login_item_escapes_paths_and_preserves_foreign_files() {
        let root = std::env::temp_dir().join(format!("kigi-login-{}", std::process::id()));
        let _ = fs::create_dir_all(&root);
        let path = root.join("login.plist");
        let exe = Path::new("/Applications/Kigi & Tests.app/Contents/MacOS/kigi-windows");
        set(&path, exe, true).unwrap();
        assert!(enabled(&path));
        assert!(fs::read_to_string(&path)
            .unwrap()
            .contains("Kigi &amp; Tests.app"));
        set(&path, exe, false).unwrap();
        assert!(!path.exists());
        fs::write(&path, "foreign").unwrap();
        assert!(set(&path, exe, true).is_err());
        assert!(set(&path, exe, false).is_err());
        assert_eq!(fs::read_to_string(&path).unwrap(), "foreign");
        let _ = fs::remove_dir_all(root);
    }
}
