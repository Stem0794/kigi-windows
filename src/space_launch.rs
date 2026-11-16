//! Bounded, cancelable launch of missing profile applications, before any AX placement.
use super::*;
struct Opening {
    receiver: Receiver<Result<i32, isize>>,
    pid: Option<i32>,
    ready: bool,
}
pub(super) struct SpaceLaunch {
    apps: Vec<Opening>,
    started: Instant,
    error: Option<String>,
}
impl Controller {
    pub(super) fn launch_profile_apps(
        &self,
        layout: &config::SavedLayout,
    ) -> Result<(), studio::bridge::Failure> {
        use studio::bridge::Failure;
        if !self.ivars().runtime_settings.borrow().launch_missing_apps {
            return Ok(());
        }
        let mut bundles = std::collections::HashSet::new();
        let mut apps = Vec::new();
        for window in &layout.windows {
            if bundles.insert(window.bundle_id.clone())
                && applications::running(&window.bundle_id).is_none()
            {
                apps.push(Opening {
                    receiver: applications::open(&window.bundle_id).map_err(Failure::invalid)?,
                    pid: None,
                    ready: false,
                });
            }
        }
        if apps.is_empty() {
            return Ok(());
        }
        self.ivars().space_launch.replace(Some(SpaceLaunch {
            apps,
            started: Instant::now(),
            error: None,
        }));
        let alert = NSAlert::new(self.mtm());
        alert.setMessageText(ns_string!("Ouverture des applications de l’espace…"));
        alert.setInformativeText(ns_string!("Attente limitée à8 secondes. Aucune fenêtre ne sera déplacée avant leur disponibilité. Annuler conserve les applications déjà ouvertes."));
        alert.addButtonWithTitle(ns_string!("Annuler"));
        #[allow(deprecated)]
        NSApplication::sharedApplication(self.mtm()).activateIgnoringOtherApps(true);
        self.schedule_capture(sel!(spaceLaunchPoll:), 0.1);
        let response = alert.runModal();
        unsafe {
            let _: () = msg_send![objc2::class!(NSObject),cancelPreviousPerformRequestsWithTarget:self,selector:sel!(spaceLaunchPoll:),object:std::ptr::null::<AnyObject>()];
        }
        let launch = self
            .ivars()
            .space_launch
            .borrow_mut()
            .take()
            .ok_or_else(|| Failure::invalid("Lancement interrompu."))?;
        if let Some(error) = launch.error {
            return Err(Failure::new("LAUNCH_FAILED", error));
        }
        if response != NSModalResponseOK || !launch.apps.iter().all(|app| app.ready) {
            return Err(Failure::new("CANCELED","Ouverture annulée ; aucune fenêtre déplacée. Les applications déjà ouvertes sont conservées."));
        }
        Ok(())
    }
    pub(super) fn poll_space_launch(&self) {
        let mut pending = self.ivars().space_launch.borrow_mut();
        let Some(launch) = pending.as_mut() else {
            return;
        };
        if !windows::trusted() {
            launch.error = Some("Accessibilité révoquée pendant le lancement.".into());
        }
        for app in &mut launch.apps {
            if app.pid.is_none() {
                match app.receiver.try_recv() {
                    Ok(Ok(pid)) => app.pid = Some(pid),
                    Ok(Err(code)) => {
                        launch.error = Some(format!(
                            "macOS refuse une application de l’espace (code{code})."
                        ));
                    }
                    Err(TryRecvError::Disconnected) => {
                        launch.error = Some("Lancement interrompu.".into());
                    }
                    Err(TryRecvError::Empty) => {}
                }
            }
            if let Some(pid) = app.pid {
                app.ready = NSRunningApplication::runningApplicationWithProcessIdentifier(pid)
                    .is_some_and(|app| windows::has_standard_window(&app));
            }
        }
        if launch.started.elapsed() > Duration::from_secs(8) {
            launch.error=Some("Une application ne fournit pas de fenêtre accessible après8 secondes. Ouvrez sa fenêtre puis réessayez.".into());
        }
        let done = launch.error.is_some() || launch.apps.iter().all(|app| app.ready);
        drop(pending);
        if done {
            NSApplication::sharedApplication(self.mtm()).stopModalWithCode(NSModalResponseOK);
        } else {
            self.schedule_capture(sel!(spaceLaunchPoll:), 0.1);
        }
    }
}
