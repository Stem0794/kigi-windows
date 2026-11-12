use crate::{
    diagnostics,
    gestures::{Contact, CornerProbe},
    hotkey::Hotkey,
    trackpad::Tap,
    windows::{self, Snapshot},
};
use objc2::{
    define_class, msg_send,
    rc::Retained,
    runtime::{AnyObject, Sel},
    sel, DefinedClass, MainThreadOnly,
};
use objc2_app_kit::*;
use objc2_foundation::{
    ns_string, MainThreadMarker, NSObject, NSObjectProtocol, NSPoint, NSProcessInfo, NSRect,
    NSSize, NSString,
};
use std::{
    cell::{Cell, OnceCell, RefCell},
    time::Instant,
};

thread_local! { static CONTROLLER: RefCell<Option<Retained<Controller>>> = const { RefCell::new(None) }; }

#[derive(Default)]
struct State {
    snapshot: RefCell<Option<Snapshot>>,
    status: OnceCell<Retained<NSMenuItem>>,
    hotkey_status: OnceCell<Retained<NSMenuItem>>,
    hotkey_count: Cell<u64>,
    permission_notice: Cell<bool>,
    local_window: RefCell<Option<Retained<NSWindow>>>,
    tap: RefCell<Option<Tap>>,
}
define_class!(
    // NSObject has no subclass requirements. All UI and ivars stay on main.
    #[unsafe(super = NSObject)]
    #[thread_kind = MainThreadOnly]
    #[ivars = State]
    struct Controller;
    unsafe impl NSObjectProtocol for Controller {}
    impl Controller {
        #[unsafe(method(copyLogs:))]
        fn copy_logs(&self, _: Option<&AnyObject>) {
            self.record_state("diagnostic.copy");
            let report = self.log_report();
            let pasteboard = NSPasteboard::generalPasteboard();
            pasteboard.clearContents();
            let kind = unsafe { NSPasteboardTypeString };
            if pasteboard.setString_forType(&NSString::from_str(&report), kind) {
                self.status("Logs copiés. Collez-les dans votre message pour le débogage.");
            } else { self.status("Impossible de copier les logs."); }
        }
        #[unsafe(method(exportLogs:))]
        fn export_logs(&self, _: Option<&AnyObject>) {
            let panel = NSSavePanel::savePanel(self.mtm());
            panel.setNameFieldStringValue(&NSString::from_str(&format!("Kigi-diagnostic-{}.txt", env!("CARGO_PKG_VERSION"))));
            panel.setCanCreateDirectories(true);
            if panel.runModal() != NSModalResponseOK { return; }
            let Some(path) = panel.URL().and_then(|u| u.path()) else {
                self.status("Destination des logs indisponible."); return;
            };
            self.record_state("diagnostic.export");
            match std::fs::write(path.to_string(), self.log_report()) {
                Ok(()) => self.status("Fichier de diagnostic enregistré. Vous pouvez le transmettre pour le débogage."),
                Err(e) => {
                    diagnostics::record("diagnostic.export_failed", format!("io_kind={:?}", e.kind()));
                    self.status("Impossible d'enregistrer les logs. Utilisez Copier les logs ou choisissez un autre dossier.");
                }
            }
        }
        #[unsafe(method(shortcutDiagnostics:))]
        fn shortcut_diagnostics(&self, _: Option<&AnyObject>) {
            let registration = self.ivars().hotkey_status.get().map(|i| i.title().to_string()).unwrap_or_default();
            let result = self.ivars().status.get().map(|i| i.title().to_string()).unwrap_or_default();
            alert(self.mtm(), "Diagnostic du raccourci", &format!(
                "Kigi Windows {}\n{}\nAccessibilité : {}\nDernier résultat : {}\n\nAppuyez sur Control + Option + T depuis Chrome. Le compteur doit augmenter à l'appui.\nCompteur inchangé : raccourci non reçu (conflit ou routage).\nCompteur augmenté : consultez le dernier résultat AX ci-dessus.\nLe menu Tester / restaurer permet d'essayer la même action sans raccourci.",
                env!("CARGO_PKG_VERSION"), registration,
                if windows::trusted() { "autorisée" } else { "non autorisée" }, result
            ));
        }
        #[unsafe(method(permission:))]
        fn permission(&self, _: Option<&AnyObject>) {
            let granted = windows::request_permission();
            diagnostics::record("permission.ax.request", format!("trusted={granted}"));
            self.status(if granted { "Accessibilité autorisée." } else { "Autorisez Kigi dans Réglages Système > Confidentialité et sécurité > Accessibilité, puis réessayez." });
        }
        #[unsafe(method(inventory:))]
        fn inventory(&self, _: Option<&AnyObject>) {
            self.record_state("windows.inventory.start");
            match windows::inventory() {
                Ok(text) => {
                    diagnostics::record("windows.inventory.complete", format!("lines={}", text.lines().count()));
                    alert(self.mtm(), "Fenêtres détectées (sans titres)", &text);
                }
                Err(e) => self.status(&e),
            }
        }
        #[unsafe(method(testWindow:))]
        fn test_window(&self, _: Option<&AnyObject>) {
            self.record_state("action.window.start");
            if !windows::trusted() {
                self.status("Déplacement bloqué : Accessibilité non autorisée pour cette application.");
                if !self.ivars().permission_notice.replace(true) {
                    alert(self.mtm(), "Autorisation Accessibilité nécessaire", "macOS bloque le contrôle des fenêtres pour cette application.\n\nDans Réglages Système > Confidentialité et sécurité > Accessibilité, autorisez Kigi Windows.\n\nSi une ancienne entrée est déjà cochée, retirez-la puis ajoutez l'application actuelle depuis Applications, et relancez Kigi.\n\nVous pouvez copier ou exporter les logs depuis le menu Kigi.");
                }
                return;
            }
            self.ivars().permission_notice.set(false);
            let mut snapshot = self.ivars().snapshot.borrow_mut();
            let result = if let Some(saved) = snapshot.as_ref() {
                diagnostics::record("action.window.restore", "snapshot=true");
                let result = saved.restore();
                if result.is_ok() { snapshot.take(); }
                result
            } else {
                windows::move_frontmost(self.mtm(), &mut snapshot)
            };
            drop(snapshot);
            diagnostics::record("action.window.complete", format!("success={}", result.is_ok()));
            self.status(&result.unwrap_or_else(|e| e));
        }
        #[unsafe(method(localProbe:))]
        fn local_probe(&self, _: Option<&AnyObject>) {
            diagnostics::record("trackpad.local.open", "mode=top_left_corner width=0.10");
            let mtm = self.mtm();
            // New probe each opening; no stale finger identity across sessions.
            let window = unsafe {
                NSWindow::initWithContentRect_styleMask_backing_defer(NSWindow::alloc(mtm),
                    NSRect::new(NSPoint::new(0.0,0.0),NSSize::new(600.0,240.0)),
                    NSWindowStyleMask::Titled | NSWindowStyleMask::Closable,
                    NSBackingStoreType::Buffered, false)
            };
            unsafe { window.setReleasedWhenClosed(false) };
            window.setTitle(ns_string!("Trackpad local — aucun geste global activé"));
            let view = TouchView::new(mtm);
            let text = NSTextField::labelWithString(ns_string!("Diagnostic local uniquement.\nGardez le pointeur dans cette fenêtre, puis levez les doigts.\nPosez un doigt dans le coin haut-gauche : les 10 % les plus à gauche. Glissez vers le bas.\nLe résultat s'affiche ici ; aucune fenêtre tierce n'est déplacée."), mtm);
            text.setFrame(NSRect::new(NSPoint::new(20.0,35.0),NSSize::new(560.0,170.0)));
            view.addSubview(&text);
            // hitTest routes the whole diagnostic area to TouchView.
            // Keep the informational label non-selectable and non-editable.
            text.setSelectable(false);
            text.setEditable(false);
            view.ivars().label.set(text).ok();
            window.setContentView(Some(&view));
            window.center();
            window.makeKeyAndOrderFront(None);
            #[allow(deprecated)]
            NSApplication::sharedApplication(mtm).activateIgnoringOtherApps(true);
            let previous = self.ivars().local_window.replace(Some(window));
            if let Some(old) = previous { old.close(); }
        }
        #[unsafe(method(globalProbe:))]
        fn global_probe(&self, _: Option<&AnyObject>) {
            let old = self.ivars().tap.borrow_mut().take();
            if let Some(tap) = old {
                let report = tap.report();
                diagnostics::record("trackpad.global.stop", &report);
                drop(tap);
                self.status("Diagnostic global arrêté.");
                alert(self.mtm(), "Résultat du spike global", &report);
            } else {
                match Tap::start() {
                    Ok(tap) => {
                        diagnostics::record("trackpad.global.start", "passive=true actions=false");
                        self.ivars().tap.replace(Some(tap));
                        self.status("Diagnostic global en cours. Testez le geste dans Chrome/Teams, puis recliquez sur Démarrer/arrêter pour voir le résultat.");
                    }
                    Err(e) => self.status(&e),
                }
            }
        }
        #[unsafe(method(quit:))]
        fn quit(&self, _: Option<&AnyObject>) {
            diagnostics::record("application.quit", "requested=true");
            self.ivars().tap.borrow_mut().take();
            NSApplication::sharedApplication(self.mtm()).stop(None);
        }
    }
);
impl Controller {
    fn new(mtm: MainThreadMarker) -> Retained<Self> {
        let this = Self::alloc(mtm).set_ivars(State::default());
        unsafe { msg_send![super(this), init] }
    }
    fn record_state(&self, event: &'static str) {
        diagnostics::record(
            event,
            format!(
                "accessibility={} received={} snapshot={}",
                windows::trusted(),
                self.ivars().hotkey_count.get(),
                self.ivars().snapshot.borrow().is_some()
            ),
        );
    }
    fn log_report(&self) -> String {
        let shortcut = self
            .ivars()
            .hotkey_status
            .get()
            .map(|i| i.title().to_string())
            .unwrap_or_default();
        let result = self
            .ivars()
            .status
            .get()
            .map(|i| i.title().to_string())
            .unwrap_or_default();
        let mut executable = std::env::current_exe()
            .map(|p| p.display().to_string())
            .unwrap_or_else(|_| "unknown".into());
        if let Ok(home) = std::env::var("HOME") {
            if !home.is_empty() && executable.starts_with(&(home.clone() + "/")) {
                executable = format!("~{}", &executable[home.len()..]);
            }
        }
        let trackpad = self
            .ivars()
            .tap
            .borrow()
            .as_ref()
            .map(|t| t.report())
            .unwrap_or_else(|| "diagnostic global arrêté".into());
        let os = NSProcessInfo::processInfo().operatingSystemVersionString();
        let exported_at = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        diagnostics::report(&format!("macOS={os}\narchitecture={}\nbundle=io.kigi.windows\nexecutable={executable}\nexported_at_unix_utc={exported_at}\naccessibility={}\n{shortcut}\nlast_result={result}\ntrackpad={trackpad}",
            std::env::consts::ARCH, windows::trusted()))
    }

    fn status(&self, text: &str) {
        diagnostics::record("status", text);
        if let Some(item) = self.ivars().status.get() {
            item.setTitle(&NSString::from_str(text));
        }
    }
}

struct TouchState {
    probe: RefCell<CornerProbe>,
    clock: Instant,
    count: Cell<u64>,
    last_ui: Cell<Instant>,
    label: OnceCell<Retained<NSTextField>>,
}
define_class!(
    #[unsafe(super = NSView)]
    #[thread_kind = MainThreadOnly]
    #[ivars = TouchState]
    struct TouchView;
    unsafe impl NSObjectProtocol for TouchView {}
    impl TouchView {
        // The diagnostic label must not steal the trackpad's hit-tested view.
        #[unsafe(method(hitTest:))]
        fn hit_test(&self, point: NSPoint) -> Option<&NSView> {
            let hit: Option<Retained<NSView>> = unsafe { msg_send![super(self), hitTest:point] };
            hit.map(|_| &**self)
        }
        #[unsafe(method(touchesBeganWithEvent:))]
        fn began(&self, event: &NSEvent) { self.touch(event, true); }
        #[unsafe(method(touchesMovedWithEvent:))]
        fn moved(&self, event: &NSEvent) { self.touch(event, false); }
        #[unsafe(method(touchesEndedWithEvent:))]
        fn ended(&self, _: &NSEvent) { self.ivars().probe.borrow_mut().cancel(); }
        #[unsafe(method(touchesCancelledWithEvent:))]
        fn cancelled(&self, _: &NSEvent) { self.ivars().probe.borrow_mut().cancel(); }
    }
);
impl TouchView {
    fn new(mtm: MainThreadMarker) -> Retained<Self> {
        let this = Self::alloc(mtm).set_ivars(TouchState {
            probe: RefCell::new(CornerProbe::default()),
            clock: Instant::now(),
            count: Cell::new(0),
            last_ui: Cell::new(Instant::now()),
            label: OnceCell::new(),
        });
        let view: Retained<Self> = unsafe {
            msg_send![super(this), initWithFrame:NSRect::new(NSPoint::new(0.0,0.0),NSSize::new(600.0,240.0))]
        };
        #[allow(deprecated)]
        view.setAcceptsTouchEvents(true);
        view.setAllowedTouchTypes(NSTouchTypeMask::Indirect);
        view.setWantsRestingTouches(true);
        view
    }
    fn touch(&self, event: &NSEvent, began: bool) {
        let touches = event.touchesMatchingPhase_inView(NSTouchPhase::Touching, Some(self));
        let mut probe = self.ivars().probe.borrow_mut();
        if touches.count() != 1 {
            probe.cancel();
            return;
        }
        let Some(touch) = touches.anyObject() else {
            return;
        };
        let point = touch.normalizedPosition();
        let identity = touch.identity();
        let key: usize = unsafe { msg_send![&*identity, hash] };
        let contact = Contact {
            identity: key,
            device: touch
                .device()
                .map(|d| Retained::as_ptr(&d) as usize)
                .unwrap_or(0),
            x: point.x,
            y: point.y,
            ms: self.ivars().clock.elapsed().as_millis() as u64,
        };
        if began {
            if touch.phase() == NSTouchPhase::Began {
                probe.begin(contact, 1);
            } else {
                probe.cancel();
            }
        } else if probe.moved(contact, 1) {
            self.ivars().count.set(self.ivars().count.get() + 1);
            diagnostics::record(
                "trackpad.local.candidate",
                format!("count={}", self.ivars().count.get()),
            );
        }
        if self.ivars().last_ui.get().elapsed().as_millis() >= 33 {
            self.ivars().last_ui.set(Instant::now());
            if let Some(label) = self.ivars().label.get() {
                label.setStringValue(&NSString::from_str(&format!(
                    "Diagnostic LOCAL : x={:.2}, y={:.2}, phase={:?}.\nGeste(s) haut-gauche vers bas : {}.\nLevez les doigts pour recommencer. Aucune action globale.",
                    point.x, point.y, touch.phase(), self.ivars().count.get()
                )));
            }
        }
    }
}
fn alert(mtm: MainThreadMarker, title: &str, text: &str) {
    let alert = NSAlert::new(mtm);
    alert.setMessageText(&NSString::from_str(title));
    alert.setInformativeText(&NSString::from_str(text));
    alert.runModal();
}
fn menu_action(
    menu: &NSMenu,
    controller: &Controller,
    title: &str,
    action: Sel,
) -> Retained<NSMenuItem> {
    let item = unsafe {
        NSMenuItem::initWithTitle_action_keyEquivalent(
            NSMenuItem::alloc(controller.mtm()),
            &NSString::from_str(title),
            Some(action),
            ns_string!(""),
        )
    };
    unsafe {
        item.setTarget(Some(controller));
    }
    menu.addItem(&item);
    item
}
pub fn shortcut_event(error: Option<i32>) {
    diagnostics::record("shortcut.received", format!("parameter_error={error:?}"));
    CONTROLLER.with(|slot| {
        if let Some(controller) = slot.borrow().as_ref() {
            let text = if let Some(code) = error {
                format!("Raccourci : événement reçu, lecture ID refusée ({code})")
            } else {
                let count = controller.ivars().hotkey_count.get() + 1;
                controller.ivars().hotkey_count.set(count);
                format!("Raccourci ⌃⌥T : enregistré — reçu {count} fois")
            };
            if let Some(item) = controller.ivars().hotkey_status.get() {
                item.setTitle(&NSString::from_str(&text));
            }
        }
    });
}
pub fn schedule_test() {
    diagnostics::record("action.window.queued", "source=shortcut");
    CONTROLLER.with(|slot| {
        if let Some(controller) = slot.borrow().as_ref() {
            // Use the documented main-thread queue, including common run-loop modes.
            let _: () = unsafe { msg_send![&**controller, performSelectorOnMainThread:sel!(testWindow:), withObject:std::ptr::null::<AnyObject>(), waitUntilDone:false] };
        }
    });
}
pub fn run() {
    let mtm = MainThreadMarker::new().expect("AppKit doit démarrer sur le thread principal");
    objc2::rc::autoreleasepool(|_| {
        diagnostics::record(
            "application.start",
            format!(
                "version={} architecture={} accessibility={}",
                env!("CARGO_PKG_VERSION"),
                std::env::consts::ARCH,
                windows::trusted()
            ),
        );
        let app = NSApplication::sharedApplication(mtm);
        app.setActivationPolicy(NSApplicationActivationPolicy::Accessory);
        let controller = Controller::new(mtm);
        let status_bar = NSStatusBar::systemStatusBar();
        let status_item = status_bar.statusItemWithLength(-1.0);
        if let Some(button) = status_item.button(mtm) {
            button.setTitle(ns_string!("Kigi"));
        }
        let menu = NSMenu::new(mtm);
        let status = NSMenuItem::new(mtm);
        status.setTitle(ns_string!(
            "P0 — prêt. Accessibilité demandée uniquement sur action."
        ));
        status.setEnabled(false);
        menu.addItem(&status);
        controller.ivars().status.set(status).ok();
        let shortcut = menu_action(
            &menu,
            &controller,
            "Raccourci ⌃⌥T : enregistrement en cours",
            sel!(shortcutDiagnostics:),
        );
        controller.ivars().hotkey_status.set(shortcut).ok();
        menu_action(
            &menu,
            &controller,
            "Autoriser / vérifier Accessibilité",
            sel!(permission:),
        );
        menu_action(
            &menu,
            &controller,
            "Détecter les fenêtres (sans titres)",
            sel!(inventory:),
        );
        menu_action(
            &menu,
            &controller,
            "Tester / restaurer la fenêtre active — ⌃⌥T",
            sel!(testWindow:),
        );
        menu.addItem(&NSMenuItem::separatorItem(mtm));
        menu_action(
            &menu,
            &controller,
            "Tester le geste dans une fenêtre locale",
            sel!(localProbe:),
        );
        menu_action(
            &menu,
            &controller,
            "Démarrer / arrêter le diagnostic global passif",
            sel!(globalProbe:),
        );
        menu.addItem(&NSMenuItem::separatorItem(mtm));
        menu_action(
            &menu,
            &controller,
            "Copier les logs de diagnostic",
            sel!(copyLogs:),
        );
        menu_action(&menu, &controller, "Exporter les logs…", sel!(exportLogs:));
        menu.addItem(&NSMenuItem::separatorItem(mtm));
        menu_action(&menu, &controller, "Quitter", sel!(quit:));
        status_item.setMenu(Some(&menu));
        CONTROLLER.with(|c| c.replace(Some(controller.clone())));
        let hotkey = Hotkey::register();
        let registration = match &hotkey {
            Ok(_) => "Raccourci ⌃⌥T : enregistré — reçu 0 fois".to_owned(),
            Err(e) => format!("Raccourci non enregistré : {e}"),
        };
        diagnostics::record("shortcut.registration", &registration);
        if let Some(item) = controller.ivars().hotkey_status.get() {
            item.setTitle(&NSString::from_str(&registration));
        }
        if let Err(e) = &hotkey {
            controller.status(e);
        }
        app.run();
        drop(hotkey);
        CONTROLLER.with(|c| c.borrow_mut().take());
        controller.ivars().tap.borrow_mut().take();
        if let Some(window) = controller.ivars().local_window.borrow_mut().take() {
            window.close();
        }
        status_bar.removeStatusItem(&status_item);
    });
}
