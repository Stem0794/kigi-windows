//! Adapters for the personal workflow; all app/window work remains on the main loop.
use super::*;
use crate::studio::bridge::Failure;
use serde_json::{json, Value};
impl Controller {
    pub(super) fn studio_runtime_data(&self) -> Value {
        json!({"settings":*self.ivars().runtime_settings.borrow(),"revision":format!("{}:{}",std::process::id(),self.ivars().runtime_revision.get()),"switcherActive":self.ivars().personal_enabled.get()&&windows::trusted()&&self.ivars().window_hotkey.borrow().as_ref().is_some_and(|t|t.active()),"loginEnabled":crate::login_item::path().is_ok_and(|path|crate::login_item::enabled(&path))})
    }
    pub(super) fn save_runtime_settings(
        &self,
        settings: crate::runtime_settings::Settings,
    ) -> Result<(), String> {
        settings.validate()?;
        let raw = serde_json::to_string(&settings).map_err(|_| "Réglages invalides.")?;
        unsafe {
            NSUserDefaults::standardUserDefaults().setObject_forKey(
                Some(&*NSString::from_str(&raw)),
                ns_string!("personalWorkflowV1"),
            );
        }
        self.ivars().runtime_settings.replace(settings);
        self.ivars()
            .runtime_revision
            .set(self.ivars().runtime_revision.get().wrapping_add(1));
        Ok(())
    }
    pub(super) fn migrate_personal_bindings(&self) {
        if std::env::var_os("KIGI_TEST_STUDIO_CONFIG").is_some()
            || std::env::args().any(|a| a.ends_with("-check"))
        {
            return;
        }
        let defaults = NSUserDefaults::standardUserDefaults();
        if defaults.boolForKey(ns_string!("personalParityBindingsV1")) {
            return;
        }
        let original = self.ivars().shortcuts.borrow().clone();
        let updated = shortcuts::personal_parity(&original);
        if updated == original {
            defaults.setBool_forKey(true, ns_string!("personalParityBindingsV1"));
            return;
        }
        match shortcuts_path().and_then(|path| {
            if let Some(bytes) = shortcuts::read_bytes(&path)? {
                let backup = path.with_extension("before-personal-v1.json");
                match std::fs::OpenOptions::new()
                    .write(true)
                    .create_new(true)
                    .open(backup)
                {
                    Ok(mut file) => {
                        std::io::Write::write_all(&mut file, &bytes)
                            .and_then(|_| file.sync_all())
                            .map_err(|_| "Sauvegarde des raccourcis précédents impossible.")?;
                    }
                    Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {}
                    Err(_) => return Err("Sauvegarde des raccourcis précédents impossible.".into()),
                }
            }
            shortcuts::save(&path, &updated, &original)
        }) {
            Ok(()) => {
                self.ivars().shortcuts.replace(updated);
                defaults.setBool_forKey(true, ns_string!("personalParityBindingsV1"));
                self.register_hotkeys();
            }
            Err(message) => self.status(&message),
        }
    }
    pub(super) fn load_runtime_settings(&self) {
        if let Some(raw) =
            NSUserDefaults::standardUserDefaults().stringForKey(ns_string!("personalWorkflowV1"))
        {
            match serde_json::from_str::<crate::runtime_settings::Settings>(&raw.to_string())
                .map_err(|_| "Réglages personnels illisibles.".to_string())
                .and_then(|s| {
                    s.validate()?;
                    Ok(s)
                }) {
                Ok(settings) => {
                    self.ivars().runtime_settings.replace(settings);
                }
                Err(message) => self.status(&message),
            }
        }
    }
    pub(super) fn set_gestures_requested(&self, enabled: bool, prompt: bool) {
        let mut settings = self.ivars().runtime_settings.borrow().clone();
        settings.gestures_enabled = enabled;
        if let Err(message) = self.save_runtime_settings(settings) {
            self.status(&message);
            return;
        }
        self.stop_tap();
        self.ivars().input_resume_attempts.set(0);
        if enabled {
            let was_picking = self.ivars().picking.replace(true);
            let result = Tap::configured(&self.ivars().runtime_settings.borrow(), prompt);
            self.ivars().picking.set(was_picking);
            match result {
                Ok(tap) => {
                    self.ivars().tap.replace(Some(tap));
                    self.begin_trackpad_poll();
                    self.status("Gestes activés et préférence mémorisée.");
                }
                Err(message) => self.status(&message),
            }
        } else {
            self.status("Gestes désactivés ; ils ne reprendront pas automatiquement.");
        }
    }
    pub(super) fn schedule_inputs_resume(&self, delay: f64) {
        unsafe {
            let _: () = msg_send![objc2::class!(NSObject),cancelPreviousPerformRequestsWithTarget:self,selector:sel!(resumePersonalInputs:),object:std::ptr::null::<AnyObject>()];
        }
        self.schedule_capture(sel!(resumePersonalInputs:), delay);
    }
    pub(super) fn sync_window_shortcut(&self) {
        self.ivars().window_resume_attempts.set(0);
        if self.ivars().personal_enabled.get()
            && !self.ivars().shortcuts.borrow().reserves_window_tab()
        {
            self.resume_personal_inputs();
        } else {
            self.ivars().window_hotkey.borrow_mut().take();
            self.ivars().window_cycle_queue.borrow_mut().clear();
            self.ivars().window_cycle.borrow_mut().finish();
        }
    }
    pub(super) fn resume_personal_inputs(&self) {
        if self.ivars().inputs_suspended.get() {
            return;
        }
        if self.ivars().picking.get() {
            if self.ivars().runtime_settings.borrow().gestures_enabled
                || self.ivars().runtime_settings.borrow().window_switcher
            {
                self.schedule_inputs_resume(1.0);
            }
            return;
        }
        if self.ivars().personal_enabled.get()
            && !self.ivars().shortcuts.borrow().reserves_window_tab()
            && self.ivars().runtime_settings.borrow().window_switcher
            && self
                .ivars()
                .window_hotkey
                .borrow()
                .as_ref()
                .is_none_or(|tap| !tap.active())
            && self.ivars().window_resume_attempts.get() < 3
        {
            self.ivars()
                .window_resume_attempts
                .set(self.ivars().window_resume_attempts.get() + 1);
            self.ivars().window_hotkey.borrow_mut().take();
            match crate::window_hotkey::WindowHotkey::start() {
                Ok(tap) => {
                    self.ivars().window_hotkey.replace(Some(tap));
                }
                Err(message) => {
                    self.status(&message);
                    if self.ivars().window_resume_attempts.get() < 3 {
                        self.schedule_inputs_resume(2.0);
                    }
                }
            }
        }
        if self.ivars().runtime_settings.borrow().gestures_enabled && !self.gestures_active() {
            if self.ivars().input_resume_attempts.get() >= 3 {
                return;
            }
            self.ivars()
                .input_resume_attempts
                .set(self.ivars().input_resume_attempts.get() + 1);
            let result = Tap::configured(&self.ivars().runtime_settings.borrow(), false);
            match result {
                Ok(tap) => {
                    self.stop_tap();
                    self.ivars().tap.replace(Some(tap));
                    self.begin_trackpad_poll();
                }
                Err(message) => {
                    self.status(&message);
                    if self.ivars().input_resume_attempts.get() < 3 {
                        self.schedule_inputs_resume(2.0);
                    }
                }
            }
        }
    }
    pub(super) fn apply_startup_workspace(&self) {
        if !self.ivars().startup_pending.replace(false) || self.ivars().picking.get() {
            return;
        }
        match windows::teams_has_meeting(&self.ivars().teams_settings.borrow()) {
            Ok(true) => self.apply_teams_now(),
            Ok(false) => {
                let result = self.open_application(applications::CHROME, true, Instant::now());
                self.status(&result.unwrap_or_else(|e| e));
            }
            Err(message) => self.status(&format!("Démarrage automatique suspendu : {message}")),
        }
    }
    pub(super) fn apply_teams_now(&self) {
        if self.ivars().picking.get() || self.ivars().layout_snapshot.borrow().is_some() {
            self.status("Terminez la disposition ou le dialogue en cours.");
            return;
        }
        self.cancel_open();
        self.cancel_gesture_dispatch();
        let now = self
            .ivars()
            .teams_started
            .get_or_init(Instant::now)
            .elapsed()
            .as_millis() as u64;
        let result = self.ivars().teams_automation.borrow_mut().apply_now(
            self.mtm(),
            &self.ivars().teams_settings.borrow(),
            now,
        );
        self.status(&result.unwrap_or_else(|e| e));
        if self.ivars().teams_automation.borrow().tracking() {
            unsafe {
                let _: () = msg_send![objc2::class!(NSObject),cancelPreviousPerformRequestsWithTarget:self,selector:sel!(teamsPoll:),object:std::ptr::null::<AnyObject>()];
            }
            self.schedule_capture(sel!(teamsPoll:), 1.0);
        }
    }
    pub(super) fn place_right_half(&self) {
        self.cancel_open();
        match windows::MaximizeRequest::start(self.mtm(), None) {
            Ok(request) => {
                self.ivars().pending_maximize.replace(Some(PendingMaximize {
                    request: request.into_right_half(),
                    started: Instant::now(),
                    activation_started: None,
                }));
                if let Some(result) = self.advance_maximize() {
                    self.status(&result.unwrap_or_else(|e| e));
                } else {
                    self.schedule_maximize_poll();
                }
            }
            Err(message) => self.status(&message),
        }
    }
    pub(super) fn cycle_windows(&self, all: bool) {
        if self.ivars().picking.get() {
            return;
        }
        let result = self.ivars().window_cycle.borrow_mut().cycle(all, false);
        self.status(&result.unwrap_or_else(|e| e));
    }
    pub(super) fn cycle_tab(&self, previous: bool) {
        if self.ivars().picking.get() {
            return;
        }
        let result = windows::cycle_tab(previous);
        self.status(&result.unwrap_or_else(|e| e));
    }
    pub(super) fn studio_save_runtime(
        &self,
        input: &crate::studio::system::RuntimeSave,
    ) -> Result<Value, Failure> {
        let revision = format!(
            "{}:{}",
            std::process::id(),
            self.ivars().runtime_revision.get()
        );
        if revision != input.expected_revision {
            return Err(Failure::new(
                "STALE_REVISION",
                "Réglages modifiés. Actualisez avant de réessayer.",
            ));
        }
        if input.settings.window_switcher && self.ivars().shortcuts.borrow().reserves_window_tab() {
            return Err(Failure::invalid("Un raccourci utilise déjà ⌘Tab ou ⌘⇧Tab. Modifiez-le avant d’activer la bascule entre fenêtres."));
        }
        if input.settings.gestures_enabled {
            for target in [&input.settings.left, &input.settings.right] {
                target
                    .resolve(&self.ivars().shortcuts.borrow())
                    .map_err(Failure::invalid)?;
                if let crate::runtime_settings::GestureTarget::Profile { profile_id } = target {
                    if !self
                        .ivars()
                        .catalog
                        .borrow()
                        .profiles
                        .iter()
                        .any(|p| p.id == *profile_id)
                    {
                        return Err(Failure::invalid(
                            "Un espace associé au geste n’existe plus.",
                        ));
                    }
                }
            }
        }
        self.save_runtime_settings(input.settings.clone())
            .map_err(Failure::invalid)?;
        self.stop_tap();
        self.ivars().window_hotkey.borrow_mut().take();
        self.ivars().window_cycle_queue.borrow_mut().clear();
        self.ivars().window_cycle.borrow_mut().finish();
        self.ivars().input_resume_attempts.set(0);
        self.ivars().window_resume_attempts.set(0);
        if input.settings.gestures_enabled && !Tap::input_trusted() {
            self.set_gestures_requested(true, true);
        }
        self.resume_personal_inputs();
        Ok(self.studio_runtime_data())
    }
}
