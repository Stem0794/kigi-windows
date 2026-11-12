//! Small native P4 form. Editing/capture actions stay in the application controller.
use crate::{
    actions::Action,
    config::{Catalog, Profile},
    geometry::NormalizedRect,
};
use objc2::{
    rc::Retained,
    runtime::{AnyObject, Sel},
    sel, AnyThread, MainThreadOnly,
};
use objc2_app_kit::*;
use objc2_foundation::{MainThreadMarker, NSPoint, NSRect, NSSize, NSString};

pub struct Preferences {
    pub window: Retained<NSWindow>,
    pub profiles: Retained<NSPopUpButton>,
    pub name: Retained<NSTextField>,
    pub positions: [Retained<NSPopUpButton>; 2],
    pub status: Retained<NSTextField>,
    tabs: Retained<NSTabView>,
    hotkeys: Retained<NSButton>,
    gestures: Retained<NSButton>,
    mouse: Retained<NSButton>,
    shortcuts_state: Retained<NSTextField>,
    mouse_state: Retained<NSTextField>,
    accessibility: Retained<NSTextField>,
    input_access: Retained<NSTextField>,
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
pub struct RuntimeSettings<'a> {
    pub personal_hotkeys: bool,
    pub shortcuts_received: u64,
    pub registration: &'a str,
    pub gestures: bool,
    pub mouse_requested: bool,
    pub mouse_active: bool,
    pub accessibility: bool,
    pub input_access: bool,
}
fn panel(mtm: MainThreadMarker) -> Retained<NSView> {
    NSView::initWithFrame(NSView::alloc(mtm), rect(0.0, 0.0, 620.0, 510.0))
}
fn add_tab(_mtm: MainThreadMarker, tabs: &NSTabView, title: &str, view: &NSView) {
    let item = unsafe { NSTabViewItem::initWithIdentifier(NSTabViewItem::alloc(), None) };
    item.setLabel(&NSString::from_str(title));
    item.setView(Some(view));
    tabs.addTabViewItem(&item);
}
fn checkbox(
    mtm: MainThreadMarker,
    root: &NSView,
    target: &AnyObject,
    title: &str,
    action: Sel,
    y: f64,
) -> Retained<NSButton> {
    let b = unsafe {
        NSButton::checkboxWithTitle_target_action(
            &NSString::from_str(title),
            Some(target),
            Some(action),
            mtm,
        )
    };
    b.setFrame(rect(22.0, y, 576.0, 28.0));
    root.addSubview(&b);
    b
}
impl Preferences {
    pub fn new(mtm: MainThreadMarker, target: &AnyObject) -> Self {
        let window = unsafe {
            NSWindow::initWithContentRect_styleMask_backing_defer(
                NSWindow::alloc(mtm),
                rect(0.0, 0.0, 680.0, 640.0),
                NSWindowStyleMask::Titled
                    | NSWindowStyleMask::Closable
                    | NSWindowStyleMask::Miniaturizable,
                NSBackingStoreType::Buffered,
                false,
            )
        };
        unsafe {
            window.setReleasedWhenClosed(false);
        }
        window.setTitle(&NSString::from_str("Kigi Windows"));
        let shell = NSView::initWithFrame(NSView::alloc(mtm), rect(0.0, 0.0, 680.0, 640.0));
        let tabs = NSTabView::initWithFrame(NSTabView::alloc(mtm), rect(16.0, 80.0, 648.0, 540.0));
        shell.addSubview(&tabs);
        let root = panel(mtm);
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
        label(
            mtm,
            &root,
            "Dispositions rapides",
            rect(22.0, 80.0, 576.0, 20.0),
        );
        button(
            mtm,
            &root,
            target,
            "Travail — Chrome 100 %",
            sel!(workspaceWork:),
            rect(18.0, 37.0, 193.0, 34.0),
        );
        button(
            mtm,
            &root,
            target,
            "Réunion — 50/50",
            sel!(workspaceMeeting:),
            rect(222.0, 37.0, 184.0, 34.0),
        );
        button(
            mtm,
            &root,
            target,
            "Restaurer",
            sel!(workspaceRestore:),
            rect(417.0, 37.0, 183.0, 34.0),
        );
        add_tab(mtm, &tabs, "Espaces", &root);

        let keyboard = panel(mtm);
        label(
            mtm,
            &keyboard,
            "Raccourcis clavier",
            rect(22.0, 463.0, 576.0, 28.0),
        );
        let hotkeys = checkbox(
            mtm,
            &keyboard,
            target,
            "Activer les raccourcis personnels",
            sel!(togglePersonal:),
            414.0,
        );
        let bindings = Action::ALL
            .iter()
            .map(|a| a.label())
            .collect::<Vec<_>>()
            .join("\n");
        label(mtm, &keyboard, &bindings, rect(30.0, 233.0, 560.0, 163.0));
        let shortcuts_state = label(
            mtm,
            &keyboard,
            "Vérification des raccourcis…",
            rect(22.0, 181.0, 576.0, 43.0),
        );
        button(
            mtm,
            &keyboard,
            target,
            "Réenregistrer les raccourcis",
            sel!(retryHotkeys:),
            rect(18.0, 126.0, 285.0, 36.0),
        );
        button(
            mtm,
            &keyboard,
            target,
            "Diagnostic des raccourcis",
            sel!(shortcutDiagnostics:),
            rect(315.0, 126.0, 285.0, 36.0),
        );
        label(mtm,&keyboard,"Les combinaisons sont fixes à cette étape. En cas de conflit avec Hammerspoon, quittez-le temporairement puis réenregistrez les raccourcis.",rect(22.0,55.0,576.0,58.0));
        add_tab(mtm, &tabs, "Raccourcis", &keyboard);

        let input = panel(mtm);
        label(
            mtm,
            &input,
            "Gestes du trackpad",
            rect(22.0, 463.0, 576.0, 28.0),
        );
        let gestures = checkbox(
            mtm,
            &input,
            target,
            "Activer les gestes de bord du trackpad",
            sel!(toggleGestures:),
            417.0,
        );
        label(mtm,&input,"Un doigt, départ dans les 5 % extrêmes, mouvement horizontal jusqu'au milieu.\nBord gauche : Teams. Bord droit : Chrome à 100 %.",rect(22.0,337.0,576.0,66.0));
        label(mtm,&input,"Les gestes sont désactivés au lancement et après une interruption. Réactivez-les ici après la veille.",rect(22.0,279.0,576.0,48.0));
        label(mtm, &input, "Souris", rect(22.0, 228.0, 576.0, 26.0));
        let mouse = checkbox(
            mtm,
            &input,
            target,
            "Bloquer le défilement horizontal des souris",
            sel!(toggleMouseHorizontal:),
            187.0,
        );
        let mouse_state = label(
            mtm,
            &input,
            "Filtre désactivé",
            rect(22.0, 125.0, 576.0, 52.0),
        );
        label(mtm,&input,"Option mémorisée, compatible Magic Mouse et d'autres souris reconnues. Le vertical est conservé. Les événements identifiés comme trackpad ou incertains sont laissés passer.",rect(22.0,40.0,576.0,68.0));
        add_tab(mtm, &tabs, "Gestes et souris", &input);

        let access = panel(mtm);
        label(
            mtm,
            &access,
            "Autorisations macOS",
            rect(22.0, 463.0, 576.0, 28.0),
        );
        let accessibility = label(
            mtm,
            &access,
            "Accessibilité : vérification…",
            rect(22.0, 415.0, 576.0, 28.0),
        );
        label(
            mtm,
            &access,
            "Nécessaire pour placer les fenêtres et filtrer le défilement de la souris.",
            rect(22.0, 369.0, 576.0, 37.0),
        );
        button(
            mtm,
            &access,
            target,
            "Demander / revérifier",
            sel!(permission:),
            rect(18.0, 316.0, 285.0, 36.0),
        );
        button(
            mtm,
            &access,
            target,
            "Afficher la copie active dans Finder",
            sel!(revealApp:),
            rect(315.0, 316.0, 285.0, 36.0),
        );
        let input_access = label(
            mtm,
            &access,
            "Surveillance de l'entrée : vérification…",
            rect(22.0, 257.0, 576.0, 30.0),
        );
        label(mtm,&access,"L'autorisation de surveillance est demandée lors de l'activation volontaire de l'écoute du trackpad, si macOS l'exige.",rect(22.0,196.0,576.0,47.0));
        label(
            mtm,
            &access,
            "Chrome : Automatisation",
            rect(22.0, 145.0, 576.0, 25.0),
        );
        label(mtm,&access,"La demande apparaît lorsque vous utilisez ChatGPT. Autorisez Google Chrome pour retrouver un onglet existant.",rect(22.0,92.0,576.0,45.0));
        button(
            mtm,
            &access,
            target,
            "Tester ChatGPT dans Chrome",
            sel!(openChatGpt:),
            rect(18.0, 35.0, 285.0, 36.0),
        );
        button(
            mtm,
            &access,
            target,
            "Diagnostic Accessibilité",
            sel!(permissionDiagnostics:),
            rect(315.0, 35.0, 285.0, 36.0),
        );
        add_tab(mtm, &tabs, "Autorisations", &access);

        let diagnostics = panel(mtm);
        label(
            mtm,
            &diagnostics,
            &format!("Kigi Windows {} — diagnostics", env!("CARGO_PKG_VERSION")),
            rect(22.0, 463.0, 576.0, 28.0),
        );
        label(mtm,&diagnostics,"Reproduisez le problème puis copiez ou exportez le rapport avant de quitter. Les 200 derniers événements restent en mémoire, sans envoi automatique.",rect(22.0,382.0,576.0,66.0));
        button(
            mtm,
            &diagnostics,
            target,
            "Copier les logs",
            sel!(copyLogs:),
            rect(18.0, 326.0, 285.0, 36.0),
        );
        button(
            mtm,
            &diagnostics,
            target,
            "Exporter les logs…",
            sel!(exportLogs:),
            rect(315.0, 326.0, 285.0, 36.0),
        );
        button(
            mtm,
            &diagnostics,
            target,
            "Détecter les fenêtres",
            sel!(inventory:),
            rect(18.0, 268.0, 285.0, 36.0),
        );
        button(
            mtm,
            &diagnostics,
            target,
            "Tester / restaurer la fenêtre",
            sel!(testWindow:),
            rect(315.0, 268.0, 285.0, 36.0),
        );
        button(
            mtm,
            &diagnostics,
            target,
            "Test local du trackpad",
            sel!(localProbe:),
            rect(18.0, 210.0, 285.0, 36.0),
        );
        button(
            mtm,
            &diagnostics,
            target,
            "Diagnostic global du trackpad",
            sel!(globalProbe:),
            rect(315.0, 210.0, 285.0, 36.0),
        );
        label(mtm,&diagnostics,"Le diagnostic global remplace l'écoute active des gestes. Relancez les gestes depuis leur onglet après l'essai.",rect(22.0,149.0,576.0,45.0));
        button(
            mtm,
            &diagnostics,
            target,
            "Afficher la configuration dans Finder",
            sel!(revealConfig:),
            rect(18.0, 94.0, 582.0, 36.0),
        );
        label(
            mtm,
            &diagnostics,
            "Fermer la fenêtre laisse Kigi en arrière-plan. ⌘Q quitte l'application.",
            rect(22.0, 35.0, 576.0, 43.0),
        );
        add_tab(mtm, &tabs, "Diagnostics", &diagnostics);
        tabs.selectTabViewItemAtIndex(0);
        let status = label(
            mtm,
            &shell,
            "Kigi est prêt. Choisissez une rubrique pour gérer vos réglages.",
            rect(22.0, 12.0, 636.0, 56.0),
        );
        window.setContentView(Some(&shell));
        window.center();
        Self {
            window,
            profiles,
            name,
            positions,
            status,
            tabs,
            hotkeys,
            gestures,
            mouse,
            shortcuts_state,
            mouse_state,
            accessibility,
            input_access,
            descriptions,
            capture,
            selected_buttons: vec![rename, duplicate, apply, geometry, delete],
        }
    }
    pub fn update_runtime(&self, state: RuntimeSettings<'_>) {
        self.hotkeys.setState(if state.personal_hotkeys {
            NSControlStateValueOn
        } else {
            NSControlStateValueOff
        });
        self.gestures.setState(if state.gestures {
            NSControlStateValueOn
        } else {
            NSControlStateValueOff
        });
        self.mouse.setState(if state.mouse_requested {
            NSControlStateValueOn
        } else {
            NSControlStateValueOff
        });
        let conflict =
            state.registration.contains("indisponible") || state.registration.contains("refusée");
        self.shortcuts_state
            .setStringValue(&NSString::from_str(&format!(
                "{} — {} action(s) reçue(s). {}",
                if state.personal_hotkeys {
                    "Raccourcis personnels activés"
                } else {
                    "Raccourcis personnels suspendus"
                },
                state.shortcuts_received,
                if conflict {
                    "Conflit ou erreur : ouvrez le diagnostic."
                } else {
                    ""
                }
            )));
        self.mouse_state.setStringValue(&NSString::from_str(if state.mouse_active { "Filtre actif" } else if state.mouse_requested {
            "Filtre demandé mais inactif. Désactivez puis réactivez l'option après vérification des autorisations."
        } else { "Filtre désactivé" }));
        self.accessibility
            .setStringValue(&NSString::from_str(if state.accessibility {
                "Accessibilité : autorisée"
            } else {
                "Accessibilité : non autorisée pour cette copie de Kigi"
            }));
        self.input_access
            .setStringValue(&NSString::from_str(if state.input_access {
                "Surveillance de l'entrée : autorisée"
            } else {
                "Surveillance de l'entrée : non autorisée"
            }));
    }
    pub fn show_permissions(&self) {
        self.tabs.selectTabViewItemAtIndex(3);
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
