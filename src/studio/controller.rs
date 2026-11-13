//! Studio adapters to existing native services; Controller remains their sole owner.
use super::*;
use crate::studio::{bridge::Failure, system::SystemCommand as S};
use serde_json::{json, Value};

impl Controller {
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
        selected: usize,
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
        self.populate_shortcuts(selected);
        self.sync_preferences();
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
            S::ShortcutsList(_) => Ok(self.studio_shortcuts_data()),
            S::Gestures(_) => Ok(self.studio_gestures_data()),
            S::Mouse(_) => Ok(self.studio_mouse_data()),
            S::Capture(_) => self.studio_capture_shortcut(),
            S::Upsert(input) => {
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
                self.populate_shortcuts(0);
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
                if !input.enabled {
                    self.stop_tap();
                } else if !self.gestures_active() {
                    if !windows::trusted() {
                        return Err(Failure::new(
                            "PERMISSION_REQUIRED",
                            "Autorisez Accessibilité avant les gestes.",
                        ));
                    }
                    self.stop_tap();
                    let tap =
                        Tap::start_actions().map_err(|e| Failure::new("INPUT_UNAVAILABLE", e))?;
                    self.ivars().tap.replace(Some(tap));
                    self.begin_trackpad_poll();
                }
                Ok(self.studio_gestures_data())
            }
            S::StopGestures(_) => {
                self.stop_tap();
                Ok(self.studio_gestures_data())
            }
            S::LocalProbe(_) => {
                self.local_probe(sel!(localProbe:), None);
                Ok(json!({"opened":true}))
            }
            S::GlobalProbe(_) => {
                self.stop_tap();
                let tap = Tap::start().map_err(|e| Failure::new("INPUT_UNAVAILABLE", e))?;
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
                self.permission(sel!(permission:), None);
                Ok(json!({"requested":true,"accessibility":windows::trusted()}))
            }
            S::Input(_) => Ok(json!({"requested":true,"inputAccess":Tap::request_input()})),
            S::OpenSettings(input) => {
                let _ = &input.section;
                let workspace = NSWorkspace::sharedWorkspace();
                let url = workspace
                    .URLForApplicationWithBundleIdentifier(ns_string!(
                        "com.apple.systempreferences"
                    ))
                    .ok_or_else(|| Failure::invalid("Réglages Système introuvable."))?;
                if !workspace.openURL(&url) {
                    return Err(Failure::invalid("Ouverture de Réglages Système refusée."));
                }
                Ok(
                    json!({"requested":true,"message":"Ouverture demandée. Sélectionnez la rubrique dans Confidentialité et sécurité."}),
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
