//! Studio adapters to existing native services; Controller remains their sole owner.
use super::*;
use crate::studio::{bridge::Failure, system::SystemCommand as S};
use serde_json::{json, Value};

impl Controller {
    pub(super) fn inhibit_teams(&self) {
        if self.ivars().teams_settings.borrow().enabled
            || self.ivars().teams_automation.borrow().tracking()
        {
            self.ivars().teams_automation.borrow_mut().inhibit();
        }
    }
    pub(super) fn load_teams_settings(&self) {
        if let Some(raw) =
            NSUserDefaults::standardUserDefaults().stringForKey(ns_string!("teamsAutomationV1"))
        {
            match serde_json::from_str::<crate::meetings::Settings>(&raw.to_string())
                .map_err(|_| "Profil Teams illisible ; automatisme désactivé.".to_string())
                .and_then(|settings| {
                    settings.validate()?;
                    Ok(settings)
                }) {
                Ok(settings) => {
                    self.ivars().teams_settings.replace(settings);
                }
                Err(message) => {
                    self.ivars().teams_automation.borrow_mut().status = message;
                }
            }
        }
        if self.ivars().teams_settings.borrow().enabled {
            self.schedule_capture(sel!(teamsPoll:), 1.0);
        }
    }
    fn teams_revision(&self) -> String {
        format!(
            "{}:{}",
            std::process::id(),
            self.ivars().teams_revision.get()
        )
    }
    pub(super) fn studio_meetings_data(&self) -> Value {
        json!({"settings":*self.ivars().teams_settings.borrow(),"revision":self.teams_revision(),"status":self.ivars().teams_automation.borrow().status,"accessibility":windows::trusted(),"teamsVersion":windows::teams_version()})
    }
    fn identify_teams(&self) -> Result<Value, Failure> {
        self.inhibit_teams();
        let mut choices = windows::teams_choices().map_err(Failure::invalid)?;
        self.ivars().picking.set(true);
        #[allow(deprecated)]
        NSApplication::sharedApplication(self.mtm()).activateIgnoringOtherApps(true);
        let result = (|| {
            let Some(chat) = choose_window(self.mtm(), "Chat Teams à identifier", &choices, false)
            else {
                return Ok(json!({"canceled":true}));
            };
            let chat = choices.remove(chat);
            let Some(meeting) =
                choose_window(self.mtm(), "Réunion Teams à identifier", &choices, false)
            else {
                return Ok(json!({"canceled":true}));
            };
            Ok(
                json!({"canceled":false,"chatTitle":chat.label,"meetingTitle":choices[meeting].label}),
            )
        })();
        self.ivars().picking.set(false);
        result
    }

    fn open_permission_settings(
        &self,
        section: &crate::studio::system::Section,
    ) -> Result<bool, Failure> {
        let workspace = NSWorkspace::sharedWorkspace();
        // Settings routing is best effort: URL acceptance is not proof of the visible pane.
        let targeted =
            objc2_foundation::NSURL::URLWithString(&NSString::from_str(section.settings_url()))
                .is_some_and(|url| workspace.openURL(&url));
        if !targeted {
            let url = workspace
                .URLForApplicationWithBundleIdentifier(ns_string!("com.apple.systempreferences"))
                .ok_or_else(|| Failure::invalid("Réglages Système introuvable."))?;
            if !workspace.openURL(&url) {
                return Err(Failure::invalid("Ouverture de Réglages Système refusée."));
            }
        }
        diagnostics::record(
            "permissions.settings.request",
            format!("section={section:?} targeted={targeted}"),
        );
        Ok(targeted)
    }
    fn permission_settings_message(
        section: &crate::studio::system::Section,
        targeted: bool,
    ) -> String {
        if targeted {
            format!("Ouverture de {} demandée. Activez Kigi puis actualisez. Si macOS affiche une autre page, sélectionnez cette rubrique dans Confidentialité et sécurité.", section.label())
        } else {
            format!("Réglages Système ouvert. Dans Confidentialité et sécurité, sélectionnez {} et activez Kigi, puis actualisez.", section.label())
        }
    }

    pub(super) fn studio_capture_layout(&self, revision: &str) -> Result<Value, Failure> {
        let path = studio_path().map_err(Failure::invalid)?;
        self.ivars()
            .studio_store
            .borrow()
            .check_revision(&path, revision)?;
        if self.ivars().picking.get() {
            return Err(Failure::new("BUSY", "Terminez le dialogue en cours."));
        }
        if !windows::trusted() {
            return Err(Failure::new(
                "PERMISSION_REQUIRED",
                "Autorisez Accessibilité avant de capturer des fenêtres.",
            ));
        }
        self.cancel_open();
        self.cancel_gesture_dispatch();
        self.cancel_shortcut_capture();
        let prior = NSWorkspace::sharedWorkspace()
            .frontmostApplication()
            .map(|a| a.processIdentifier())
            .filter(|p| *p != std::process::id() as i32)
            .or(self.ivars().activation_anchor.get());
        let (_, mut choices, partial) = windows::pair_candidates_for_app(self.mtm(), prior)
            .map_err(|e| Failure::new("CAPTURE_FAILED", e))?;
        if std::env::args().any(|a| a == "--studio-capture-check")
            && std::env::var_os("KIGI_TEST_STUDIO_CAPTURE").is_some()
        {
            choices.retain(|c| {
                matches!(
                    c.bundle_id.as_str(),
                    "io.kigi.focus.source" | "io.kigi.focus.target"
                )
            });
            if choices.len() != 2 {
                return Err(Failure::new(
                    "CAPTURE_FAILED",
                    "Deux fenêtres de test dédiées sont requises.",
                ));
            }
        }
        self.ivars().picking.set(true);
        #[allow(deprecated)]
        NSApplication::sharedApplication(self.mtm()).activateIgnoringOtherApps(true);
        let result = (|| {
            let Some(first) = choose_window(self.mtm(), "1/2 à capturer", &choices, partial)
            else {
                return Ok(json!({"canceled":true}));
            };
            let first = choices.remove(first);
            let Some(second) = choose_window(self.mtm(), "2/2 à capturer", &choices, partial)
            else {
                return Ok(json!({"canceled":true}));
            };
            let second = choices.remove(second);
            let layout = windows::capture_layout(self.mtm(), [&first, &second])
                .map_err(|e| Failure::new("CAPTURE_FAILED", e))?;
            self.ivars()
                .studio_store
                .borrow()
                .check_revision(&path, revision)?;
            Ok(json!({"canceled":false,"windows":layout.windows}))
        })();
        self.ivars().picking.set(false);
        result
    }

    pub(super) fn studio_shortcuts_data(&self) -> Value {
        let runtime = self.ivars().shortcuts.borrow();
        let mut value = match shortcuts_path() {
            Ok(path) => self
                .ivars()
                .studio_shortcuts
                .borrow_mut()
                .list(&path, &runtime),
            Err(message) => {
                json!({"settings":*runtime,"combinations":[],"revision":null,"canWrite":false,"externalChanged":true,"error":message})
            }
        };
        let keys = self.ivars().hotkeys.borrow();
        value["globalEnabled"] = json!(self.ivars().personal_enabled.get());
        value["registered"] = json!(keys.as_ref().is_some_and(|k| !k.failed));
        value["registrationFailed"] = json!(keys.as_ref().is_some_and(|k| k.failed));
        value["registrationReport"] =
            json!(keys.as_ref().map(|k| k.report.clone()).unwrap_or_default());
        value["capturing"] = json!(self.ivars().studio_recorder.borrow().is_some());
        value
    }
    pub(super) fn studio_gestures_data(&self) -> Value {
        let tap = self.ivars().tap.borrow();
        let counts=tap.as_ref().map(|t|{let e=t.evidence();json!({"events":e.events,"contacts":e.contacts,"samples":e.samples,"began":e.began,"edgeStarts":e.edge_starts,"recognized":e.recognized})});
        json!({"active":tap.as_ref().is_some_and(Tap::active_actions),"listening":tap.as_ref().is_some_and(|t|!t.interrupted()),"interrupted":tap.as_ref().is_some_and(Tap::interrupted),"counts":counts,"summary":tap.as_ref().map(Tap::summary).unwrap_or_else(||self.ivars().trackpad_last_summary.borrow().clone()),"inputAccess":Tap::input_trusted(),"accessibility":windows::trusted()})
    }
    pub(super) fn studio_mouse_data(&self) -> Value {
        json!({"requested":self.ivars().mouse_scroll_requested.get(),"active":self.mouse_scroll_active(),"accessibility":windows::trusted(),"report":self.ivars().mouse_scroll_filter.borrow().as_ref().map(MouseScrollTap::report).unwrap_or_else(||self.ivars().mouse_scroll_last.borrow().clone())})
    }
    pub(super) fn persist_shortcuts_transaction(
        &self,
        settings: Shortcuts,
        _selected: usize,
        source: Option<&Option<Vec<u8>>>,
    ) -> Result<(), String> {
        settings.validate()?;
        self.ivars().shortcut_queue.take();
        self.cancel_open();
        self.cancel_shortcut_capture();
        self.ivars().hotkeys.borrow_mut().take();
        let keys = Hotkeys::register(self.ivars().personal_enabled.get(), &settings);
        if keys.failed {
            diagnostics::record("shortcuts.save.rejected", "registration_conflict=true");
            let message = format!(
                "macOS refuse une combinaison. Les anciens raccourcis sont conservés.\n{}",
                keys.report
            );
            drop(keys);
            self.register_hotkeys();
            return Err(message);
        }
        let result = shortcuts_path().and_then(|path| match source {
            Some(bytes) => {
                shortcuts::save_checked(&path, &settings, &self.ivars().shortcuts.borrow(), bytes)
            }
            None => shortcuts::save(&path, &settings, &self.ivars().shortcuts.borrow()),
        });
        if let Err(message) = result {
            diagnostics::record("shortcuts.save.rejected", "write_failed=true");
            drop(keys);
            self.register_hotkeys();
            return Err(message);
        }
        self.ivars().shortcuts.replace(settings);
        self.ivars().hotkeys.replace(Some(keys));
        diagnostics::record("shortcuts.save.complete", "success=true");

        Ok(())
    }
    fn studio_shortcut_source(&self, revision: &str) -> Result<Option<Vec<u8>>, Failure> {
        self.ivars().studio_shortcuts.borrow().prepare(
            &shortcuts_path().map_err(Failure::invalid)?,
            revision,
            &self.ivars().shortcuts.borrow(),
        )
    }
    fn studio_capture_shortcut(&self) -> Result<Value, Failure> {
        self.cancel_shortcut_capture();
        if self.ivars().picking.replace(true) {
            return Err(Failure::new("BUSY", "Un dialogue est déjà ouvert."));
        }
        self.ivars().shortcut_queue.take();
        self.cancel_open();
        self.cancel_gesture_dispatch();
        self.ivars().hotkeys.borrow_mut().take();
        self.ivars().studio_capture_result.borrow_mut().take();
        let window = unsafe {
            NSWindow::initWithContentRect_styleMask_backing_defer(
                NSWindow::alloc(self.mtm()),
                NSRect::new(NSPoint::new(0., 0.), NSSize::new(510., 170.)),
                NSWindowStyleMask::Titled | NSWindowStyleMask::Closable,
                NSBackingStoreType::Buffered,
                false,
            )
        };
        unsafe {
            window.setReleasedWhenClosed(false);
        }
        window.setTitle(ns_string!("Capturer un raccourci"));
        window.setDelegate(Some(ProtocolObject::from_ref(self)));
        let root = NSView::initWithFrame(
            NSView::alloc(self.mtm()),
            NSRect::new(NSPoint::new(0., 0.), NSSize::new(510., 170.)),
        );
        let label = NSTextField::labelWithString(
            ns_string!("Une combinaison avec ⌘, ⌥ ou ⌃. Échap annule. Délai : 15 s."),
            self.mtm(),
        );
        label.setFrame(NSRect::new(NSPoint::new(20., 119.), NSSize::new(470., 36.)));
        root.addSubview(&label);
        let recorder = crate::shortcut_recorder::Recorder::new(self.mtm());
        recorder.setFrame(NSRect::new(NSPoint::new(20., 65.), NSSize::new(470., 42.)));
        root.addSubview(&recorder);
        let cancel = unsafe {
            NSButton::buttonWithTitle_target_action(
                ns_string!("Annuler"),
                Some(self),
                Some(sel!(cancelStudioCapture:)),
                self.mtm(),
            )
        };
        cancel.setFrame(NSRect::new(NSPoint::new(390., 15.), NSSize::new(100., 32.)));
        root.addSubview(&cancel);
        window.setContentView(Some(&root));
        window.setInitialFirstResponder(Some(&recorder));
        window.center();
        window.makeKeyAndOrderFront(None);
        #[allow(deprecated)]
        NSApplication::sharedApplication(self.mtm()).activateIgnoringOtherApps(true);
        self.ivars().studio_recorder.replace(Some(recorder.clone()));
        if recorder.start() {
            self.schedule_capture(sel!(shortcutCaptureTimeout:), 15.0);
            if std::env::args().any(|a| a == "--studio-system-check") {
                let count = self.ivars().capture_check_count.get() + 1;
                self.ivars().capture_check_count.set(count);
                if count == 2 {
                    self.schedule_capture(sel!(studioCaptureFixture:), 0.15);
                }
            }
            NSApplication::sharedApplication(self.mtm()).runModalForWindow(&window);
        }
        self.cancel_capture_timer();
        recorder.cancel();
        self.ivars().studio_recorder.borrow_mut().take();
        window.setDelegate(None);
        window.close();
        self.ivars().captured.borrow_mut().take();
        self.ivars().picking.set(false);
        self.register_hotkeys();
        let result = self
            .ivars()
            .studio_capture_result
            .borrow_mut()
            .take()
            .flatten();
        let restored = self
            .ivars()
            .hotkeys
            .borrow()
            .as_ref()
            .is_some_and(|k| !k.failed);
        Ok(match result {
            Some((key, modifiers, key_label)) => {
                json!({"canceled":false,"key":key,"modifiers":modifiers,"keyLabel":key_label,"registrationRestored":restored})
            }
            None => json!({"canceled":true,"registrationRestored":restored}),
        })
    }
    pub(super) fn copy_diagnostics(&self) -> Result<(), String> {
        self.record_state("diagnostic.copy");
        let text = self.log_report();
        let pasteboard = NSPasteboard::generalPasteboard();
        pasteboard.clearContents();
        if pasteboard.setString_forType(&NSString::from_str(&text), unsafe {
            NSPasteboardTypeString
        }) {
            Ok(())
        } else {
            Err("Impossible de copier le rapport.".into())
        }
    }
    pub(super) fn export_diagnostics(&self) -> Result<bool, String> {
        let panel = NSSavePanel::savePanel(self.mtm());
        panel.setNameFieldStringValue(&NSString::from_str(&format!(
            "Kigi-diagnostic-{}.txt",
            env!("CARGO_PKG_VERSION")
        )));
        panel.setCanCreateDirectories(true);
        #[allow(deprecated)]
        panel.setAllowedFileTypes(Some(&NSArray::from_slice(&[ns_string!("txt")])));
        self.ivars().picking.set(true);
        let result = panel.runModal();
        self.ivars().picking.set(false);
        if result != NSModalResponseOK {
            return Ok(false);
        }
        let path = panel
            .URL()
            .and_then(|u| u.path())
            .ok_or("Destination indisponible.")?;
        self.record_state("diagnostic.export");
        std::fs::write(path.to_string(), self.log_report())
            .map_err(|_| "Impossible d’écrire le rapport.".to_string())?;
        Ok(true)
    }
    pub(super) fn studio_system(&self, op: &S) -> Result<Value, Failure> {
        if matches!(op, S::CancelCapture(_)) {
            self.cancel_shortcut_capture();
            return Ok(json!({"canceled":true}));
        }
        if self.ivars().picking.get()
            && !matches!(op, S::ShortcutsList(_) | S::Gestures(_) | S::Mouse(_))
        {
            return Err(Failure::new("BUSY", "Terminez le dialogue en cours."));
        }
        match op {
            S::Login(input) => {
                let path = crate::login_item::path().map_err(Failure::invalid)?;
                let exe = std::env::current_exe()
                    .map_err(|_| Failure::invalid("Chemin d’application indisponible."))?;
                crate::login_item::set(&path, &exe, input.enabled).map_err(Failure::invalid)?;
                Ok(self.studio_runtime_data())
            }
            S::Runtime(_) => Ok(self.studio_runtime_data()),
            S::RuntimeSave(input) => self.studio_save_runtime(input),
            S::MeetingsApply(_) => {
                self.apply_teams_now();
                Ok(json!({"message":self.ivars().status.borrow().clone()}))
            }
            S::Meetings(_) => Ok(self.studio_meetings_data()),
            S::MeetingsIdentify(_) => self.identify_teams(),
            S::MeetingsProbe(input) => {
                input.settings.validate().map_err(Failure::invalid)?;
                Ok(
                    json!({"message":windows::teams_probe(&input.settings).map_err(Failure::invalid)?}),
                )
            }
            S::MeetingsPause(_) => {
                self.inhibit_teams();
                Ok(self.studio_meetings_data())
            }
            S::MeetingsSave(input) => {
                if input.expected_revision != self.teams_revision() {
                    return Err(Failure::new(
                        "STALE_REVISION",
                        "Les réglages Teams ont changé. Actualisez avant de réessayer.",
                    ));
                }
                input.settings.validate().map_err(Failure::invalid)?;
                if input.settings.enabled {
                    windows::teams_verify(&input.settings).map_err(Failure::invalid)?;
                }
                self.inhibit_teams();
                let raw = serde_json::to_string(&input.settings)
                    .map_err(|_| Failure::invalid("Profil Teams invalide."))?;
                // NSString is a valid property-list object owned through the call.
                unsafe {
                    NSUserDefaults::standardUserDefaults().setObject_forKey(
                        Some(&*NSString::from_str(&raw)),
                        ns_string!("teamsAutomationV1"),
                    );
                }
                self.ivars().teams_settings.replace(input.settings.clone());
                self.ivars()
                    .teams_revision
                    .set(self.ivars().teams_revision.get().wrapping_add(1));
                // Reconfiguration never restores or repositions an in-progress meeting.
                if !input.settings.enabled {
                    self.ivars()
                        .teams_automation
                        .replace(windows::TeamsAutomation::default());
                }
                unsafe {
                    let _: () = msg_send![objc2::class!(NSObject),cancelPreviousPerformRequestsWithTarget:self,selector:sel!(teamsPoll:),object:std::ptr::null::<AnyObject>()];
                }
                if input.settings.enabled {
                    self.schedule_capture(sel!(teamsPoll:), 1.0);
                }
                Ok(self.studio_meetings_data())
            }
            S::ShortcutsList(_) => Ok(self.studio_shortcuts_data()),
            S::Gestures(_) => Ok(self.studio_gestures_data()),
            S::Mouse(_) => Ok(self.studio_mouse_data()),
            S::Capture(_) => self.studio_capture_shortcut(),
            S::Upsert(input) => {
                if input.binding.enabled
                    && input.binding.key == 48
                    && input.binding.modifiers == 256
                    && self.ivars().runtime_settings.borrow().window_switcher
                {
                    return Err(Failure::invalid("⌘Tab est géré dans Général. Désactivez ce remplacement avant de l’utiliser comme raccourci."));
                }
                let source = self.studio_shortcut_source(&input.expected_revision)?;
                let mut settings = self.ivars().shortcuts.borrow().clone();
                let index = input.index.unwrap_or(settings.entries.len());
                if index > settings.entries.len() || index >= 32 {
                    return Err(Failure::invalid("Index ou limite de raccourcis invalide."));
                }
                if index == settings.entries.len() {
                    settings.entries.push(input.binding.clone());
                } else {
                    settings.entries[index] = input.binding.clone();
                }
                settings.validate().map_err(Failure::invalid)?;
                if let Command::ApplyProfile { profile_id } = input.binding.command {
                    let (catalog, _) = self.read_catalog().map_err(Failure::invalid)?;
                    catalog.profile(profile_id).map_err(Failure::invalid)?;
                }
                self.persist_shortcuts_transaction(settings, index, Some(&source))
                    .map_err(|e| Failure::new("SHORTCUT_CONFLICT", e))?;
                Ok(self.studio_shortcuts_data())
            }
            S::Delete(input) => {
                if !input.confirmed {
                    return Err(Failure::new(
                        "CONFIRMATION_REQUIRED",
                        "Confirmez la suppression.",
                    ));
                }
                let source = self.studio_shortcut_source(&input.expected_revision)?;
                let mut settings = self.ivars().shortcuts.borrow().clone();
                if input.index >= settings.entries.len() {
                    return Err(Failure::invalid("Raccourci absent."));
                }
                settings.entries.remove(input.index);
                self.persist_shortcuts_transaction(
                    settings,
                    input.index.saturating_sub(1),
                    Some(&source),
                )
                .map_err(|e| Failure::new("SHORTCUT_CONFLICT", e))?;
                Ok(self.studio_shortcuts_data())
            }
            S::Reset(input) => {
                if !input.confirmed {
                    return Err(Failure::new(
                        "CONFIRMATION_REQUIRED",
                        "Confirmez le rétablissement des raccourcis.",
                    ));
                }
                let source = self.studio_shortcut_source(&input.expected_revision)?;
                self.persist_shortcuts_transaction(Shortcuts::default(), 1, Some(&source))
                    .map_err(|e| Failure::new("SHORTCUT_CONFLICT", e))?;
                Ok(self.studio_shortcuts_data())
            }
            S::Reload(_) => {
                self.cancel_shortcut_capture();
                let settings = shortcuts::load(&shortcuts_path().map_err(Failure::invalid)?)
                    .map_err(Failure::invalid)?;
                self.ivars().hotkeys.borrow_mut().take();
                let keys = Hotkeys::register(self.ivars().personal_enabled.get(), &settings);
                if keys.failed {
                    drop(keys);
                    self.register_hotkeys();
                    return Err(Failure::new(
                        "SHORTCUT_CONFLICT",
                        "Réenregistrement refusé ; anciens raccourcis conservés.",
                    ));
                }
                self.ivars().shortcuts.replace(settings);
                self.ivars().hotkeys.replace(Some(keys));

                Ok(self.studio_shortcuts_data())
            }
            S::Global(input) => {
                self.cancel_shortcut_capture();
                self.ivars().shortcut_queue.take();
                self.cancel_open();
                self.ivars().hotkeys.borrow_mut().take();
                let keys = Hotkeys::register(input.enabled, &self.ivars().shortcuts.borrow());
                if keys.failed {
                    drop(keys);
                    self.register_hotkeys();
                    return Err(Failure::new(
                        "SHORTCUT_CONFLICT",
                        "Activation refusée ; réglage précédent conservé.",
                    ));
                }
                self.ivars().personal_enabled.set(input.enabled);
                self.ivars().hotkeys.replace(Some(keys));
                if !std::env::args().any(|a| a == "--studio-system-check") {
                    NSUserDefaults::standardUserDefaults()
                        .setBool_forKey(input.enabled, ns_string!("personalHotkeysEnabled"));
                }
                Ok(self.studio_shortcuts_data())
            }
            S::Reregister(_) => {
                self.cancel_shortcut_capture();
                self.register_hotkeys();
                Ok(self.studio_shortcuts_data())
            }
            S::Test(input) => {
                if self
                    .ivars()
                    .studio
                    .borrow()
                    .as_ref()
                    .is_some_and(|s| s.dirty())
                {
                    return Err(Failure::new(
                        "UNSAVED_CHANGES",
                        "Enregistrez avant de tester.",
                    ));
                }
                self.studio_shortcut_source(&input.expected_revision)?;
                let command = self
                    .ivars()
                    .shortcuts
                    .borrow()
                    .entries
                    .get(input.index)
                    .ok_or_else(|| Failure::invalid("Raccourci absent."))?
                    .command
                    .clone();
                self.execute_command(command);
                Ok(
                    json!({"requested":true,"message":"Commande transmise. Actualisez les diagnostics pour confirmer son résultat."}),
                )
            }
            S::GesturesEnable(input) => {
                self.set_gestures_requested(input.enabled, true);
                Ok(self.studio_gestures_data())
            }
            S::StopGestures(_) => {
                self.set_gestures_requested(false, false);
                Ok(self.studio_gestures_data())
            }
            S::LocalProbe(_) => {
                self.local_probe(sel!(localProbe:), None);
                Ok(json!({"opened":true}))
            }
            S::GlobalProbe(_) => {
                self.stop_tap();
                let tap = Tap::probe(&self.ivars().runtime_settings.borrow())
                    .map_err(|e| Failure::new("INPUT_UNAVAILABLE", e))?;
                self.ivars().tap.replace(Some(tap));
                self.begin_trackpad_poll();
                Ok(self.studio_gestures_data())
            }
            S::MouseEnable(input) => {
                if input.enabled {
                    self.start_mouse_scroll()
                        .map_err(|e| Failure::new("INPUT_UNAVAILABLE", e))?;
                } else {
                    self.stop_mouse_scroll();
                }
                self.ivars().mouse_scroll_requested.set(input.enabled);
                if !std::env::args().any(|a| a == "--studio-system-check") {
                    NSUserDefaults::standardUserDefaults()
                        .setBool_forKey(input.enabled, ns_string!("disableMouseHorizontalScroll"));
                }
                Ok(self.studio_mouse_data())
            }
            S::Accessibility(_) => {
                let trusted = self.refresh_permission();
                let action = self.ivars().permission_flow.borrow_mut().click(trusted);
                if action.request_prompt {
                    let immediate = windows::request_permission();
                    diagnostics::record(
                        "permission.ax.request",
                        format!("immediate_result={immediate} asynchronous=true"),
                    );
                    if !self.refresh_permission() {
                        self.schedule_permission_poll();
                    }
                }
                let trusted = windows::trusted();
                let settings_requested = action.open_settings && !trusted;
                let section = crate::studio::system::Section::Accessibility;
                let targeted = settings_requested && self.open_permission_settings(&section)?;
                let message = if trusted {
                    "Accessibilité accordée à cette copie.".into()
                } else {
                    Self::permission_settings_message(&section, targeted)
                };
                Ok(
                    json!({"requested":action.request_prompt,"settingsRequested":settings_requested,"settingsTargeted":targeted,"accessibility":trusted,"message":message}),
                )
            }
            S::Input(_) => {
                let action = self
                    .ivars()
                    .input_permission_flow
                    .borrow_mut()
                    .click(Tap::input_trusted());
                if action.request_prompt {
                    let _ = Tap::request_input();
                }
                let trusted = Tap::input_trusted();
                let settings_requested = action.open_settings && !trusted;
                let section = crate::studio::system::Section::InputMonitoring;
                let targeted = settings_requested && self.open_permission_settings(&section)?;
                let message = if trusted {
                    "Surveillance de l’entrée accordée à cette copie.".into()
                } else {
                    Self::permission_settings_message(&section, targeted)
                };
                Ok(
                    json!({"requested":action.request_prompt,"settingsRequested":settings_requested,"settingsTargeted":targeted,"inputAccess":trusted,"message":message}),
                )
            }
            S::OpenSettings(input) => {
                let targeted = self.open_permission_settings(&input.section)?;
                Ok(
                    json!({"requested":true,"settingsRequested":true,"settingsTargeted":targeted,"message":Self::permission_settings_message(&input.section,targeted)}),
                )
            }
            S::Copy(_) => {
                self.copy_diagnostics().map_err(Failure::invalid)?;
                Ok(json!({"copied":true}))
            }
            S::Export(_) => {
                let saved = self.export_diagnostics().map_err(Failure::invalid)?;
                Ok(json!({"saved":saved,"canceled":!saved}))
            }
            S::CancelCapture(_) => unreachable!(),
        }
    }
}
