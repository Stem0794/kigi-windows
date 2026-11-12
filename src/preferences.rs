//! Small native P4 form. Editing/capture actions stay in the application controller.
use crate::{
    config::{Catalog, Profile},
    geometry::NormalizedRect,
};
use objc2::{
    rc::Retained,
    runtime::{AnyObject, Sel},
    sel, MainThreadOnly,
};
use objc2_app_kit::*;
use objc2_foundation::{MainThreadMarker, NSPoint, NSRect, NSSize, NSString};

pub struct Preferences {
    pub window: Retained<NSWindow>,
    pub profiles: Retained<NSPopUpButton>,
    pub name: Retained<NSTextField>,
    pub positions: [Retained<NSPopUpButton>; 2],
    pub status: Retained<NSTextField>,
    descriptions: [Retained<NSTextField>; 2],
    capture: Retained<NSButton>,
    selected_buttons: Vec<Retained<NSButton>>,
}
fn rect(x: f64, y: f64, w: f64, h: f64) -> NSRect {
    NSRect::new(NSPoint::new(x, y), NSSize::new(w, h))
}
fn label(mtm: MainThreadMarker, root: &NSView, text: &str, frame: NSRect) -> Retained<NSTextField> {
    let field = NSTextField::wrappingLabelWithString(&NSString::from_str(text), mtm);
    field.setFrame(frame);
    root.addSubview(&field);
    field
}
fn button(
    mtm: MainThreadMarker,
    root: &NSView,
    target: &AnyObject,
    text: &str,
    action: Sel,
    frame: NSRect,
) -> Retained<NSButton> {
    let b = unsafe {
        NSButton::buttonWithTitle_target_action(
            &NSString::from_str(text),
            Some(target),
            Some(action),
            mtm,
        )
    };
    b.setFrame(frame);
    root.addSubview(&b);
    b
}
fn popup(mtm: MainThreadMarker, root: &NSView, frame: NSRect) -> Retained<NSPopUpButton> {
    let p = NSPopUpButton::initWithFrame_pullsDown(NSPopUpButton::alloc(mtm), frame, false);
    root.addSubview(&p);
    p
}
impl Preferences {
    pub fn new(mtm: MainThreadMarker, target: &AnyObject) -> Self {
        let window = unsafe {
            NSWindow::initWithContentRect_styleMask_backing_defer(
                NSWindow::alloc(mtm),
                rect(0.0, 0.0, 620.0, 510.0),
                NSWindowStyleMask::Titled | NSWindowStyleMask::Closable,
                NSBackingStoreType::Buffered,
                false,
            )
        };
        unsafe {
            window.setReleasedWhenClosed(false);
        }
        window.setTitle(&NSString::from_str("Kigi — Espaces de travail"));
        let root = NSView::initWithFrame(NSView::alloc(mtm), rect(0.0, 0.0, 620.0, 510.0));
        label(
            mtm,
            &root,
            "Enregistrez et retrouvez vos dispositions de deux fenêtres.",
            rect(22.0, 467.0, 576.0, 24.0),
        );
        let profiles = popup(mtm, &root, rect(20.0, 423.0, 580.0, 30.0));
        unsafe {
            profiles.setTarget(Some(target));
            profiles.setAction(Some(sel!(profileSelected:)));
        }
        label(
            mtm,
            &root,
            "Nom de l'espace",
            rect(22.0, 393.0, 250.0, 20.0),
        );
        let name = NSTextField::textFieldWithString(&NSString::from_str("Nouvel espace"), mtm);
        name.setFrame(rect(22.0, 360.0, 355.0, 27.0));
        root.addSubview(&name);
        let rename = button(
            mtm,
            &root,
            target,
            "Renommer",
            sel!(renameProfile:),
            rect(387.0, 356.0, 105.0, 34.0),
        );
        let duplicate = button(
            mtm,
            &root,
            target,
            "Dupliquer",
            sel!(duplicateProfile:),
            rect(497.0, 356.0, 105.0, 34.0),
        );
        let capture = button(
            mtm,
            &root,
            target,
            "Enregistrer deux fenêtres…",
            sel!(captureProfile:),
            rect(18.0, 304.0, 350.0, 36.0),
        );
        button(
            mtm,
            &root,
            target,
            "Actualiser",
            sel!(refreshProfiles:),
            rect(395.0, 304.0, 205.0, 36.0),
        );
        label(
            mtm,
            &root,
            "Positions sur l'écran courant",
            rect(22.0, 266.0, 555.0, 23.0),
        );
        let descriptions = [
            label(mtm, &root, "Fenêtre 1", rect(22.0, 213.0, 275.0, 42.0)),
            label(mtm, &root, "Fenêtre 2", rect(22.0, 158.0, 275.0, 42.0)),
        ];
        let positions = [
            popup(mtm, &root, rect(310.0, 223.0, 290.0, 28.0)),
            popup(mtm, &root, rect(310.0, 168.0, 290.0, 28.0)),
        ];
        for p in &positions {
            for title in [
                "Position enregistrée",
                "Moitié gauche",
                "Moitié droite",
                "100 % de l'écran",
            ] {
                p.addItemWithTitle(&NSString::from_str(title));
            }
        }
        let apply = button(
            mtm,
            &root,
            target,
            "Appliquer",
            sel!(applySelectedProfile:),
            rect(18.0, 108.0, 155.0, 34.0),
        );
        let geometry = button(
            mtm,
            &root,
            target,
            "Enregistrer les positions",
            sel!(saveProfilePositions:),
            rect(185.0, 108.0, 245.0, 34.0),
        );
        let delete = button(
            mtm,
            &root,
            target,
            "Supprimer",
            sel!(deleteProfile:),
            rect(445.0, 108.0, 155.0, 34.0),
        );
        let status = label(
            mtm,
            &root,
            "Choisissez un espace ou créez-en un nouveau. Les fenêtres doivent être ouvertes.",
            rect(22.0, 24.0, 576.0, 67.0),
        );
        window.setContentView(Some(&root));
        window.center();
        Self {
            window,
            profiles,
            name,
            positions,
            status,
            descriptions,
            capture,
            selected_buttons: vec![rename, duplicate, apply, geometry, delete],
        }
    }
    pub fn selected_id(&self) -> Option<u32> {
        self.profiles
            .selectedItem()
            .and_then(|i| u32::try_from(i.tag()).ok())
            .filter(|id| *id != 0)
    }
    pub fn populate(&self, catalog: &Catalog, selected: Option<u32>) {
        self.profiles.removeAllItems();
        self.profiles
            .addItemWithTitle(&NSString::from_str("Nouvel espace…"));
        for p in &catalog.profiles {
            // Quoting keeps the reserved New entry distinct, while using the
            // standard popup API to retain AppKit's selection/action routing.
            self.profiles
                .addItemWithTitle(&NSString::from_str(&format!("« {} »", p.name)));
            if let Some(item) = self.profiles.lastItem() {
                item.setTag(p.id as isize);
            }
        }
        let index = selected
            .and_then(|id| catalog.profiles.iter().position(|p| p.id == id))
            .map(|i| i + 1)
            .unwrap_or(0);
        self.profiles.selectItemAtIndex(index as isize);
        self.show_selection(catalog);
    }
    pub fn show_selection(&self, catalog: &Catalog) {
        let profile = self.selected_id().and_then(|id| catalog.profile(id).ok());
        for b in &self.selected_buttons {
            b.setEnabled(profile.is_some());
        }
        for p in &self.positions {
            p.setEnabled(profile.is_some());
            p.selectItemAtIndex(0);
        }
        self.capture
            .setTitle(&NSString::from_str(if profile.is_some() {
                "Recapturer les deux fenêtres…"
            } else {
                "Enregistrer deux fenêtres…"
            }));
        self.name.setStringValue(&NSString::from_str(
            &profile
                .map(|p| p.name.clone())
                .unwrap_or_else(|| format!("Nouvel espace {}", catalog.next_id)),
        ));
        for (i, description) in self.descriptions.iter().enumerate() {
            let text = if let Some(profile) = profile {
                let w = &profile.windows[i];
                let g = w.geometry;
                format!("{} — {}\n{}", i + 1, app_name(&w.bundle_id), placement(g))
            } else {
                format!("Fenêtre {} à sélectionner", i + 1)
            };
            description.setStringValue(&NSString::from_str(&text));
        }
    }
    pub fn edited_layout(&self, profile: &Profile) -> crate::config::SavedLayout {
        let mut layout = profile.layout();
        for (w, p) in layout.windows.iter_mut().zip(&self.positions) {
            w.geometry = match p.indexOfSelectedItem() {
                1 => NormalizedRect {
                    x: 0.0,
                    y: 0.0,
                    width: 0.5,
                    height: 1.0,
                },
                2 => NormalizedRect {
                    x: 0.5,
                    y: 0.0,
                    width: 0.5,
                    height: 1.0,
                },
                3 => NormalizedRect {
                    x: 0.0,
                    y: 0.0,
                    width: 1.0,
                    height: 1.0,
                },
                _ => w.geometry,
            };
        }
        layout
    }
}
fn app_name(bundle: &str) -> String {
    NSWorkspace::sharedWorkspace()
        .URLForApplicationWithBundleIdentifier(&NSString::from_str(bundle))
        .and_then(|u| u.lastPathComponent())
        .map(|n| n.to_string().trim_end_matches(".app").to_owned())
        .unwrap_or_else(|| bundle.to_owned())
}

fn placement(g: NormalizedRect) -> String {
    let near = |a: f64, b: f64| (a - b).abs() < 0.000001;
    if near(g.y, 0.0) && near(g.height, 1.0) {
        if near(g.x, 0.0) && near(g.width, 1.0) {
            return "100 % de l'écran".into();
        }
        if near(g.width, 0.5) && near(g.x, 0.0) {
            return "Moitié gauche".into();
        }
        if near(g.width, 0.5) && near(g.x, 0.5) {
            return "Moitié droite".into();
        }
    }
    format!(
        "Personnalisée : {:.0} % × {:.0} %",
        g.width * 100.0,
        g.height * 100.0
    )
}
