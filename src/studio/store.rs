//! Small Studio session: opaque revision + exact primary bytes, existing catalog writes.
use super::bridge::{EditParams, Failure};
use crate::{
    config::{self, Catalog, SavedLayout},
    geometry,
};
use serde::Serialize;
use std::{
    path::Path,
    time::{SystemTime, UNIX_EPOCH},
};

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Listing {
    pub profiles: Vec<config::Profile>,
    pub using_backup: bool,
    pub can_write: bool,
    pub revision: Option<String>,
}
struct Snapshot {
    revision: String,
    bytes: Option<Vec<u8>>,
}
#[derive(Default)]
pub struct Store {
    serial: u64,
    snapshot: Option<Snapshot>,
}
pub enum Mutation<'a> {
    Save(&'a EditParams),
    Duplicate(u32),
    Delete(u32),
}
impl Store {
    pub fn list(&mut self, path: &Path) -> Result<Listing, Failure> {
        match config::read_primary(path) {
            Ok((catalog, bytes)) => {
                if !self.snapshot.as_ref().is_some_and(|s| s.bytes == bytes) {
                    self.serial += 1;
                    let now = SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_nanos();
                    self.snapshot = Some(Snapshot {
                        revision: format!("rev-{now:x}-{:x}", self.serial),
                        bytes,
                    });
                }
                Ok(Listing {
                    profiles: catalog.profiles,
                    using_backup: false,
                    can_write: true,
                    revision: self.snapshot.as_ref().map(|s| s.revision.clone()),
                })
            }
            Err(message) => {
                self.snapshot = None;
                if let Ok(Some((catalog, true))) = config::load(path) {
                    Ok(Listing {
                        profiles: catalog.profiles,
                        using_backup: true,
                        can_write: false,
                        revision: None,
                    })
                } else {
                    Err(Failure::new("CONFIG_INVALID", message))
                }
            }
        }
    }
    pub fn mutate(
        &mut self,
        path: &Path,
        revision: &str,
        mutation: Mutation<'_>,
    ) -> Result<(Listing, Catalog, Option<u32>), Failure> {
        let snapshot = self
            .snapshot
            .as_ref()
            .filter(|s| s.revision == revision)
            .ok_or_else(|| Failure::new("CONFIG_CONFLICT", config::CONFLICT))?;
        let (mut catalog, bytes) =
            config::read_primary(path).map_err(|e| Failure::new("CONFIG_INVALID", e))?;
        if bytes != snapshot.bytes {
            return Err(Failure::new("CONFIG_CONFLICT", config::CONFLICT));
        }
        let selected = match mutation {
            Mutation::Save(input) => {
                let mut layout = SavedLayout {
                    schema_version: 1,
                    windows: input.windows.clone(),
                };
                layout.validate().map_err(Failure::invalid)?;
                if let Some(percent) = input.left_percent {
                    let geometry = geometry::normalized_split(percent).map_err(Failure::invalid)?;
                    for (window, g) in layout.windows.iter_mut().zip(geometry) {
                        if ![
                            window.geometry.x - g.x,
                            window.geometry.y - g.y,
                            window.geometry.width - g.width,
                            window.geometry.height - g.height,
                        ]
                        .iter()
                        .all(|n| n.abs() < 1e-9)
                        {
                            return Err(Failure::invalid(
                                "La géométrie ne correspond pas aux proportions.",
                            ));
                        }
                        window.geometry = g;
                    }
                }
                Some(
                    catalog
                        .upsert(input.profile_id, &input.name, &layout)
                        .map_err(Failure::invalid)?,
                )
            }
            Mutation::Duplicate(id) => {
                let original = catalog.profile(id).map_err(Failure::invalid)?;
                let base: String = original.name.chars().take(48).collect();
                let layout = original.layout();
                let name = (1..=33)
                    .map(|n| format!("{base} copie {n}"))
                    .find(|name| {
                        !catalog
                            .profiles
                            .iter()
                            .any(|p| p.name.to_lowercase() == name.to_lowercase())
                    })
                    .ok_or_else(|| Failure::invalid("Nom de copie indisponible."))?;
                Some(
                    catalog
                        .upsert(None, &name, &layout)
                        .map_err(Failure::invalid)?,
                )
            }
            Mutation::Delete(id) => {
                catalog.remove(id).map_err(Failure::invalid)?;
                None
            }
        };
        config::save_checked(path, &catalog, &bytes).map_err(|e| {
            Failure::new(
                if e == config::CONFLICT {
                    "CONFIG_CONFLICT"
                } else {
                    "CONFIG_WRITE_FAILED"
                },
                e,
            )
        })?;
        let listing = self.list(path)?;
        Ok((listing, catalog, selected))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{Profile, SavedWindow};
    fn input(percent: u8) -> EditParams {
        let [a, b] = geometry::normalized_split(percent).unwrap();
        EditParams {
            expected_revision: String::new(),
            profile_id: None,
            name: "Réunion".into(),
            left_percent: Some(percent),
            windows: [
                SavedWindow {
                    bundle_id: "com.google.Chrome".into(),
                    geometry: a,
                },
                SavedWindow {
                    bundle_id: "com.microsoft.teams2".into(),
                    geometry: b,
                },
            ],
        }
    }
    #[test]
    fn editor_roundtrips_ratios_and_preserves_free_geometry_and_conflicts() {
        let dir = std::env::temp_dir().join(format!(
            "kigi-editor-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("config.json");
        let mut store = Store::default();
        let initial = store.list(&path).unwrap();
        assert!(!path.exists());
        let mut edit = input(35);
        let (listing, _, id) = store
            .mutate(
                &path,
                initial.revision.as_deref().unwrap(),
                Mutation::Save(&edit),
            )
            .unwrap();
        assert_eq!(listing.profiles[0].windows[0].geometry.width, 0.35);
        let mut restarted = Store::default();
        let loaded = restarted.list(&path).unwrap();
        assert_eq!(loaded.profiles, listing.profiles);
        edit.profile_id = id;
        edit.name = "Renommé".into();
        edit.left_percent = None;
        edit.windows[0].geometry = geometry::NormalizedRect {
            x: 0.1,
            y: 0.2,
            width: 0.6,
            height: 0.7,
        };
        let (listing, _, _) = restarted
            .mutate(
                &path,
                loaded.revision.as_deref().unwrap(),
                Mutation::Save(&edit),
            )
            .unwrap();
        assert_eq!(listing.profiles[0].windows, edit.windows);
        let before = std::fs::read(&path).unwrap();
        std::fs::write(&path, [before.clone(), b"\n".to_vec()].concat()).unwrap();
        assert_eq!(
            restarted
                .mutate(
                    &path,
                    listing.revision.as_deref().unwrap(),
                    Mutation::Delete(id.unwrap())
                )
                .err()
                .unwrap()
                .code,
            "CONFIG_CONFLICT"
        );
        assert_eq!(
            std::fs::read(&path).unwrap(),
            [before, b"\n".to_vec()].concat()
        );
        let fresh = restarted.list(&path).unwrap();
        let (copied, _, copy) = restarted
            .mutate(
                &path,
                fresh.revision.as_deref().unwrap(),
                Mutation::Duplicate(id.unwrap()),
            )
            .unwrap();
        assert_ne!(copy, id);
        assert_eq!(copied.profiles[0].windows, copied.profiles[1].windows);
        let (deleted, _, _) = restarted
            .mutate(
                &path,
                copied.revision.as_deref().unwrap(),
                Mutation::Delete(copy.unwrap()),
            )
            .unwrap();
        assert_eq!(deleted.profiles.len(), 1);
        let valid = std::fs::read(&path).unwrap();
        std::fs::write(path.with_extension("json.bak"), &valid).unwrap();
        std::fs::write(&path, b"invalid").unwrap();
        let backup = restarted.list(&path).unwrap();
        assert!(backup.using_backup && !backup.can_write && backup.revision.is_none());
        assert!(restarted
            .mutate(&path, "stale", Mutation::Save(&edit))
            .is_err());
        assert_eq!(std::fs::read(&path).unwrap(), b"invalid");
        std::fs::remove_dir_all(dir).unwrap();
    }
    #[test]
    fn ratio_extremes_keep_the_existing_geometry_contract() {
        for percent in [1, 35, 49, 50, 70, 99] {
            let [a, b] = geometry::normalized_split(percent).unwrap();
            assert!(a.valid() && b.valid());
            assert_eq!(a.width + b.width, 1.);
            let screen = geometry::Rect {
                x: -2000.,
                y: 25.,
                width: 1000.,
                height: 900.,
            };
            let left = a.on_display(screen);
            let right = b.on_display(screen);
            assert!((right.x - (left.x + left.width)).abs() < 1e-8);
            assert!((left.width + right.width - 1000.).abs() < 1e-8);
        }
        assert!(geometry::normalized_split(0).is_err());
        assert!(geometry::normalized_split(100).is_err());
        let p = Profile {
            id: 1,
            name: "Ancien libre".into(),
            windows: input(35).windows,
        };
        assert_eq!(p.layout().schema_version, 1);
    }
}
