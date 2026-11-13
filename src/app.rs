use crate::{
    actions::{Action, ActivationGuard, ActivationState},
    applications, config, diagnostics,
    gestures::{Contact, Edge, EdgeProbe},
    hotkey::Hotkeys,
    mouse_scroll::MouseScrollTap,
    preferences::{Preferences, RuntimeSettings},
    shortcuts::{self, Binding, Command, Shortcuts},
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
    ns_string, MainThreadMarker, NSArray, NSBundle, NSNotification, NSObject, NSObjectProtocol,
    NSPoint, NSProcessInfo, NSRect, NSSize, NSString, NSUserDefaults,
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

struct PendingMaximize {
    request: windows::MaximizeRequest,
    started: Instant,
    activation_started: Option<Instant>,
}
struct PendingOpen {
    guard: ActivationGuard,
    raise_attempted: bool,
    raise_error: Option<String>,
    maximize: bool,
    result: Receiver<Result<i32, isize>>,
    pid: Option<i32>,
    started: Instant,
}

#[derive(Default)]
struct State {
    snapshot: RefCell<Option<Snapshot>>,
    layout_snapshot: RefCell<Option<windows::LayoutSnapshot>>,
    hotkeys: RefCell<Option<Hotkeys>>,
    personal_enabled: Cell<bool>,
    shortcuts: RefCell<Shortcuts>,
    shortcut_queue: Cell<Option<usize>>,
    captured: RefCell<Option<Option<(u32, u32, String)>>>,
    trackpad_last_samples: Cell<u64>,
    trackpad_empty_reported: Cell<bool>,
    trackpad_last_summary: RefCell<String>,
    pending: RefCell<Option<PendingOpen>>,
    pending_maximize: RefCell<Option<PendingMaximize>>,
    launch_validation: RefCell<String>,
    activation_check: Cell<bool>,
    activation_check_done: Cell<bool>,
    previous_launch_known: Cell<bool>,
    picking: Cell<bool>,
    menu_open: Cell<bool>,
    mouse_scroll_requested: Cell<bool>,
    mouse_scroll_filter: RefCell<Option<MouseScrollTap>>,
    mouse_scroll_last: RefCell<String>,
    queued_gesture: Cell<Option<Edge>>,
    status: RefCell<String>,
    hotkey_count: Cell<u64>,
    permission_notice: Cell<bool>,
    permission_flow: RefCell<PermissionFlow>,
    last_permission: Cell<Option<bool>>,
    local_window: RefCell<Option<Retained<NSWindow>>>,
    preferences: RefCell<Option<Preferences>>,
    preferences_anchor: Cell<Option<i32>>,
    catalog: RefCell<config::Catalog>,
    profiles_menu: OnceCell<Retained<NSMenu>>,
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
        #[unsafe(method(shortcutSelected:))]
        fn shortcut_selected(&self,_:Option<&AnyObject>) {
            self.cancel_shortcut_capture();
            let selected=self.ivars().preferences.borrow().as_ref().and_then(Preferences::selected_shortcut).unwrap_or(0);
            self.populate_shortcuts(selected);
            self.status("Modifiez la combinaison ou l’action, puis cliquez sur Enregistrer.");
        }
        #[unsafe(method(shortcutDraftChanged:))]
        fn shortcut_draft_changed(&self,_:Option<&AnyObject>) { self.status("Modifications non enregistrées. Cliquez sur Enregistrer pour les appliquer."); }
        #[unsafe(method(shortcutActionChanged:))]
        fn shortcut_action_changed(&self,_:Option<&AnyObject>) {
            if let Some(p)=self.ivars().preferences.borrow().as_ref(){p.update_shortcut_action();}
            self.status("Action modifiée. Cliquez sur Enregistrer pour l’appliquer.");
        }
        #[unsafe(method(recordShortcut:))]
        fn record_shortcut(&self,_:Option<&AnyObject>) {
            self.cancel_shortcut_capture();
            self.ivars().hotkeys.borrow_mut().take();
            let started=self.ivars().preferences.borrow().as_ref().is_some_and(|p|p.recorder.start());
            if started {
                self.status("Appuyez sur une combinaison avec ⌘, ⌥ ou ⌃. Échap annule. Les raccourcis Kigi sont suspendus pendant la saisie.");
                unsafe {let _:()=msg_send![self,performSelector:sel!(shortcutCaptureTimeout:),withObject:std::ptr::null::<AnyObject>(),afterDelay:15.0f64];}
            } else {self.register_hotkeys();}
        }
        #[unsafe(method(shortcutCaptureDone:))]
        fn capture_done(&self,_:Option<&AnyObject>) {
            let Some(result)=self.ivars().captured.borrow_mut().take() else{return;};
            self.cancel_capture_timer();
            let captured=result.is_some();
            if let Some((key,modifiers,name))=result {
                if let Some(p)=self.ivars().preferences.borrow().as_ref(){p.set_shortcut_key(key,modifiers,name);}
            } else {self.restore_shortcut_caption();}
            self.register_hotkeys();
            self.status(if captured {"Combinaison choisie. Cliquez sur Enregistrer pour l’appliquer."}else{"Saisie annulée. Vos raccourcis précédents restent actifs."});
        }
        #[unsafe(method(shortcutCaptureTimeout:))]
        fn capture_timeout(&self,_:Option<&AnyObject>) {self.cancel_shortcut_capture();}
        #[unsafe(method(chooseShortcutApp:))]
        fn choose_shortcut_app(&self,_:Option<&AnyObject>) {
            self.cancel_shortcut_capture();
            let panel=NSOpenPanel::openPanel(self.mtm());
            panel.setTitle(Some(ns_string!("Choisir une application")));
            panel.setPrompt(Some(ns_string!("Choisir")));
            panel.setCanChooseDirectories(false);panel.setCanChooseFiles(true);panel.setAllowsMultipleSelection(false);
            #[allow(deprecated)] panel.setAllowedFileTypes(Some(&NSArray::from_slice(&[ns_string!("app")])));
            self.ivars().picking.set(true);
            let result=panel.runModal();self.ivars().picking.set(false);
            if result!=NSModalResponseOK{return;}
            let result=(|| {
                let url=panel.URL().ok_or("Aucune application sélectionnée.")?;
                let bundle=NSBundle::bundleWithURL(&url).and_then(|b|b.bundleIdentifier()).ok_or("Cette application ne fournit pas d’identifiant valide.")?;
                if bundle.to_string()=="io.kigi.windows" {return Err("Choisissez une autre application que Kigi.");}
                let name=url.lastPathComponent().map(|s|s.to_string().trim_end_matches(".app").to_owned()).ok_or("Nom d’application absent.")?;
                if let Some(p)=self.ivars().preferences.borrow().as_ref(){p.set_shortcut_app(bundle.to_string(),name);}
                Ok(())
            })();
            match result {Ok(())=>self.status("Application choisie. Cliquez sur Enregistrer pour l’appliquer."),Err(e)=>self.status(e)}
        }
        #[unsafe(method(saveShortcut:))]
        fn save_shortcut(&self,_:Option<&AnyObject>) {
            self.cancel_shortcut_capture();
            let draft=self.ivars().preferences.borrow().as_ref().and_then(|p|p.selected_shortcut().map(|i|(i,p.shortcut_draft())));
            if let Some((index,draft))=draft {
                match draft {
                    Ok(binding)=>{let mut settings=self.ivars().shortcuts.borrow().clone();if let Some(slot)=settings.entries.get_mut(index){*slot=binding;self.persist_shortcuts(settings,index);}}
                    Err(e)=>self.status(&e),
                }
            }
        }
        #[unsafe(method(addShortcut:))]
        fn add_shortcut(&self,_:Option<&AnyObject>) {
            self.cancel_shortcut_capture();
            let mut settings=self.ivars().shortcuts.borrow().clone();let selected=settings.entries.len();
            if selected>=32 {self.status("Vous pouvez créer jusqu’à 32 raccourcis.");return;}
            settings.entries.push(Binding{enabled:false,key:45,modifiers:1<<11,key_label:"N".into(),command:Command::Maximize});
            self.persist_shortcuts(settings,selected);
        }
        #[unsafe(method(deleteShortcut:))]
        fn delete_shortcut(&self,_:Option<&AnyObject>) {
            self.cancel_shortcut_capture();
            let selected=self.ivars().preferences.borrow().as_ref().and_then(Preferences::selected_shortcut);
            if let Some(i)=selected {
                let mut settings=self.ivars().shortcuts.borrow().clone();
                if i<settings.entries.len(){settings.entries.remove(i);self.persist_shortcuts(settings,i.saturating_sub(1));}
            }
        }
        #[unsafe(method(resetShortcuts:))]
        fn reset_shortcuts(&self,_:Option<&AnyObject>) {
            self.cancel_shortcut_capture();
            let dialog=NSAlert::new(self.mtm());dialog.setMessageText(ns_string!("Rétablir les raccourcis par défaut ?"));
            dialog.setInformativeText(ns_string!("Vos combinaisons et applications personnalisées seront remplacées. Les espaces et les options souris sont conservés."));
            dialog.addButtonWithTitle(ns_string!("Rétablir"));dialog.addButtonWithTitle(ns_string!("Annuler"));
            self.ivars().picking.set(true);let result=dialog.runModal();self.ivars().picking.set(false);
            if result==NSAlertFirstButtonReturn{self.persist_shortcuts(Shortcuts::default(),1);}
        }
        #[unsafe(method(runShortcut:))]
        fn run_shortcut(&self,_:Option<&AnyObject>) {
            let Some(index)=self.ivars().shortcut_queue.take() else{return;};
            let command=self.ivars().shortcuts.borrow().entries.get(index).filter(|b|b.enabled).map(|b|b.command.clone());
            if self.ivars().personal_enabled.get(){if let Some(c)=command{self.execute_command(c);}}
        }
        #[unsafe(method(trackpadPoll:))]
        fn trackpad_poll(&self,_:Option<&AnyObject>) {self.poll_trackpad();}
        #[unsafe(method(activationCheck:))]
        fn activation_check(&self,_:Option<&AnyObject>) {
            let maximize=std::env::args().any(|a|a=="--check-maximize");
            let result=self.open_application("io.kigi.focus.target",maximize,Instant::now());
            if result.is_err(){self.finish_activation_check(false);}
        }
        #[unsafe(method(activationCheckTimeout:))]
        fn activation_check_timeout(&self,_:Option<&AnyObject>) {self.finish_activation_check(false);}
        #[unsafe(method(maximizePoll:))]
        fn maximize_poll(&self,_:Option<&AnyObject>){self.poll_maximize();}
        #[unsafe(method(performanceReport:))]
        fn performance_report(&self,_:Option<&AnyObject>){
            alert(self.mtm(),"Mesures de cette session",&format!("{}\n\n{}",diagnostics::performance_report(),self.ivars().launch_validation.borrow()));
        }
        #[unsafe(method(startupReport:))]
        fn startup_report(&self,_:Option<&AnyObject>){
            let visible=self.ivars().preferences.borrow().as_ref().is_some_and(|p|p.window.isVisible());
            println!("KIGI_STARTUP {}",serde_json::json!({"version":env!("CARGO_PKG_VERSION"),"architecture":std::env::consts::ARCH,"window_visible":visible,"rss_bytes":diagnostics::rss_bytes(),"peak_rss_bytes":diagnostics::peak_rss_bytes(),"accessibility":windows::trusted(),"input_access":Tap::input_trusted(),"previous_launch_known":self.ivars().previous_launch_known.get(),"hotkey_conflict":self.ivars().hotkeys.borrow().as_ref().is_some_and(|h|h.failed)}));
        }
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
            let result = self.ivars().status.borrow().clone();
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
                self.ivars().layout_snapshot.borrow_mut().take();
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
                &mut self.ivars().layout_snapshot.borrow_mut(),
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
            let result = if let Some(snapshot) = self.ivars().layout_snapshot.borrow_mut().as_mut() {
                snapshot.restore()
            } else {
                Ok("Aucune disposition temporaire à restaurer.".into())
            };
            if result.is_ok() {
                self.ivars().layout_snapshot.borrow_mut().take();
            }
            diagnostics::record("workspace.restore", format!("success={}", result.is_ok()));
            self.status(&result.unwrap_or_else(|e| e));
        }
        #[unsafe(method(saveLayout:))]
        fn save_layout(&self, _: Option<&AnyObject>) { self.open_profiles(); }
        #[unsafe(method(applyLayout:))]
        fn apply_layout(&self, _: Option<&AnyObject>) { self.open_profiles(); }
        #[unsafe(method(openProfiles:))]
        fn open_profiles_action(&self, _: Option<&AnyObject>) { self.open_profiles(); }
        #[unsafe(method(profileSelected:))]
        fn profile_selected(&self, _: Option<&AnyObject>) {
            if let Some(prefs) = self.ivars().preferences.borrow().as_ref() {
                prefs.show_selection(&self.ivars().catalog.borrow());
            }
        }
        #[unsafe(method(refreshProfiles:))]
        fn refresh_profiles_action(&self, _: Option<&AnyObject>) {
            if self.ivars().picking.get() { return; }
            let selected = self.ivars().preferences.borrow().as_ref().and_then(Preferences::selected_id);
            self.refresh_profiles(selected);
        }
        #[unsafe(method(captureProfile:))]
        fn capture_profile(&self, _: Option<&AnyObject>) {
            if self.ivars().picking.get() { return; }
            let values = self.ivars().preferences.borrow().as_ref()
                .map(|p| (p.selected_id(), p.name.stringValue().to_string().trim().to_owned()));
            if let Some((id,name)) = values {
                if let Err(e) = config::Catalog::valid_name(&name) { self.status(&e); return; }
                self.personal_layout(true, id, Some(name));
            }
        }
        #[unsafe(method(applySelectedProfile:))]
        fn apply_selected_profile(&self, _: Option<&AnyObject>) {
            let selected = self.ivars().preferences.borrow().as_ref().and_then(Preferences::selected_id);
            if let Some(id) = selected {
                if self.ivars().layout_snapshot.borrow().is_some() {
                    self.status("Restaurez la disposition active ou choisissez Travail avant d'en appliquer une autre."); return;
                }
                if self.edit_profile(true) { self.personal_layout(false, Some(id), None); }
            }
        }
        #[unsafe(method(applyNamedProfile:))]
        fn apply_named_profile(&self, sender: Option<&NSMenuItem>) {
            if let Some(id) = sender.and_then(|s| u32::try_from(s.tag()).ok()).filter(|id| *id != 0) {
                self.personal_layout(false, Some(id), None);
            }
        }
        #[unsafe(method(renameProfile:))]
        fn rename_profile(&self, _: Option<&AnyObject>) { self.edit_profile(false); }
        #[unsafe(method(saveProfilePositions:))]
        fn save_profile_positions(&self, _: Option<&AnyObject>) { self.edit_profile(true); }
        #[unsafe(method(duplicateProfile:))]
        fn duplicate_profile(&self, _: Option<&AnyObject>) {
            if self.ivars().picking.get() { return; }
            let selected = self.ivars().preferences.borrow().as_ref().and_then(Preferences::selected_id);
            let Some(id) = selected else { return; };
            let result = (|| {
                let (mut catalog, _) = self.read_catalog()?;
                let profile = catalog.profile(id)?;
                let base: String = profile.name.chars().take(48).collect();
                let layout = profile.layout();
                let name = (1..=33).map(|n| format!("{base} copie {n}"))
                    .find(|name| !catalog.profiles.iter().any(|p|p.name.to_lowercase()==name.to_lowercase()))
                    .ok_or("Aucun nom de copie disponible.")?;
                let id = catalog.upsert(None, &name, &layout)?;
                config::save(&config::path()?, &catalog)?;
                self.set_catalog(catalog, Some(id), true);
                Ok("Copie créée. Vous pouvez la renommer et modifier ses positions.".to_owned())
            })();
            diagnostics::record("profiles.duplicate", format!("success={}", result.is_ok()));
            self.status(&result.unwrap_or_else(|e: String| e));
        }
        #[unsafe(method(deleteProfile:))]
        fn delete_profile(&self, _: Option<&AnyObject>) {
            if self.ivars().picking.get() { return; }
            let selected = self.ivars().preferences.borrow().as_ref().and_then(Preferences::selected_id);
            let Some(id) = selected else { return; };
            let result = (|| {
                let (mut catalog, _) = self.read_catalog()?;
                let profile = catalog.profile(id)?;
                let dialog = NSAlert::new(self.mtm());
                dialog.setMessageText(&NSString::from_str("Supprimer cet espace ?"));
                dialog.setInformativeText(&NSString::from_str(&format!("« {} » sera retiré des espaces enregistrés. Les fenêtres ouvertes resteront en place.", profile.name)));
                dialog.addButtonWithTitle(&NSString::from_str("Supprimer"));
                dialog.addButtonWithTitle(&NSString::from_str("Annuler"));
                self.ivars().picking.set(true);
                let confirmed = dialog.runModal() == NSAlertFirstButtonReturn;
                self.ivars().picking.set(false);
                if !confirmed { return Ok("Suppression annulée.".to_owned()); }
                // Re-read after the modal in case the config was externally edited.
                catalog = self.read_catalog()?.0;
                catalog.remove(id)?;
                config::save(&config::path()?, &catalog)?;
                let next = catalog.profiles.first().map(|p|p.id);
                self.set_catalog(catalog, next, true);
                Ok("Espace supprimé. Les autres espaces sont conservés.".into())
            })();
            self.status(&result.unwrap_or_else(|e: String| e));
        }
        #[unsafe(method(revealConfig:))]
        fn reveal_config(&self, _: Option<&AnyObject>) {
            match config::path() {
                Ok(path) if path.exists() => {
                    let shown = NSWorkspace::sharedWorkspace().selectFile_inFileViewerRootedAtPath(
                        Some(&NSString::from_str(&path.to_string_lossy())), ns_string!(""));
                    if !shown { self.status("Impossible d'afficher le fichier dans Finder."); }
                },
                _ => self.status("Aucune configuration enregistrée. Créez un espace depuis Gérer les espaces de travail."),
            }
        }
        #[unsafe(method(openPoll:))]
        fn open_poll(&self, _: Option<&AnyObject>) { self.poll_open(); }
        #[unsafe(method(retryHotkeys:))]
        fn retry_hotkeys(&self, _: Option<&AnyObject>) { self.register_hotkeys(); }
        #[unsafe(method(togglePersonal:))]
        fn toggle_personal(&self, _: Option<&AnyObject>) {
            self.ivars().personal_enabled.set(!self.ivars().personal_enabled.get());
            NSUserDefaults::standardUserDefaults().setBool_forKey(self.ivars().personal_enabled.get(), ns_string!("personalHotkeysEnabled"));
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
            diagnostics::record("trackpad.local.open", "mode=edges width=0.10");
            let mtm = self.mtm();
            // New probe each opening; no stale finger identity across sessions.
            let window = unsafe {
                NSWindow::initWithContentRect_styleMask_backing_defer(NSWindow::alloc(mtm),
                    NSRect::new(NSPoint::new(0.0,0.0),NSSize::new(600.0,240.0)),
                    NSWindowStyleMask::Titled | NSWindowStyleMask::Closable,
                    NSBackingStoreType::Buffered, false)
            };
            unsafe { window.setReleasedWhenClosed(false) };
            window.setTitle(ns_string!("Test local — bords du trackpad à 10 %"));
            let view = TouchView::new(mtm);
            let text = NSTextField::labelWithString(ns_string!("Test local : gardez le pointeur dans cette fenêtre et levez les doigts.\nUn doigt depuis les 10 % extrêmes gauche ou droite vers l’intérieur sur 10 % de la largeur.\nLes compteurs confirment les contacts et les gestes ; aucune application n’est activée.\nCe test local ne valide pas la réception globale."), mtm);
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
                self.ivars().trackpad_last_summary.replace(format!("Test arrêté. {}",tap.summary()));
                diagnostics::record("trackpad.global.stop", &report);
                drop(tap);
                self.status("Écoute globale arrêtée.");
                alert(self.mtm(), "Bilan du trackpad", &report);
            } else {
                match Tap::start() {
                    Ok(tap) => {
                        diagnostics::record("trackpad.global.start", "passive=true actions=false");
                        self.ivars().tap.replace(Some(tap));
                        self.begin_trackpad_poll();
                        self.status("Diagnostic global démarré. Effectuez le geste horizontal à un doigt ; les compteurs apparaissent dans Gestes et souris.");
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
                    self.begin_trackpad_poll();
                    self.update_gesture_label();
                    diagnostics::record("gestures.start", "left=0.10 right=0.10 min_travel=0.10 direction=inward whole_height=true");
                    self.status("Test global démarré. Réception des contacts non confirmée ; effectuez un geste puis consultez le bilan.");
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
                self.status("Écoute trackpad arrêtée (veille/session inactive/interruption). Réactivez-la dans Gestes et souris.");
            }
        }
        #[unsafe(method(toggleMouseHorizontal:))]
        fn toggle_mouse_horizontal(&self, _: Option<&AnyObject>) {
            if self.ivars().mouse_scroll_requested.get() {
                self.ivars().mouse_scroll_requested.set(false);
                self.stop_mouse_scroll();
                NSUserDefaults::standardUserDefaults().setBool_forKey(false, ns_string!("disableMouseHorizontalScroll"));
                self.status("Défilement horizontal des souris rétabli.");
            } else {
                if !self.window_permission() { return; }
                match self.start_mouse_scroll() {
                    Ok(()) => {
                        self.ivars().mouse_scroll_requested.set(true);
                        NSUserDefaults::standardUserDefaults().setBool_forKey(true, ns_string!("disableMouseHorizontalScroll"));
                        self.status("Blocage horizontal souris activé. Le vertical est conservé ; testez aussi le trackpad.");
                    },
                    Err(e) => self.status(&e),
                }
            }
            self.update_mouse_scroll_label();
        }
        #[unsafe(method(mouseFilterInterrupted:))]
        fn mouse_filter_interrupted(&self, _: Option<&AnyObject>) {
            self.stop_mouse_scroll();
            self.status("Filtre souris interrompu par macOS. Désactivez puis réactivez l'option pour réessayer.");
        }
        #[unsafe(method(suspendInputs:))]
        fn suspend_inputs(&self, _: Option<&AnyObject>) {
            self.pause_gestures(sel!(pauseGestures:), None);
            self.stop_mouse_scroll();
        }
        #[unsafe(method(resumeMouseScroll:))]
        fn resume_mouse_scroll(&self, _: Option<&AnyObject>) {
            if self.ivars().mouse_scroll_requested.get() && !self.mouse_scroll_active() {
                if let Err(e) = self.start_mouse_scroll() { self.status(&e); }
                self.update_mouse_scroll_label();
            }
        }
        #[unsafe(method(workspaceContextChanged:))]
        fn context_changed(&self, _: &NSNotification) { self.remember_context(); }
        #[unsafe(method(quit:))]
        fn quit(&self, _: Option<&AnyObject>) {
            self.cancel_shortcut_capture();
            self.cancel_permission_poll();
            self.cancel_open();
            diagnostics::record("application.quit", "requested=true");
            self.stop_tap();
            self.stop_mouse_scroll();
            NSApplication::sharedApplication(self.mtm()).stop(None);
        }
    }
    unsafe impl NSApplicationDelegate for Controller {
        #[unsafe(method(applicationShouldHandleReopen:hasVisibleWindows:))]
        fn reopen(&self, _: &NSApplication, _: bool) -> bool { self.open_profiles(); true }
        #[unsafe(method(applicationShouldTerminateAfterLastWindowClosed:))]
        fn keep_running(&self, _: &NSApplication) -> bool { false }
        #[unsafe(method(applicationDidBecomeActive:))]
        fn became_active(&self, _: &NSNotification) { self.refresh_permission(); }
        #[unsafe(method(applicationDidResignActive:))]
        fn resigned_active(&self,_:&NSNotification) {self.cancel_shortcut_capture();}
    }
    unsafe impl NSWindowDelegate for Controller {
        #[unsafe(method(windowWillClose:))]
        fn settings_closed(&self,_:&NSNotification){self.cancel_shortcut_capture();}
    }
    unsafe impl NSMenuDelegate for Controller {
        #[unsafe(method(menuWillOpen:))]
        fn menu_will_open(&self, _: &NSMenu) {
            self.ivars().menu_open.set(true);
            self.cancel_gesture_dispatch();
            self.refresh_permission();
            self.update_gesture_label();
            self.update_mouse_scroll_label();
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
    fn sync_preferences(&self) {
        if let Some(prefs) = self.ivars().preferences.borrow().as_ref() {
            let registration = self
                .ivars()
                .hotkeys
                .borrow()
                .as_ref()
                .map(|h| h.report.clone())
                .unwrap_or_default();
            prefs.update_runtime(RuntimeSettings {
                personal_hotkeys: self.ivars().personal_enabled.get(),
                shortcuts_received: self.ivars().hotkey_count.get(),
                registration: &registration,
                gestures: self.gestures_active(),
                mouse_requested: self.ivars().mouse_scroll_requested.get(),
                mouse_active: self.mouse_scroll_active(),
                accessibility: windows::trusted(),
                input_access: Tap::input_trusted(),
                trackpad_report: &self
                    .ivars()
                    .tap
                    .borrow()
                    .as_ref()
                    .map(Tap::summary)
                    .unwrap_or_else(|| self.ivars().trackpad_last_summary.borrow().clone()),
            });
        }
    }
    fn populate_shortcuts(&self, index: usize) {
        if let Some(p) = self.ivars().preferences.borrow().as_ref() {
            p.populate_shortcuts(&self.ivars().shortcuts.borrow(), index);
        }
    }
    fn restore_shortcut_caption(&self) {
        if let Some(p) = self.ivars().preferences.borrow().as_ref() {
            p.restore_shortcut_caption();
        }
    }
    fn cancel_capture_timer(&self) {
        unsafe {
            let _: () = msg_send![objc2::class!(NSObject),cancelPreviousPerformRequestsWithTarget:self,selector:sel!(shortcutCaptureTimeout:),object:std::ptr::null::<AnyObject>()];
        }
    }
    fn cancel_shortcut_capture(&self) {
        self.cancel_capture_timer();
        if let Some(p) = self.ivars().preferences.borrow().as_ref() {
            p.recorder.cancel();
        }
        // The responder queues cancellation. Finish synchronously before starting
        // another editor operation, so an old cancellation cannot discard new input.
        if self.ivars().captured.borrow().is_some() {
            self.capture_done(sel!(shortcutCaptureDone:), None);
        }
    }
    fn persist_shortcuts(&self, settings: Shortcuts, selected: usize) {
        self.ivars().shortcut_queue.take();
        self.cancel_open();
        if let Err(e) = settings.validate() {
            self.status(&e);
            return;
        }
        self.ivars().hotkeys.borrow_mut().take();
        let keys = Hotkeys::register(self.ivars().personal_enabled.get(), &settings);
        if keys.failed {
            diagnostics::record("shortcuts.save.rejected", "registration_conflict=true");
            let report = keys.report.clone();
            drop(keys);
            self.register_hotkeys();
            alert(self.mtm(),"Raccourci indisponible",&format!("macOS a refusé une combinaison. Vos anciens réglages sont conservés.\n\n{report}"));
            self.status("Enregistrement annulé : conflit de raccourcis. Les anciens réglages sont conservés.");
            return;
        }
        let result = shortcuts_path()
            .and_then(|p| shortcuts::save(&p, &settings, &self.ivars().shortcuts.borrow()));
        match result {
            Ok(()) => {
                self.ivars().shortcuts.replace(settings);
                self.ivars().hotkeys.replace(Some(keys));
                self.populate_shortcuts(selected);
                self.sync_preferences();
                self.status(
                    "Raccourcis enregistrés. Les réglages seront conservés au prochain lancement.",
                );
            }
            Err(e) => {
                drop(keys);
                self.register_hotkeys();
                self.status(&e);
            }
        }
    }
    fn execute_command(&self, command: Command) {
        match command {
            Command::Application {
                bundle, maximize, ..
            } => {
                if self.ivars().picking.get() {
                    return;
                }
                self.cancel_open();
                self.cancel_gesture_dispatch();
                let result = if bundle == "com.microsoft.teams2" {
                    applications::teams_bundle().map(str::to_owned)
                } else {
                    Ok(bundle)
                }
                .and_then(|b| self.open_application(&b, maximize, Instant::now()));
                self.status(&result.unwrap_or_else(|e| e));
            }
            Command::ChatGpt => self.execute(Action::ChatGpt),
            Command::Maximize => self.execute(Action::Maximize),
            Command::PairLeft => self.execute(Action::PairLeft),
            Command::PairRight => self.execute(Action::PairRight),
            Command::Test => self.test_window(sel!(testWindow:), None),
        }
    }
    fn open_application(
        &self,
        bundle: &str,
        maximize: bool,
        started: Instant,
    ) -> Result<String, String> {
        let origin = NSWorkspace::sharedWorkspace()
            .frontmostApplication()
            .map(|a| a.processIdentifier());
        let fullscreen = NSApplication::sharedApplication(self.mtm())
            .currentSystemPresentationOptions()
            .contains(NSApplicationPresentationOptions::FullScreen);
        diagnostics::record(
            "application.activation.source",
            format!("pid={origin:?} native_fullscreen={fullscreen}"),
        );
        applications::open(bundle).map(|result| {
            self.ivars().pending.replace(Some(PendingOpen {
                guard: ActivationGuard::new(origin),
                raise_attempted: false,
                raise_error: None,
                maximize,
                result,
                pid: None,
                started,
            }));
            self.schedule_open_poll();
            "Activation demandée…".into()
        })
    }
    fn begin_trackpad_poll(&self) {
        unsafe {
            let _: () = msg_send![objc2::class!(NSObject),cancelPreviousPerformRequestsWithTarget:self,selector:sel!(trackpadPoll:),object:std::ptr::null::<AnyObject>()];
        }
        self.ivars().trackpad_last_samples.set(0);
        self.ivars().trackpad_empty_reported.set(false);
        self.poll_trackpad();
    }
    fn poll_trackpad(&self) {
        let data = self.ivars().tap.borrow().as_ref().map(|t| {
            (
                t.contacts(),
                t.elapsed_seconds(),
                t.summary(),
                t.report(),
                t.evidence(),
                t.active_actions(),
            )
        });
        let Some((samples, elapsed, summary, report, evidence, actions)) = data else {
            return;
        };
        self.ivars().trackpad_last_summary.replace(summary);
        if samples > 0 && self.ivars().trackpad_last_samples.replace(samples) == 0 {
            diagnostics::record("trackpad.contacts.received", "coordinates_recorded=false");
        }
        if elapsed >= 12 && !self.ivars().trackpad_empty_reported.replace(true) {
            diagnostics::record("trackpad.trial.complete", &report);
            if !actions || evidence.recognized == 0 {
                self.stop_tap();
                self.ivars()
                    .trackpad_last_summary
                    .replace(format!("Test arrêté. {}", evidence.diagnosis()));
                self.status(evidence.diagnosis());
                return;
            }
            self.status(evidence.diagnosis());
        }
        self.sync_preferences();
        unsafe {
            let _: () = msg_send![self,performSelector:sel!(trackpadPoll:),withObject:std::ptr::null::<AnyObject>(),afterDelay:1.0f64];
        }
    }
    fn remember_context(&self) {
        if let Some(pid) = NSWorkspace::sharedWorkspace()
            .frontmostApplication()
            .map(|a| a.processIdentifier())
            .filter(|pid| *pid != std::process::id() as i32)
        {
            self.ivars().preferences_anchor.set(Some(pid));
        }
    }
    fn read_catalog(&self) -> Result<(config::Catalog, bool), String> {
        Ok(config::load(&config::path()?)?.unwrap_or_else(|| (config::Catalog::default(), false)))
    }
    fn set_catalog(&self, catalog: config::Catalog, selected: Option<u32>, update_form: bool) {
        if let Some(menu) = self.ivars().profiles_menu.get() {
            menu.removeAllItems();
            if catalog.profiles.is_empty() {
                let item = NSMenuItem::new(self.mtm());
                item.setTitle(&NSString::from_str("Aucun espace enregistré"));
                item.setEnabled(false);
                menu.addItem(&item);
            } else {
                for profile in &catalog.profiles {
                    let item = menu_action(menu, self, &profile.name, sel!(applyNamedProfile:));
                    item.setTag(profile.id as isize);
                }
            }
        }
        if update_form {
            if let Some(prefs) = self.ivars().preferences.borrow().as_ref() {
                prefs.populate(&catalog, selected);
            }
        }
        self.ivars().catalog.replace(catalog);
    }
    fn refresh_profiles(&self, selected: Option<u32>) {
        match self.read_catalog() {
            Ok((catalog, backup)) => {
                self.set_catalog(catalog, selected, true);
                if backup {
                    self.status("Configuration principale illisible : sauvegarde valide affichée, fichier principal conservé.");
                }
            }
            Err(e) => self.status(&e),
        }
    }
    fn open_profiles(&self) {
        if self.ivars().picking.get() {
            return;
        }
        if let Some(pid) = NSWorkspace::sharedWorkspace()
            .frontmostApplication()
            .map(|a| a.processIdentifier())
            .filter(|pid| *pid != std::process::id() as i32)
        {
            self.ivars().preferences_anchor.set(Some(pid));
        }
        self.cancel_open();
        self.cancel_gesture_dispatch();
        if self.ivars().preferences.borrow().is_none() {
            self.ivars()
                .preferences
                .replace(Some(Preferences::new(self.mtm(), self)));
            if let Some(p) = self.ivars().preferences.borrow().as_ref() {
                p.window.setDelegate(Some(ProtocolObject::from_ref(self)));
            }
        }
        let visible = self
            .ivars()
            .preferences
            .borrow()
            .as_ref()
            .is_some_and(|p| p.window.isVisible() || p.window.isMiniaturized());
        if !visible {
            let selected = self
                .ivars()
                .preferences
                .borrow()
                .as_ref()
                .and_then(Preferences::selected_id)
                .or_else(|| self.ivars().catalog.borrow().profiles.first().map(|p| p.id));
            self.refresh_profiles(selected);
            self.populate_shortcuts(1);
        }
        self.refresh_permission();
        if let Some(prefs) = self.ivars().preferences.borrow().as_ref() {
            if prefs.window.isMiniaturized() {
                prefs.window.deminiaturize(None);
            }
            prefs.window.makeKeyAndOrderFront(None);
        }
        #[allow(deprecated)]
        NSApplication::sharedApplication(self.mtm()).activateIgnoringOtherApps(true);
    }
    fn edit_profile(&self, positions: bool) -> bool {
        if self.ivars().picking.get() {
            return false;
        }
        let data = self.ivars().preferences.borrow().as_ref().and_then(|p| {
            p.selected_id()
                .map(|id| (id, p.name.stringValue().to_string().trim().to_owned()))
        });
        let Some((id, name)) = data else {
            return false;
        };
        let result = (|| {
            let (mut catalog, _) = self.read_catalog()?;
            let previous = catalog.clone();
            let profile = catalog.profile(id)?;
            let layout = if positions {
                self.ivars()
                    .preferences
                    .borrow()
                    .as_ref()
                    .unwrap()
                    .edited_layout(profile)
            } else {
                profile.layout()
            };
            catalog.upsert(Some(id), &name, &layout)?;
            if catalog != previous {
                config::save(&config::path()?, &catalog)?;
            }
            self.set_catalog(catalog, Some(id), true);
            Ok(if positions {
                "Positions enregistrées. Cliquez sur Appliquer pour les utiliser."
            } else {
                "Espace renommé."
            }
            .to_owned())
        })();
        diagnostics::record(
            "profiles.edit",
            format!("id={id} positions={positions} success={}", result.is_ok()),
        );
        let success = result.is_ok();
        self.status(&result.unwrap_or_else(|e: String| e));
        success
    }
    fn mouse_scroll_active(&self) -> bool {
        self.ivars()
            .mouse_scroll_filter
            .borrow()
            .as_ref()
            .is_some_and(MouseScrollTap::active)
    }
    fn update_mouse_scroll_label(&self) {
        self.sync_preferences();
    }
    fn stop_mouse_scroll(&self) {
        let _: () = unsafe {
            msg_send![objc2::class!(NSObject), cancelPreviousPerformRequestsWithTarget:self, selector:sel!(mouseFilterInterrupted:), object:std::ptr::null::<AnyObject>()]
        };
        let filter = self.ivars().mouse_scroll_filter.borrow_mut().take();
        if let Some(filter) = filter {
            let report = filter.report();
            diagnostics::record("mouse_scroll.stop", &report);
            self.ivars().mouse_scroll_last.replace(report);
            drop(filter);
        }
        self.update_mouse_scroll_label();
    }
    fn start_mouse_scroll(&self) -> Result<(), String> {
        if self.mouse_scroll_active() {
            return Ok(());
        }
        self.stop_mouse_scroll();
        if !windows::trusted() {
            return Err("Filtre souris inactif : autorisez l'Accessibilité de la copie actuelle de Kigi, puis réactivez l'option.".into());
        }
        let filter = MouseScrollTap::start(self.mtm())?;
        diagnostics::record(
            "mouse_scroll.start",
            "horizontal_only=true classification=public_phases scope=all_apps no_vendor_id=true",
        );
        self.ivars().mouse_scroll_filter.replace(Some(filter));
        Ok(())
    }
    fn gestures_active(&self) -> bool {
        self.ivars()
            .tap
            .borrow()
            .as_ref()
            .is_some_and(Tap::active_actions)
    }
    fn update_gesture_label(&self) {
        self.sync_preferences();
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
            msg_send![objc2::class!(NSObject), cancelPreviousPerformRequestsWithTarget:self, selector:sel!(trackpadPoll:), object:std::ptr::null::<AnyObject>()]
        };
        let _: () = unsafe {
            msg_send![objc2::class!(NSObject), cancelPreviousPerformRequestsWithTarget:self, selector:sel!(pauseGestures:), object:std::ptr::null::<AnyObject>()]
        };
        let tap = self.ivars().tap.borrow_mut().take();
        if let Some(tap) = tap {
            self.ivars()
                .trackpad_last_summary
                .replace(format!("Test arrêté. {}", tap.summary()));
            diagnostics::record("trackpad.stop", tap.report());
            drop(tap);
        }
        self.update_gesture_label();
    }
    fn update_shortcut_label(&self) {
        self.sync_preferences();
    }
    fn register_hotkeys(&self) {
        self.ivars().hotkeys.borrow_mut().take(); // unregister before reacquiring
        let keys = Hotkeys::register(
            self.ivars().personal_enabled.get(),
            &self.ivars().shortcuts.borrow(),
        );
        diagnostics::record("shortcut.registration", &keys.report);
        self.ivars().hotkeys.replace(Some(keys));
        self.update_shortcut_label();
    }
    fn cancel_open(&self) {
        if self.ivars().pending.borrow_mut().take().is_some() {
            diagnostics::record("application.open.cancelled", "superseded_or_shutdown=true");
        }
        if self.ivars().pending_maximize.borrow_mut().take().is_some() {
            diagnostics::record("window.maximize.cancelled", "superseded_or_shutdown=true");
        }
        unsafe {
            let _: () = msg_send![objc2::class!(NSObject),cancelPreviousPerformRequestsWithTarget:self,selector:sel!(maximizePoll:),object:std::ptr::null::<AnyObject>()];
        }
        let _: () = unsafe {
            msg_send![objc2::class!(NSObject), cancelPreviousPerformRequestsWithTarget:self, selector:sel!(openPoll:), object:std::ptr::null::<AnyObject>()]
        };
    }
    fn finish_activation_check(&self, success: bool) {
        if !self.ivars().activation_check.get() || self.ivars().activation_check_done.replace(true)
        {
            return;
        }
        println!(
            "KIGI_FOCUS_CHECK {}",
            serde_json::json!({"success":success,"frontmost_pid":NSWorkspace::sharedWorkspace().frontmostApplication().map(|a|a.processIdentifier()),"accessibility":windows::trusted(),"report":self.log_report()})
        );
        NSApplication::sharedApplication(self.mtm()).stop(None);
    }
    fn remember_launch(&self) {
        let defaults = NSUserDefaults::standardUserDefaults();
        let previous = defaults.stringForKey(ns_string!("validationLastVersion"));
        self.ivars().previous_launch_known.set(previous.is_some());
        let text = if let Some(version) = previous {
            format!("Relance : version précédente {version}, actuelle {} ; Accessibilité précédente={}, actuelle={} ; surveillance précédente={}, actuelle={}. Une mise à jour ad hoc peut nécessiter une nouvelle autorisation.",env!("CARGO_PKG_VERSION"),defaults.boolForKey(ns_string!("validationLastAX")),windows::trusted(),defaults.boolForKey(ns_string!("validationLastInput")),Tap::input_trusted())
        } else {
            "Relance : premier relevé disponible. Quittez puis relancez cette même copie pour comparer les permissions.".into()
        };
        self.ivars().launch_validation.replace(text.clone());
        diagnostics::record("application.launch.validation", text);
        unsafe {
            defaults.setObject_forKey(
                Some(&NSString::from_str(env!("CARGO_PKG_VERSION"))),
                ns_string!("validationLastVersion"),
            );
        }
        self.remember_permissions();
    }
    fn remember_permissions(&self) {
        let defaults = NSUserDefaults::standardUserDefaults();
        defaults.setBool_forKey(windows::trusted(), ns_string!("validationLastAX"));
        defaults.setBool_forKey(Tap::input_trusted(), ns_string!("validationLastInput"));
    }
    fn begin_maximize(
        &self,
        app: Option<&NSRunningApplication>,
        activation_started: Option<Instant>,
    ) -> Result<String, String> {
        let started = Instant::now();
        let request = match windows::MaximizeRequest::start(self.mtm(), app) {
            Ok(request) => request,
            Err(e) => {
                diagnostics::timing(diagnostics::Metric::Maximize, started.elapsed(), false);
                if let Some(t) = activation_started {
                    diagnostics::timing(diagnostics::Metric::Activation, t.elapsed(), false);
                }
                return Err(e);
            }
        };
        self.ivars().pending_maximize.replace(Some(PendingMaximize {
            request,
            started,
            activation_started,
        }));
        match self.advance_maximize() {
            Some(result) => result,
            None => {
                self.schedule_maximize_poll();
                Ok("Transition de fenêtre en cours. Le placement suivra sans bloquer Kigi.".into())
            }
        }
    }
    fn schedule_maximize_poll(&self) {
        unsafe {
            let _: () = msg_send![self,performSelector:sel!(maximizePoll:),withObject:std::ptr::null::<AnyObject>(),afterDelay:0.1f64];
        }
    }
    fn advance_maximize(&self) -> Option<Result<String, String>> {
        let mut pending = self.ivars().pending_maximize.borrow_mut().take()?;
        let result = match pending.request.poll(self.mtm()) {
            Ok(None) => {
                self.ivars().pending_maximize.replace(Some(pending));
                return None;
            }
            Ok(Some(text)) => Ok(text),
            Err(e) => Err(e),
        };
        diagnostics::timing(
            diagnostics::Metric::Maximize,
            pending.started.elapsed(),
            result.is_ok(),
        );
        if let Some(t) = pending.activation_started {
            diagnostics::timing(diagnostics::Metric::Activation, t.elapsed(), result.is_ok());
        }
        diagnostics::record(
            "window.maximize.complete",
            format!(
                "success={} elapsed_ms={}",
                result.is_ok(),
                pending.started.elapsed().as_millis()
            ),
        );
        if pending.activation_started.is_some() {
            diagnostics::record(
                "window.maximize.result",
                result.as_ref().map(String::as_str).unwrap_or_else(|e| e),
            );
            self.finish_activation_check(result.is_ok());
        }
        Some(result)
    }
    fn poll_maximize(&self) {
        if let Some(result) = self.advance_maximize() {
            self.status(&result.unwrap_or_else(|e| e));
        } else if self.ivars().pending_maximize.borrow().is_some() {
            self.schedule_maximize_poll();
        }
    }
    fn schedule_open_poll(&self) {
        let _: () = unsafe {
            msg_send![self, performSelector:sel!(openPoll:), withObject:std::ptr::null::<AnyObject>(), afterDelay:0.1f64]
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
                    applications::teams_bundle().map(str::to_owned)
                } else {
                    Ok(applications::CHROME.to_owned())
                };
                bundle.and_then(|b| self.open_application(&b, action == Action::Browser, started))
            }
            Action::ChatGpt => {
                // Apple Events may run a consent modal loop. Ignore reentrant
                // hotkeys/gestures until this single explicit action completes.
                self.ivars().picking.set(true);
                let result = applications::chatgpt(self.mtm());
                self.ivars().picking.set(false);
                result
            }
            Action::Maximize => self.begin_maximize(None, None),
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
                    diagnostics::timing(
                        diagnostics::Metric::Activation,
                        pending.started.elapsed(),
                        false,
                    );
                    self.finish_activation_check(false);
                    self.status(&format!("Ouverture refusée par macOS (code {code})."));
                    return;
                }
                Err(TryRecvError::Disconnected) => {
                    diagnostics::timing(
                        diagnostics::Metric::Activation,
                        pending.started.elapsed(),
                        false,
                    );
                    self.finish_activation_check(false);
                    self.status("Réponse d'ouverture interrompue.");
                    return;
                }
                Err(TryRecvError::Empty) => {}
            }
        }
        let result = if let Some(pid) = pending.pid {
            let front = NSWorkspace::sharedWorkspace()
                .frontmostApplication()
                .map(|a| a.processIdentifier());
            let state = pending.guard.observe(front, pid, std::process::id() as i32);
            if state == ActivationState::Cancelled {
                Some(Err(
                    "Activation annulée : vous avez changé d’application.".into()
                ))
            } else if let Some(app) =
                NSRunningApplication::runningApplicationWithProcessIdentifier(pid)
            {
                if app.isTerminated() {
                    Some(Err("L’application a quitté pendant l’activation.".into()))
                } else {
                    let trusted = windows::trusted();
                    if trusted
                        && !pending.raise_attempted
                        && windows::application_window_ready(&app)
                    {
                        pending.raise_attempted = true;
                        if let Err(e) = windows::raise_application_window(&app)
                            .and_then(|_| applications::activate(&app))
                        {
                            pending.raise_error = Some(e);
                        }
                    }
                    if state == ActivationState::Active && app.isActive() {
                        if let Some(error) = pending.raise_error.take() {
                            Some(Err(format!(
                                "Application active, mais fenêtre non relevée : {error}"
                            )))
                        } else if trusted && !pending.raise_attempted {
                            None
                        } else if !pending.maximize {
                            Some(Ok(if trusted {"Application au premier plan ; fenêtre existante relevée."}else{"Application au premier plan ; fenêtre non vérifiée sans Accessibilité."}.to_owned()))
                        } else if self.window_permission() {
                            let result = self.begin_maximize(Some(&app), Some(pending.started));
                            if result.is_err() {
                                self.finish_activation_check(false);
                            }
                            self.status(&result.unwrap_or_else(|e| e));
                            return;
                        } else {
                            diagnostics::timing(
                                diagnostics::Metric::Activation,
                                pending.started.elapsed(),
                                false,
                            );
                            self.finish_activation_check(false);
                            return;
                        }
                    } else {
                        None
                    }
                }
            } else {
                Some(Err("Le processus cible n’est plus disponible.".into()))
            }
        } else {
            None
        };
        if let Some(result) = result {
            diagnostics::record(
                "application.activation.result",
                result.as_ref().map(String::as_str).unwrap_or_else(|e| e),
            );
            self.finish_activation_check(result.is_ok());
            diagnostics::timing(
                diagnostics::Metric::Activation,
                pending.started.elapsed(),
                result.is_ok(),
            );
            diagnostics::record(
                "action.complete",
                format!(
                    "application=true maximize={} success={} elapsed_ms={}",
                    pending.maximize,
                    result.is_ok(),
                    pending.started.elapsed().as_millis()
                ),
            );
            self.status(&result.unwrap_or_else(|e| e));
        } else if pending.started.elapsed() >= Duration::from_secs(4) {
            self.finish_activation_check(false);
            diagnostics::timing(
                diagnostics::Metric::Activation,
                pending.started.elapsed(),
                false,
            );
            self.status(
                "Premier plan non confirmé après 4 secondes. Aucune fenêtre de secours demandée.",
            );
        } else {
            self.ivars().pending.replace(Some(pending));
            self.schedule_open_poll();
        }
    }
    fn personal_layout(&self, save: bool, profile_id: Option<u32>, profile_name: Option<String>) {
        if self.ivars().picking.get() || !self.window_permission() {
            return;
        }
        if !save && self.ivars().layout_snapshot.borrow().is_some() {
            self.status("Restaurez la disposition active ou choisissez Travail avant d'en appliquer une autre.");
            return;
        }
        self.cancel_open();
        self.cancel_gesture_dispatch();
        // Resolve all windows before any AX write. Titles stay in the chooser.
        let result = (|| {
            let path = config::path()?;
            let (catalog, backup) = self.read_catalog()?;
            let saved = if save {
                let name = profile_name.as_deref().ok_or("Nom de l'espace absent.")?;
                config::Catalog::valid_name(name)?;
                if let Some(id) = profile_id {
                    catalog.profile(id)?;
                } else if catalog.profiles.len() >= 32 {
                    return Err("La limite de 32 espaces est atteinte.".into());
                }
                if catalog.profiles.iter().any(|p| {
                    Some(p.id) != profile_id && p.name.to_lowercase() == name.to_lowercase()
                }) {
                    return Err("Ce nom existe déjà. Choisissez un autre nom.".into());
                }
                None
            } else {
                Some((
                    catalog
                        .profile(profile_id.ok_or("Choisissez un espace enregistré.")?)?
                        .layout(),
                    backup,
                ))
            };
            let prior_pid = NSWorkspace::sharedWorkspace()
                .frontmostApplication()
                .map(|a| a.processIdentifier())
                .filter(|p| *p != std::process::id() as i32)
                .or(self.ivars().preferences_anchor.get());
            let (display, mut choices, partial) =
                windows::pair_candidates_for_app(self.mtm(), prior_pid)?;
            self.ivars().picking.set(true);
            #[allow(deprecated)]
            NSApplication::sharedApplication(self.mtm()).activateIgnoringOtherApps(true);
            let result = (|| {
                let selected = if let Some((layout, _)) = saved.as_ref() {
                    let mut selected = Vec::new();
                    for (i, window) in layout.windows.iter().enumerate() {
                        let matching: Vec<usize> = choices
                            .iter()
                            .enumerate()
                            .filter(|(_, c)| c.bundle_id == window.bundle_id)
                            .map(|(i, _)| i)
                            .collect();
                        let index = match matching.as_slice() {
                            [] => return Err(format!("Fenêtre absente pour {}. Ouvrez-la puis réessayez ; aucun placement effectué.", window.bundle_id)),
                            [index] if !partial => *index,
                            _ => {
                                // Reuse the same native chooser, scoped to one bundle.
                                let mut options = Vec::new();
                                for index in matching.into_iter().rev() { options.push(choices.remove(index)); }
                                options.reverse();
                                let title = format!("{} à réappliquer (plusieurs fenêtres de cette application)", i + 1);
                                let Some(index) = choose_window(self.mtm(), &title, &options, partial) else { return Ok("Disposition annulée ; aucune fenêtre déplacée.".into()); };
                                let choice = options.remove(index);
                                choices.extend(options); selected.push(choice); continue;
                            }
                        };
                        selected.push(choices.remove(index));
                    }
                    let second = selected.pop().unwrap();
                    let first = selected.pop().unwrap();
                    [first, second]
                } else {
                    let Some(first) =
                        choose_window(self.mtm(), "1/2 à enregistrer", &choices, partial)
                    else {
                        return Ok("Enregistrement annulé.".into());
                    };
                    let first = choices.remove(first);
                    let Some(second) =
                        choose_window(self.mtm(), "2/2 à enregistrer", &choices, partial)
                    else {
                        return Ok("Enregistrement annulé.".into());
                    };
                    [first, choices.remove(second)]
                };
                if let Some((layout, backup)) = saved {
                    let result = windows::apply_saved_layout(
                        &layout,
                        display,
                        selected,
                        &mut self.ivars().layout_snapshot.borrow_mut(),
                        prior_pid,
                    )?;
                    Ok(if backup {
                        format!("{result} Configuration récupérée depuis la sauvegarde valide ; fichier principal conservé.")
                    } else {
                        result
                    })
                } else {
                    let layout = windows::capture_layout(self.mtm(), [&selected[0], &selected[1]])?;
                    let mut catalog = self.read_catalog()?.0;
                    let id = catalog.upsert(
                        profile_id,
                        profile_name.as_deref().ok_or("Nom absent.")?,
                        &layout,
                    )?;
                    config::save(&path, &catalog)?;
                    self.set_catalog(catalog, Some(id), true);
                    Ok("Espace enregistré. Il est disponible dans le menu et après redémarrage de Kigi.".into())
                }
            })();
            self.ivars().picking.set(false);
            result
        })();
        diagnostics::record(
            "workspace.personal",
            format!("save={save} success={}", result.is_ok()),
        );
        self.status(&result.unwrap_or_else(|e: String| e));
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
        if let Some(prefs) = self.ivars().preferences.borrow().as_ref() {
            prefs.show_permissions();
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
        self.remember_permissions();
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
        self.sync_preferences();
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
        let result = self.ivars().status.borrow().clone();
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
        let mouse_scroll = self
            .ivars()
            .mouse_scroll_filter
            .borrow()
            .as_ref()
            .map(MouseScrollTap::report)
            .unwrap_or_else(|| {
                format!(
                    "inactive; last_session=[{}]",
                    self.ivars().mouse_scroll_last.borrow()
                )
            });
        let probe = windows::permission_probe();
        let peers = other_instances().len();
        let translocated = executable.contains("/AppTranslocation/");
        let os = NSProcessInfo::processInfo().operatingSystemVersionString();
        let exported_at = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        let performance = diagnostics::performance_report();
        let launch = self.ivars().launch_validation.borrow();
        diagnostics::report(&format!("performance={performance}\nlaunch_validation={launch}\nmacOS={os}\narchitecture={}\nbundle=io.kigi.windows\npid={}\nother_instances={peers}\napp_translocated={translocated}\nexecutable={executable}\nexported_at_unix_utc={exported_at}\naccessibility={}\n{shortcut}\nlast_result={result}\ntrackpad={trackpad}\nmouse_scroll={mouse_scroll}\npermission_probe={probe}\nprofiles_count={}",
            std::env::consts::ARCH, std::process::id(), windows::trusted(), self.ivars().catalog.borrow().profiles.len()))
    }

    fn status(&self, text: &str) {
        diagnostics::record("status", text);
        if let Some(prefs) = self.ivars().preferences.borrow().as_ref() {
            prefs.status.setStringValue(&NSString::from_str(text));
        }
        self.ivars().status.replace(text.into());
        self.sync_preferences();
    }
}

struct TouchState {
    samples: Cell<u64>,
    probe: RefCell<EdgeProbe>,
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
        fn ended(&self,event:&NSEvent) {
            if event.touchesMatchingPhase_inView(NSTouchPhase::Touching,Some(self)).count()==0{self.ivars().probe.borrow_mut().release();}
            else{self.ivars().probe.borrow_mut().cancel();}
        }
        #[unsafe(method(touchesCancelledWithEvent:))]
        fn cancelled(&self, _: &NSEvent) { self.ivars().probe.borrow_mut().cancel(); }
    }
);
impl TouchView {
    fn new(mtm: MainThreadMarker) -> Retained<Self> {
        let this = Self::alloc(mtm).set_ivars(TouchState {
            samples: Cell::new(0),
            probe: RefCell::new(EdgeProbe::default()),
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
        let samples = self.ivars().samples.get() + 1;
        self.ivars().samples.set(samples);
        if samples == 1 {
            diagnostics::record(
                "trackpad.local.contacts",
                "received=true coordinates_recorded=false",
            );
        }
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
        } else if let Some(edge) = probe.moved(contact, 1) {
            self.ivars().count.set(self.ivars().count.get() + 1);
            diagnostics::record(
                "trackpad.local.candidate",
                format!("edge={edge:?} count={}", self.ivars().count.get()),
            );
        }
        if self.ivars().last_ui.get().elapsed().as_millis() >= 33 {
            self.ivars().last_ui.set(Instant::now());
            if let Some(label) = self.ivars().label.get() {
                label.setStringValue(&NSString::from_str(&format!(
                    "Test LOCAL : contacts reçus, x={:.2}, y={:.2}, phase={:?}.\nGeste(s) bord 10 % vers intérieur : {}.\nLevez les doigts pour recommencer. Aucune action globale.",
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
pub fn queue_mouse_filter_interrupted() {
    CONTROLLER.with(|slot| {
        if let Some(controller) = slot.borrow().as_ref() {
            let _: () = unsafe {
                msg_send![&**controller, performSelector:sel!(mouseFilterInterrupted:), withObject:std::ptr::null::<AnyObject>(), afterDelay:0.0f64]
            };
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
pub fn schedule_shortcut(index: usize) {
    CONTROLLER.with(|slot| {
        if let Some(controller)=slot.borrow().as_ref(){
            if controller.ivars().picking.get(){return;}
            controller.ivars().hotkey_count.set(controller.ivars().hotkey_count.get()+1);
            controller.ivars().shortcut_queue.set(Some(index));
            controller.cancel_gesture_dispatch();
            diagnostics::record("shortcut.received",format!("slot={index}"));
            unsafe{let _:()=msg_send![&**controller,performSelectorOnMainThread:sel!(runShortcut:),withObject:std::ptr::null::<AnyObject>(),waitUntilDone:false];}
        }
    });
}
pub fn shortcut_captured(value: Option<(u32, u32, String)>) {
    CONTROLLER.with(|slot| {
        if let Some(controller)=slot.borrow().as_ref(){
            controller.ivars().captured.replace(Some(value));
            unsafe{let _:()=msg_send![&**controller,performSelector:sel!(shortcutCaptureDone:),withObject:std::ptr::null::<AnyObject>(),afterDelay:0.0f64];}
        }
    });
}
fn shortcuts_path() -> Result<std::path::PathBuf, String> {
    Ok(config::path()?.with_file_name("shortcuts.json"))
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
        let activation_check = std::env::args().any(|a| a == "--activation-check");
        let controller = Controller::new(mtm);
        controller.ivars().activation_check.set(activation_check);
        controller
            .ivars()
            .trackpad_last_summary
            .replace("Test global arrêté. Aucun geste global validé.".into());
        controller.remember_launch();
        controller.remember_context();
        app.setActivationPolicy(if activation_check {
            NSApplicationActivationPolicy::Accessory
        } else {
            NSApplicationActivationPolicy::Regular
        });
        let status_bar = NSStatusBar::systemStatusBar();
        let status_item = status_bar.statusItemWithLength(-1.0);
        if let Some(button) = status_item.button(mtm) {
            button.setTitle(ns_string!("Kigi"));
        }
        let menu = NSMenu::new(mtm);
        menu_action(&menu, &controller, "Ouvrir Kigi…", sel!(openProfiles:));
        menu.addItem(&NSMenuItem::separatorItem(mtm));
        let profiles_menu = NSMenu::new(mtm);
        let profiles_parent = NSMenuItem::new(mtm);
        profiles_parent.setTitle(ns_string!("Espaces enregistrés"));
        profiles_parent.setSubmenu(Some(&profiles_menu));
        menu.addItem(&profiles_parent);
        controller.ivars().profiles_menu.set(profiles_menu).ok();
        controller.refresh_profiles(None);
        menu_action(
            &menu,
            &controller,
            "Travail — Chrome 100 %",
            sel!(workspaceWork:),
        );
        menu_action(
            &menu,
            &controller,
            "Réunion — Teams / Chrome",
            sel!(workspaceMeeting:),
        );
        menu_action(
            &menu,
            &controller,
            "Restaurer la disposition précédente",
            sel!(workspaceRestore:),
        );
        menu.addItem(&NSMenuItem::separatorItem(mtm));
        menu_action(&menu, &controller, "Quitter Kigi", sel!(quit:));
        menu.setDelegate(Some(ProtocolObject::from_ref(&*controller)));
        app.setDelegate(Some(ProtocolObject::from_ref(&*controller)));
        let main_menu = NSMenu::new(mtm);
        let app_menu = NSMenu::new(mtm);
        let open = menu_action(&app_menu, &controller, "Ouvrir Kigi…", sel!(openProfiles:));
        open.setKeyEquivalent(ns_string!(","));
        let quit = menu_action(&app_menu, &controller, "Quitter Kigi", sel!(quit:));
        quit.setKeyEquivalent(ns_string!("q"));
        let app_parent = NSMenuItem::new(mtm);
        app_parent.setSubmenu(Some(&app_menu));
        main_menu.addItem(&app_parent);
        for (title, entries) in [
            (
                "Édition",
                vec![
                    ("Annuler", sel!(undo:), "z"),
                    ("Rétablir", sel!(redo:), "Z"),
                    ("Couper", sel!(cut:), "x"),
                    ("Copier", sel!(copy:), "c"),
                    ("Coller", sel!(paste:), "v"),
                    ("Tout sélectionner", sel!(selectAll:), "a"),
                ],
            ),
            (
                "Fenêtre",
                vec![
                    ("Fermer", sel!(performClose:), "w"),
                    ("Réduire", sel!(performMiniaturize:), "m"),
                ],
            ),
        ] {
            let submenu = NSMenu::new(mtm);
            submenu.setTitle(&NSString::from_str(title));
            for (label, action, key) in entries {
                let item = unsafe {
                    NSMenuItem::initWithTitle_action_keyEquivalent(
                        NSMenuItem::alloc(mtm),
                        &NSString::from_str(label),
                        Some(action),
                        &NSString::from_str(key),
                    )
                };
                submenu.addItem(&item);
            }
            let parent = NSMenuItem::new(mtm);
            parent.setTitle(&NSString::from_str(title));
            parent.setSubmenu(Some(&submenu));
            main_menu.addItem(&parent);
        }
        app.setMainMenu(Some(&main_menu));
        status_item.setMenu(Some(&menu));
        CONTROLLER.with(|c| c.replace(Some(controller.clone())));
        let defaults = NSUserDefaults::standardUserDefaults();
        controller.ivars().personal_enabled.set(
            defaults
                .objectForKey(ns_string!("personalHotkeysEnabled"))
                .is_none()
                || defaults.boolForKey(ns_string!("personalHotkeysEnabled")),
        );
        match shortcuts_path().and_then(|p| shortcuts::load(&p)) {
            Ok(settings) => {
                controller.ivars().shortcuts.replace(settings);
            }
            Err(e) => controller.status(&e),
        }
        if !activation_check {
            controller.register_hotkeys();
        }
        controller.ivars().mouse_scroll_requested.set(
            NSUserDefaults::standardUserDefaults()
                .boolForKey(ns_string!("disableMouseHorizontalScroll")),
        );
        if activation_check {
            controller.ivars().mouse_scroll_requested.set(false);
        }
        if controller.ivars().mouse_scroll_requested.get() {
            if let Err(e) = controller.start_mouse_scroll() {
                controller.status(&e);
            }
        }
        controller.update_mouse_scroll_label();
        let center = NSWorkspace::sharedWorkspace().notificationCenter();
        unsafe {
            for name in [
                NSWorkspaceWillSleepNotification,
                NSWorkspaceScreensDidSleepNotification,
                NSWorkspaceSessionDidResignActiveNotification,
            ] {
                center.addObserver_selector_name_object(
                    &controller,
                    sel!(suspendInputs:),
                    Some(name),
                    None,
                );
            }
        }
        unsafe {
            for name in [
                NSWorkspaceDidWakeNotification,
                NSWorkspaceScreensDidWakeNotification,
                NSWorkspaceSessionDidBecomeActiveNotification,
            ] {
                center.addObserver_selector_name_object(
                    &controller,
                    sel!(resumeMouseScroll:),
                    Some(name),
                    None,
                );
            }
        }
        controller.remember_context();
        unsafe {
            center.addObserver_selector_name_object(
                &controller,
                sel!(workspaceContextChanged:),
                Some(NSWorkspaceDidActivateApplicationNotification),
                None,
            );
            if activation_check {
                let _: () = msg_send![&*controller,performSelector:sel!(activationCheck:),withObject:std::ptr::null::<AnyObject>(),afterDelay:0.0f64];
                let _: () = msg_send![&*controller,performSelector:sel!(activationCheckTimeout:),withObject:std::ptr::null::<AnyObject>(),afterDelay:12.0f64];
            } else {
                let _: () = msg_send![&*controller, performSelector:sel!(openProfiles:), withObject:std::ptr::null::<AnyObject>(), afterDelay:0.0f64];
            }
        }
        if std::env::args().any(|a| a == "--startup-report") {
            unsafe {
                let _: () = msg_send![&*controller,performSelector:sel!(startupReport:),withObject:std::ptr::null::<AnyObject>(),afterDelay:2.0f64];
            }
        }
        app.run();
        controller.remember_permissions();
        app.setDelegate(None);
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
        controller.stop_mouse_scroll();
        if let Some(prefs) = controller.ivars().preferences.borrow_mut().take() {
            prefs.window.setDelegate(None);
            prefs.window.close();
        }
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
