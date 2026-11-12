use crate::{
    actions::Action,
    applications, diagnostics,
    gestures::{Contact, CornerProbe, Edge},
    hotkey::Hotkeys,
    trackpad::Tap,
    windows::{self, Snapshot},
};
use objc2::{
    define_class, msg_send,
    rc::Retained,
    runtime::{AnyObject, ProtocolObject, Sel},
    sel, DefinedClass, MainThreadOnly,
};
use objc2_app_kit::*;
use objc2_foundation::{
    ns_string, MainThreadMarker, NSBundle, NSObject, NSObjectProtocol, NSPoint, NSProcessInfo,
    NSRect, NSSize, NSString,
};
use std::{
    cell::{Cell, OnceCell, RefCell},
    sync::mpsc::{Receiver, TryRecvError},
    time::{Duration, Instant},
};

thread_local! { static CONTROLLER: RefCell<Option<Retained<Controller>>> = const { RefCell::new(None) }; }

#[derive(Default)]
struct PermissionFlow {
    requested: bool,
    polls_left: u8,
}
impl PermissionFlow {
    fn request_once(&mut self, trusted: bool) -> bool {
        if trusted || self.requested {
            return false;
        }
        self.requested = true;
        self.polls_left = 15;
        true
    }
    fn next_poll(&mut self, trusted: bool) -> bool {
        if trusted {
            self.polls_left = 0;
            return false;
        }
        self.polls_left = self.polls_left.saturating_sub(1);
        self.polls_left > 0
    }
}

struct PendingOpen {
    action: Action,
    result: Receiver<Result<i32, isize>>,
    pid: Option<i32>,
    started: Instant,
}

#[derive(Default)]
struct State {
    snapshot: RefCell<Option<Snapshot>>,
    meeting_snapshot: RefCell<Option<windows::MeetingSnapshot>>,
    hotkeys: RefCell<Option<Hotkeys>>,
    personal_enabled: Cell<bool>,
    personal_item: OnceCell<Retained<NSMenuItem>>,
    pending: RefCell<Option<PendingOpen>>,
    picking: Cell<bool>,
    menu_open: Cell<bool>,
    gesture_item: OnceCell<Retained<NSMenuItem>>,
    queued_gesture: Cell<Option<Edge>>,
    status: OnceCell<Retained<NSMenuItem>>,
    hotkey_status: OnceCell<Retained<NSMenuItem>>,
    hotkey_count: Cell<u64>,
    permission_notice: Cell<bool>,
    permission_flow: RefCell<PermissionFlow>,
    permission_item: OnceCell<Retained<NSMenuItem>>,
    last_permission: Cell<Option<bool>>,
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
            let registration = self.ivars().hotkeys.borrow().as_ref().map(|h| h.report.clone()).unwrap_or_default();
            let result = self.ivars().status.get().map(|i| i.title().to_string()).unwrap_or_default();
            alert(self.mtm(), "Diagnostic des raccourcis", &format!(
                "Kigi Windows {}\n{registration}\nActions reçues : {}\nAccessibilité : {}\nDernier résultat : {result}\n\nEn cas de conflit, quittez temporairement Hammerspoon puis choisissez Réenregistrer les raccourcis. Les menus exécutent les mêmes actions. Aucun fichier Hammerspoon n'est modifié.",
                env!("CARGO_PKG_VERSION"), self.ivars().hotkey_count.get(), windows::trusted()));
        }
        #[unsafe(method(focusTeams:))]
        fn focus_teams(&self, _: Option<&AnyObject>) { self.execute(Action::Teams); }
        #[unsafe(method(focusBrowser:))]
        fn focus_browser(&self, _: Option<&AnyObject>) { self.execute(Action::Browser); }
        #[unsafe(method(openChatGpt:))]
        fn open_chatgpt(&self, _: Option<&AnyObject>) { self.execute(Action::ChatGpt); }
        #[unsafe(method(maximizeWindow:))]
        fn maximize_window(&self, _: Option<&AnyObject>) { self.execute(Action::Maximize); }
        #[unsafe(method(pairLeft:))]
        fn pair_left(&self, _: Option<&AnyObject>) { self.execute(Action::PairLeft); }
        #[unsafe(method(pairRight:))]
        fn pair_right(&self, _: Option<&AnyObject>) { self.execute(Action::PairRight); }
        #[unsafe(method(workspaceWork:))]
        fn workspace_work(&self, _: Option<&AnyObject>) {
            if self.ivars().picking.get() || !self.window_permission() {
                return;
            }
            self.cancel_open();
            self.cancel_gesture_dispatch();
            let result = windows::work_browser(self.mtm());
            if result.is_ok() {
                // A named Work scene ends the previous temporary Meeting scene.
                self.ivars().meeting_snapshot.borrow_mut().take();
            }
            diagnostics::record("workspace.work", format!("success={}", result.is_ok()));
            self.status(&result.unwrap_or_else(|e| e));
        }
        #[unsafe(method(workspaceMeeting:))]
        fn workspace_meeting(&self, _: Option<&AnyObject>) {
            if self.ivars().picking.get() || !self.window_permission() {
                return;
            }
            self.cancel_open();
            self.cancel_gesture_dispatch();
            let result = windows::meeting_browser(
                self.mtm(),
                &mut self.ivars().meeting_snapshot.borrow_mut(),
            );
            diagnostics::record("workspace.meeting", format!("success={}", result.is_ok()));
            self.status(&result.unwrap_or_else(|e| e));
        }
        #[unsafe(method(workspaceRestore:))]
        fn workspace_restore(&self, _: Option<&AnyObject>) {
            if self.ivars().picking.get() || !self.window_permission() {
                return;
            }
            self.cancel_open();
            self.cancel_gesture_dispatch();
            let result = if let Some(snapshot) = self.ivars().meeting_snapshot.borrow().as_ref() {
                snapshot.restore()
            } else {
                Ok("Aucune scène de réunion à restaurer.".into())
            };
            if result.is_ok() {
                self.ivars().meeting_snapshot.borrow_mut().take();
            }
            diagnostics::record("workspace.restore", format!("success={}", result.is_ok()));
            self.status(&result.unwrap_or_else(|e| e));
        }
        #[unsafe(method(openPoll:))]
        fn open_poll(&self, _: Option<&AnyObject>) { self.poll_open(); }
        #[unsafe(method(retryHotkeys:))]
        fn retry_hotkeys(&self, _: Option<&AnyObject>) { self.register_hotkeys(); }
        #[unsafe(method(togglePersonal:))]
        fn toggle_personal(&self, _: Option<&AnyObject>) {
            self.ivars().personal_enabled.set(!self.ivars().personal_enabled.get());
            self.cancel_open();
            self.register_hotkeys();
        }
        #[unsafe(method(permission:))]
        fn permission(&self, _: Option<&AnyObject>) {
            let trusted = self.refresh_permission();
            if trusted { self.status("Accessibilité reconnue pour ce processus. Vous pouvez tester la fenêtre."); return; }
            let request = self.ivars().permission_flow.borrow_mut().request_once(trusted);
            if request {
                let immediate = windows::request_permission();
                diagnostics::record("permission.ax.request", format!("immediate_result={immediate} asynchronous=true"));
                if self.refresh_permission() { return; }
                self.status("Demande envoyée. Activez la copie actuelle dans Accessibilité ; l'état sera revérifié sans nouvelle demande.");
                self.schedule_permission_poll();
            } else {
                self.status("Vérification sans nouvelle demande : macOS ne reconnaît pas encore l'accès pour ce processus. Utilisez le diagnostic Accessibilité.");
            }
        }
        #[unsafe(method(permissionPoll:))]
        fn permission_poll(&self, _: Option<&AnyObject>) {
            let trusted = self.refresh_permission();
            if self.ivars().permission_flow.borrow_mut().next_poll(trusted) {
                self.schedule_permission_poll();
            }
        }
        #[unsafe(method(permissionDiagnostics:))]
        fn permission_diagnostics(&self, _: Option<&AnyObject>) {
            self.refresh_permission();
            let probe = windows::permission_probe();
            let bundle = NSBundle::mainBundle().bundlePath().to_string();
            let instances = other_instances();
            alert(self.mtm(), "Diagnostic Accessibilité", &format!(
                "Kigi Windows {} — PID {}\nCopie active : {}\nAutres processus Kigi : {}\n\n{}\n\nUne case cochée pour une ancienne copie ne prouve pas l'autorisation de ce processus. Le menu Afficher la copie active dans Finder permet de choisir la bonne application.\n\nCes prototypes utilisent une signature ad hoc : une mise à jour peut nécessiter une nouvelle autorisation. Si macOS reste en refus, copiez les logs avant de changer encore de version.",
                env!("CARGO_PKG_VERSION"), std::process::id(), bundle, instances.len(), probe
            ));
        }
        #[unsafe(method(revealApp:))]
        fn reveal_app(&self, _: Option<&AnyObject>) {
            let path = NSBundle::mainBundle().bundlePath();
            let shown = NSWorkspace::sharedWorkspace().selectFile_inFileViewerRootedAtPath(Some(&path), ns_string!(""));
            diagnostics::record("application.reveal", format!("success={shown}"));
            if shown { self.status("La copie actuellement lancée est sélectionnée dans Finder."); }
            else { self.status("Impossible d'afficher cette copie dans Finder ; son chemin figure dans les logs."); }
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
            if self.ivars().picking.get() { return; }
            self.cancel_open();
            self.cancel_gesture_dispatch();
            self.record_state("action.window.start");
            if !self.window_permission() { return; }
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
            self.cancel_gesture_dispatch();
            let old = self.ivars().tap.borrow_mut().take();
            self.update_gesture_label();
            if let Some(tap) = old {
                let report = tap.report();
                diagnostics::record("trackpad.global.stop", &report);
                drop(tap);
                self.status("Écoute globale arrêtée.");
                alert(self.mtm(), "Bilan du trackpad", &report);
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
        #[unsafe(method(toggleGestures:))]
        fn toggle_gestures(&self, _: Option<&AnyObject>) {
            if self.gestures_active() {
                self.stop_tap(); self.status("Gestes désactivés."); return;
            }
            if !self.window_permission() { return; }
            self.stop_tap();
            match Tap::start_actions() {
                Ok(tap) => {
                    self.ivars().tap.replace(Some(tap));
                    self.update_gesture_label();
                    diagnostics::record("gestures.start", "left=0.05 right=0.05 target_x=0.50 direction=inward whole_height=true");
                    self.status("Gestes activés : bord gauche vers milieu → Teams ; bord droit vers milieu → Chrome à 100 %.");
                }
                Err(e) => self.status(&e),
            }
        }
        #[unsafe(method(gestureAction:))]
        fn gesture_action(&self, _: Option<&AnyObject>) {
            let Some(edge) = self.ivars().queued_gesture.take() else { return; };
            if !self.gestures_active() || self.ivars().picking.get() || self.ivars().menu_open.get() { return; }
            if !Tap::input_trusted() || !self.refresh_permission() {
                self.stop_tap(); self.status("Gestes arrêtés : une autorisation a été révoquée."); return;
            }
            let action = match edge { Edge::Left => Action::Teams, Edge::Right => Action::Browser };
            diagnostics::record("gesture.action", format!("edge={edge:?} action={action:?}"));
            self.execute(action);
        }
        #[unsafe(method(pauseGestures:))]
        fn pause_gestures(&self, _: Option<&AnyObject>) {
            self.cancel_open();
            let listening = self.ivars().tap.borrow().is_some();
            self.stop_tap();
            if listening {
                self.status("Écoute trackpad arrêtée (veille/session inactive/interruption). Réactivez-la depuis le menu.");
            }
        }
        #[unsafe(method(quit:))]
        fn quit(&self, _: Option<&AnyObject>) {
            self.cancel_permission_poll();
            self.cancel_open();
            diagnostics::record("application.quit", "requested=true");
            self.stop_tap();
            NSApplication::sharedApplication(self.mtm()).stop(None);
        }
    }
    unsafe impl NSMenuDelegate for Controller {
        #[unsafe(method(menuWillOpen:))]
        fn menu_will_open(&self, _: &NSMenu) {
            self.ivars().menu_open.set(true);
            self.cancel_gesture_dispatch();
            self.refresh_permission();
            self.update_gesture_label();
        }
        #[unsafe(method(menuDidClose:))]
        fn menu_did_close(&self, _: &NSMenu) { self.ivars().menu_open.set(false); }
    }
);
impl Controller {
    fn new(mtm: MainThreadMarker) -> Retained<Self> {
        let this = Self::alloc(mtm).set_ivars(State::default());
        unsafe { msg_send![super(this), init] }
    }
    fn gestures_active(&self) -> bool {
        self.ivars()
            .tap
            .borrow()
            .as_ref()
            .is_some_and(Tap::active_actions)
    }
    fn update_gesture_label(&self) {
        if let Some(item) = self.ivars().gesture_item.get() {
            item.setTitle(&NSString::from_str(if self.gestures_active() {
                "Gestes de bord : actifs — désactiver"
            } else {
                "Activer les gestes de bord Teams / Chrome"
            }));
        }
    }
    fn cancel_gesture_dispatch(&self) {
        self.ivars().queued_gesture.take();
        let _: () = unsafe {
            msg_send![objc2::class!(NSObject), cancelPreviousPerformRequestsWithTarget:self, selector:sel!(gestureAction:), object:std::ptr::null::<AnyObject>()]
        };
    }
    fn stop_tap(&self) {
        self.cancel_gesture_dispatch();
        let _: () = unsafe {
            msg_send![objc2::class!(NSObject), cancelPreviousPerformRequestsWithTarget:self, selector:sel!(pauseGestures:), object:std::ptr::null::<AnyObject>()]
        };
        let tap = self.ivars().tap.borrow_mut().take();
        if let Some(tap) = tap {
            diagnostics::record("trackpad.stop", tap.report());
            drop(tap);
        }
        self.update_gesture_label();
    }
    fn update_shortcut_label(&self) {
        if let Some(item) = self.ivars().hotkey_status.get() {
            let report = self.ivars().hotkeys.borrow();
            let conflict = report
                .as_ref()
                .is_some_and(|h| h.report.contains("indisponible") || h.report.contains("refusée"));
            item.setTitle(&NSString::from_str(&format!(
                "Raccourcis : {} — reçus {} fois (diagnostic)",
                if conflict {
                    "conflit / erreur"
                } else if self.ivars().personal_enabled.get() {
                    "enregistrés"
                } else {
                    "personnels suspendus"
                },
                self.ivars().hotkey_count.get()
            )));
        }
    }
    fn register_hotkeys(&self) {
        self.ivars().hotkeys.borrow_mut().take(); // unregister before reacquiring
        let keys = Hotkeys::register(self.ivars().personal_enabled.get());
        diagnostics::record("shortcut.registration", &keys.report);
        self.ivars().hotkeys.replace(Some(keys));
        self.update_shortcut_label();
        if let Some(item) = self.ivars().personal_item.get() {
            item.setTitle(&NSString::from_str(
                if self.ivars().personal_enabled.get() {
                    "Suspendre les raccourcis personnels"
                } else {
                    "Activer les raccourcis personnels"
                },
            ));
        }
    }
    fn cancel_open(&self) {
        if self.ivars().pending.borrow_mut().take().is_some() {
            diagnostics::record("application.open.cancelled", "superseded_or_shutdown=true");
        }
        let _: () = unsafe {
            msg_send![objc2::class!(NSObject), cancelPreviousPerformRequestsWithTarget:self, selector:sel!(openPoll:), object:std::ptr::null::<AnyObject>()]
        };
    }
    fn schedule_open_poll(&self) {
        let _: () = unsafe {
            msg_send![self, performSelector:sel!(openPoll:), withObject:std::ptr::null::<AnyObject>(), afterDelay:0.2f64]
        };
    }
    fn execute(&self, action: Action) {
        if self.ivars().picking.get() {
            return;
        }
        self.cancel_open();
        self.cancel_gesture_dispatch();
        if matches!(
            action,
            Action::Maximize | Action::PairLeft | Action::PairRight
        ) && !self.window_permission()
        {
            return;
        }
        let started = Instant::now();
        diagnostics::record("action.start", format!("action={action:?}"));
        let result = match action {
            Action::Teams | Action::Browser => {
                let bundle = if action == Action::Teams {
                    applications::teams_bundle()
                } else {
                    Ok(applications::CHROME)
                };
                bundle.and_then(applications::open).map(|result| {
                    self.ivars().pending.replace(Some(PendingOpen {
                        action,
                        result,
                        pid: None,
                        started,
                    }));
                    self.schedule_open_poll();
                    "Activation demandée…".to_owned()
                })
            }
            Action::ChatGpt => {
                // Apple Events may run a consent modal loop. Ignore reentrant
                // hotkeys/gestures until this single explicit action completes.
                self.ivars().picking.set(true);
                let result = applications::chatgpt(self.mtm());
                self.ivars().picking.set(false);
                result
            }
            Action::Maximize => windows::maximize(self.mtm(), None),
            Action::PairLeft | Action::PairRight => self.pick_pair(action == Action::PairLeft),
            Action::Test => return,
        };
        diagnostics::record(
            "action.dispatch",
            format!(
                "action={action:?} success={} elapsed_ms={}",
                result.is_ok(),
                started.elapsed().as_millis()
            ),
        );
        self.status(&result.unwrap_or_else(|e| e));
    }
    fn poll_open(&self) {
        let Some(mut pending) = self.ivars().pending.borrow_mut().take() else {
            return;
        };
        if pending.pid.is_none() {
            match pending.result.try_recv() {
                Ok(Ok(pid)) => pending.pid = Some(pid),
                Ok(Err(code)) => {
                    self.status(&format!("Ouverture refusée par macOS (code {code})."));
                    return;
                }
                Err(TryRecvError::Disconnected) => {
                    self.status("Réponse d'ouverture interrompue.");
                    return;
                }
                Err(TryRecvError::Empty) => {}
            }
        }
        let result = if let Some(pid) = pending.pid {
            match pending.action {
                Action::Teams => Some(Ok(
                    "Teams activé (aucune nouvelle fenêtre demandée).".to_owned()
                )),
                _ => {
                    let app = NSRunningApplication::runningApplicationWithProcessIdentifier(pid);
                    if let Some(app) = app {
                        // Do not reposition an app after the user has switched elsewhere.
                        if !app.isActive() {
                            Some(Err(
                                "Chrome n'est plus actif : maximisation annulée. Réessayez ⌥B."
                                    .into(),
                            ))
                        } else {
                            if !windows::trusted() || windows::application_window_ready(&app) {
                                if self.window_permission() {
                                    Some(windows::maximize(self.mtm(), Some(&app)))
                                } else {
                                    return;
                                }
                            } else {
                                None
                            } // wait for a window, never repeat geometry writes
                        }
                    } else {
                        Some(Err("Chrome a quitté avant le positionnement.".into()))
                    }
                }
            }
        } else {
            None
        };
        if let Some(result) = result {
            diagnostics::record(
                "action.complete",
                format!(
                    "action={:?} success={} elapsed_ms={}",
                    pending.action,
                    result.is_ok(),
                    pending.started.elapsed().as_millis()
                ),
            );
            self.status(&result.unwrap_or_else(|e| e));
        } else if pending.started.elapsed() >= Duration::from_secs(3) {
            self.status(
                "Ouverture non confirmée après 3 secondes. Réessayez une fois l'application prête.",
            );
        } else {
            self.ivars().pending.replace(Some(pending));
            self.schedule_open_poll();
        }
    }
    fn pick_pair(&self, first_left: bool) -> Result<String, String> {
        let (display, mut choices, partial) = windows::pair_candidates(self.mtm())?;
        self.ivars().picking.set(true);
        #[allow(deprecated)]
        NSApplication::sharedApplication(self.mtm()).activateIgnoringOtherApps(true);
        let result = (|| {
            let side = if first_left { "gauche" } else { "droite" };
            let Some(first) = choose_window(self.mtm(), side, &choices, partial) else {
                return Ok("Partage annulé ; aucune fenêtre déplacée.".into());
            };
            let first = choices.remove(first);
            let side = if first_left { "droite" } else { "gauche" };
            let Some(second) = choose_window(self.mtm(), side, &choices, partial) else {
                return Ok("Partage annulé ; aucune fenêtre déplacée.".into());
            };
            let second = &choices[second];
            if first_left {
                windows::arrange_pair(display, &first, second, false)
            } else {
                windows::arrange_pair(display, second, &first, true)
            }
        })();
        self.ivars().picking.set(false);
        result
    }
    fn window_permission(&self) -> bool {
        if self.refresh_permission() {
            return true;
        }
        self.status("Placement bloqué : Accessibilité non autorisée pour cette copie de Kigi.");
        if !self.ivars().permission_notice.replace(true) {
            let bundle = NSBundle::mainBundle().bundlePath();
            alert(self.mtm(), "Autorisation Accessibilité nécessaire", &format!(
                "Le raccourci est reçu, mais macOS refuse le contrôle des fenêtres.\n\nCopie active : {bundle}\n\nAutorisez cette copie dans Réglages Système > Confidentialité et sécurité > Accessibilité. Si l'ancienne entrée cochée reste sans effet après remplacement, retirez-la puis ajoutez cette copie et relancez Kigi.\n\nL'ouverture d'une application ou de ChatGPT ne prouve pas que le placement est autorisé."));
        }
        false
    }
    fn refresh_permission(&self) -> bool {
        let trusted = windows::trusted();
        let previous = self.ivars().last_permission.replace(Some(trusted));
        if previous != Some(trusted) {
            diagnostics::record(
                "permission.ax.changed",
                format!("previous={previous:?} current={trusted}"),
            );
            if trusted {
                self.ivars().permission_notice.set(false);
                self.cancel_permission_poll();
                self.status("Accessibilité reconnue pour le processus actif.");
            }
        }
        if let Some(item) = self.ivars().permission_item.get() {
            let label = if trusted {
                "Accessibilité : autorisée — vérifier"
            } else if self.ivars().permission_flow.borrow().requested {
                "Accessibilité : non reconnue — revérifier sans demander"
            } else {
                "Accessibilité : non autorisée — demander"
            };
            item.setTitle(&NSString::from_str(label));
        }
        trusted
    }
    fn schedule_permission_poll(&self) {
        self.cancel_permission_poll();
        let _: () = unsafe {
            msg_send![self, performSelector:sel!(permissionPoll:), withObject:std::ptr::null::<AnyObject>(), afterDelay:2.0f64]
        };
    }
    fn cancel_permission_poll(&self) {
        let _: () = unsafe {
            msg_send![objc2::class!(NSObject), cancelPreviousPerformRequestsWithTarget:self, selector:sel!(permissionPoll:), object:std::ptr::null::<AnyObject>()]
        };
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
            .hotkeys
            .borrow()
            .as_ref()
            .map(|h| h.report.clone())
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
        let probe = windows::permission_probe();
        let peers = other_instances().len();
        let translocated = executable.contains("/AppTranslocation/");
        let os = NSProcessInfo::processInfo().operatingSystemVersionString();
        let exported_at = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        diagnostics::report(&format!("macOS={os}\narchitecture={}\nbundle=io.kigi.windows\npid={}\nother_instances={peers}\napp_translocated={translocated}\nexecutable={executable}\nexported_at_unix_utc={exported_at}\naccessibility={}\n{shortcut}\nlast_result={result}\ntrackpad={trackpad}\npermission_probe={probe}",
            std::env::consts::ARCH, std::process::id(), windows::trusted()))
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
fn choose_window(
    mtm: MainThreadMarker,
    side: &str,
    choices: &[windows::Choice],
    partial: bool,
) -> Option<usize> {
    let dialog = NSAlert::new(mtm);
    dialog.setMessageText(&NSString::from_str(&format!(
        "Choisir la fenêtre de {side}"
    )));
    dialog.setInformativeText(&NSString::from_str(if partial {
        "Liste partielle : certaines applications n'ont pas répondu à temps. Annulez puis réessayez si une fenêtre manque." } else {
        "Sélectionnez une fenêtre. Le placement sera appliqué après les deux choix. Échap annule." }));
    let picker = NSPopUpButton::initWithFrame_pullsDown(
        NSPopUpButton::alloc(mtm),
        NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(520.0, 28.0)),
        false,
    );
    for (i, choice) in choices.iter().enumerate() {
        // Prefix keeps entries distinct even when two windows have identical titles.
        picker.addItemWithTitle(&NSString::from_str(&format!("{}. {}", i + 1, choice.label)));
    }
    dialog.setAccessoryView(Some(&picker));
    dialog.addButtonWithTitle(&NSString::from_str("Choisir"));
    dialog.addButtonWithTitle(&NSString::from_str("Annuler"));
    if dialog.runModal() != NSAlertFirstButtonReturn {
        return None;
    }
    usize::try_from(picker.indexOfSelectedItem())
        .ok()
        .filter(|i| *i < choices.len())
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
/// Called from the main-loop CG tap. Only store one small event and defer UI/AX
/// until the callback has returned. Disabled/menu/busy states never queue work.
pub fn queue_gesture(edge: Edge) {
    CONTROLLER.with(|slot| {
        if let Some(controller) = slot.borrow().as_ref() {
            if controller.ivars().picking.get() || controller.ivars().menu_open.get() { return; }
            if controller.ivars().queued_gesture.replace(Some(edge)).is_none() {
                let _: () = unsafe {
                    msg_send![&**controller, performSelector:sel!(gestureAction:), withObject:std::ptr::null::<AnyObject>(), afterDelay:0.0f64]
                };
            }
        }
    });
}
pub fn queue_tap_interrupted() {
    CONTROLLER.with(|slot| {
        if let Some(controller) = slot.borrow().as_ref() {
            let _: () = unsafe {
                msg_send![&**controller, performSelector:sel!(pauseGestures:), withObject:std::ptr::null::<AnyObject>(), afterDelay:0.0f64]
            };
        }
    });
}
pub fn schedule_action(action: Action) {
    CONTROLLER.with(|slot| {
        if let Some(controller) = slot.borrow().as_ref() {
            if controller.ivars().picking.get() { return; }
            let count = controller.ivars().hotkey_count.get() + 1;
            controller.ivars().hotkey_count.set(count);
            controller.cancel_gesture_dispatch();
            diagnostics::record("shortcut.received", format!("action={action:?} count={count}"));
            controller.update_shortcut_label();
            let selector = Sel::register(std::ffi::CString::new(action.selector()).unwrap().as_c_str());
            // Carbon callback only queues; never perform AX or modal UI here.
            let _: () = unsafe { msg_send![&**controller, performSelectorOnMainThread:selector, withObject:std::ptr::null::<AnyObject>(), waitUntilDone:false] };
        }
    });
}
fn other_instances() -> Vec<Retained<NSRunningApplication>> {
    NSRunningApplication::runningApplicationsWithBundleIdentifier(ns_string!("io.kigi.windows"))
        .iter()
        .filter(|a| a.processIdentifier() > 0 && a.processIdentifier() != std::process::id() as i32)
        .collect()
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
        let peers = other_instances();
        if !peers.is_empty() {
            let copies = peers
                .iter()
                .map(|p| {
                    format!(
                        "PID {} : {}",
                        p.processIdentifier(),
                        p.bundleURL()
                            .and_then(|u| u.path())
                            .map(|p| p.to_string())
                            .unwrap_or_else(|| "chemin inconnu".into())
                    )
                })
                .collect::<Vec<_>>()
                .join("\n");
            alert(mtm, "Une autre copie de Kigi est déjà lancée", &format!("Quittez l'autre copie depuis son menu Kigi avant de lancer cette version. Cette nouvelle copie va se fermer sans modifier l'autre.\n\n{copies}"));
            return;
        }
        let controller = Controller::new(mtm);
        let status_bar = NSStatusBar::systemStatusBar();
        let status_item = status_bar.statusItemWithLength(-1.0);
        if let Some(button) = status_item.button(mtm) {
            button.setTitle(ns_string!("Kigi"));
        }
        let menu = NSMenu::new(mtm);
        let status = NSMenuItem::new(mtm);
        status.setTitle(ns_string!(
            "Prêt — gestes activables à la demande depuis le menu."
        ));
        status.setEnabled(false);
        menu.addItem(&status);
        controller.ivars().status.set(status).ok();
        let shortcut = menu_action(
            &menu,
            &controller,
            "Raccourcis : enregistrement en cours",
            sel!(shortcutDiagnostics:),
        );
        controller.ivars().hotkey_status.set(shortcut).ok();
        let permission = menu_action(
            &menu,
            &controller,
            "Accessibilité : à vérifier",
            sel!(permission:),
        );
        controller.ivars().permission_item.set(permission).ok();
        menu_action(
            &menu,
            &controller,
            "Diagnostic Accessibilité",
            sel!(permissionDiagnostics:),
        );
        menu_action(
            &menu,
            &controller,
            "Afficher la copie active dans Finder",
            sel!(revealApp:),
        );
        menu.setDelegate(Some(ProtocolObject::from_ref(&*controller)));
        controller.refresh_permission();
        menu.addItem(&NSMenuItem::separatorItem(mtm));
        for action in &Action::ALL[1..] {
            let selector = Sel::register(
                std::ffi::CString::new(action.selector())
                    .unwrap()
                    .as_c_str(),
            );
            menu_action(&menu, &controller, action.label(), selector);
        }
        let personal = menu_action(
            &menu,
            &controller,
            "Suspendre les raccourcis personnels",
            sel!(togglePersonal:),
        );
        controller.ivars().personal_item.set(personal).ok();
        menu_action(
            &menu,
            &controller,
            "Réenregistrer les raccourcis (après conflit)",
            sel!(retryHotkeys:),
        );
        menu.addItem(&NSMenuItem::separatorItem(mtm));
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
            "Espace Travail — Chrome 100 %",
            sel!(workspaceWork:),
        );
        menu_action(
            &menu,
            &controller,
            "Espace Réunion — Teams gauche / Chrome droite",
            sel!(workspaceMeeting:),
        );
        menu_action(
            &menu,
            &controller,
            "Restaurer la disposition avant réunion",
            sel!(workspaceRestore:),
        );
        let gestures = menu_action(
            &menu,
            &controller,
            "Activer les gestes de bord Teams / Chrome",
            sel!(toggleGestures:),
        );
        controller.ivars().gesture_item.set(gestures).ok();
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
        controller.ivars().personal_enabled.set(true);
        controller.register_hotkeys();
        let center = NSWorkspace::sharedWorkspace().notificationCenter();
        unsafe {
            for name in [
                NSWorkspaceWillSleepNotification,
                NSWorkspaceScreensDidSleepNotification,
                NSWorkspaceSessionDidResignActiveNotification,
            ] {
                center.addObserver_selector_name_object(
                    &controller,
                    sel!(pauseGestures:),
                    Some(name),
                    None,
                );
            }
        }
        app.run();
        unsafe {
            center.removeObserver(&controller);
        }
        let _: () = unsafe {
            msg_send![objc2::class!(NSObject), cancelPreviousPerformRequestsWithTarget:&*controller]
        };
        controller.ivars().hotkeys.borrow_mut().take();
        controller.cancel_open();
        CONTROLLER.with(|c| c.borrow_mut().take());
        controller.stop_tap();
        if let Some(window) = controller.ivars().local_window.borrow_mut().take() {
            window.close();
        }
        status_bar.removeStatusItem(&status_item);
    });
}

#[cfg(test)]
mod tests {
    use super::PermissionFlow;
    #[test]
    fn permission_is_requested_once_then_checks_stop_after_grant_or_budget() {
        let mut flow = PermissionFlow::default();
        assert!(flow.request_once(false));
        assert!(!flow.request_once(false));
        assert!(flow.next_poll(false));
        assert!(!flow.next_poll(true));
        assert!(!flow.next_poll(false));
        assert!(!flow.request_once(false));
        let mut denied = PermissionFlow::default();
        assert!(denied.request_once(false));
        for _ in 0..14 {
            assert!(denied.next_poll(false));
        }
        assert!(!denied.next_poll(false));
        assert!(!denied.next_poll(false));
        assert!(!PermissionFlow::default().request_once(true));
    }
}
