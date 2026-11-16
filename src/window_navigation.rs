//! User-triggered window cycling; AX window titles stay in this in-memory list.
use super::*;
#[derive(Default)]
pub struct WindowCycle {
    choices: Vec<Choice>,
    index: usize,
    last: Option<Instant>,
    all: bool,
    apps: Vec<i32>,
}
impl WindowCycle {
    pub fn note_app(&mut self, pid: i32) {
        self.apps.retain(|p| *p != pid);
        self.apps.insert(0, pid);
        self.apps.truncate(64);
    }
    pub fn finish(&mut self) {
        self.last = None;
        self.choices.clear();
    }
    pub fn cycle(&mut self, all: bool, reverse: bool) -> Result<String, String> {
        self.cycle_steps(all, reverse, 1)
    }
    pub fn cycle_steps(
        &mut self,
        all: bool,
        reverse: bool,
        steps: usize,
    ) -> Result<String, String> {
        require_permission()?;
        let current = NSWorkspace::sharedWorkspace()
            .frontmostApplication()
            .filter(|app| app.processIdentifier() != std::process::id() as i32);
        if !all && current.is_none() {
            return Err(
                "Activez d’abord une autre application pour parcourir ses fenêtres.".into(),
            );
        }
        let rebuild = self
            .last
            .is_none_or(|t| t.elapsed() > Duration::from_millis(1800))
            || self.all != all;
        if rebuild {
            self.choices.clear();
            self.index = 0;
            self.all = all;
            let focused = current.as_ref().and_then(|app| preferred_window(app).ok());
            let mut found_focus = false;
            let fixture_scope = std::env::args().any(|a| a == "--navigation-check")
                && std::env::var_os("KIGI_TEST_NAVIGATION").is_some();
            let mut apps: Vec<_> = NSWorkspace::sharedWorkspace()
                .runningApplications()
                .iter()
                .filter(|a| {
                    (!fixture_scope
                        || a.bundleIdentifier().is_some_and(|b| {
                            matches!(
                                b.to_string().as_str(),
                                "io.kigi.focus.source" | "io.kigi.focus.target"
                            )
                        }))
                        && a.processIdentifier() != std::process::id() as i32
                        && a.activationPolicy()
                            == objc2_app_kit::NSApplicationActivationPolicy::Regular
                        && (all
                            || current.as_ref().is_some_and(|current| {
                                a.processIdentifier() == current.processIdentifier()
                            }))
                })
                .collect();
            apps.sort_by_key(|a| {
                if current
                    .as_ref()
                    .is_some_and(|current| a.processIdentifier() == current.processIdentifier())
                {
                    0
                } else {
                    self.apps
                        .iter()
                        .position(|p| *p == a.processIdentifier())
                        .map(|i| i + 1)
                        .unwrap_or(100)
                }
            });
            let started = Instant::now();
            for app in apps {
                if started.elapsed() > Duration::from_secs(2) {
                    break;
                }
                let Ok(array) = Element::app(app.processIdentifier()).attribute("AXWindows") else {
                    continue;
                };
                for i in 0..unsafe { CFArrayGetCount(array.0) }.min(32) {
                    if self.choices.len() >= 64 || started.elapsed() > Duration::from_secs(2) {
                        break;
                    }
                    let window = Element(unsafe { CFRetain(CFArrayGetValueAtIndex(array.0, i)) });
                    if !window.is_standard() {
                        continue;
                    }
                    let is_focused = current.as_ref().is_some_and(|current| {
                        app.processIdentifier() == current.processIdentifier()
                    }) && focused
                        .as_ref()
                        .is_some_and(|f| unsafe { CFEqual(f.0, window.0) != 0 });
                    let choice = Choice {
                        label: String::new(),
                        bundle_id: String::new(),
                        window,
                        app: app.clone(),
                    };
                    if is_focused {
                        found_focus = true;
                        self.choices.insert(0, choice);
                    } else {
                        self.choices.push(choice);
                    }
                }
            }
            if all {
                self.choices.sort_by_key(|choice| {
                    if focused
                        .as_ref()
                        .is_some_and(|focused| unsafe { CFEqual(focused.0, choice.window.0) != 0 })
                    {
                        0
                    } else if current.as_ref().is_some_and(|app| {
                        app.processIdentifier() == choice.app.processIdentifier()
                    }) {
                        1000
                    } else {
                        self.apps
                            .iter()
                            .position(|pid| *pid == choice.app.processIdentifier())
                            .map(|index| index + 1)
                            .unwrap_or(999)
                    }
                });
            }
            if !found_focus && !self.choices.is_empty() {
                self.index = if reverse { 0 } else { self.choices.len() - 1 };
            }
        }
        if self.choices.is_empty() || (self.choices.len() < 2 && current.is_some()) {
            self.finish();
            return Err("Aucune autre fenêtre accessible à parcourir.".into());
        }
        self.index = if reverse {
            (self.index + self.choices.len() - steps % self.choices.len()) % self.choices.len()
        } else {
            (self.index + steps) % self.choices.len()
        };
        let choice = &self.choices[self.index];
        diagnostics::record(
            "window.cycle.selection",
            format!(
                "all={all} index={} count={} pid={}",
                self.index,
                self.choices.len(),
                choice.app.processIdentifier()
            ),
        );
        choice.window.unminimize()?;
        let _ = crate::applications::request_activation(&choice.app);
        focus_window(&choice.app, &choice.window)?;
        self.last = Some(Instant::now());
        Ok(format!(
            "Activation de la fenêtre {} / {} demandée.",
            self.index + 1,
            self.choices.len()
        ))
    }
}
/// Explicit Ctrl+Tab adapter for the frontmost app. No key monitoring or content reads.
pub fn cycle_tab(previous: bool) -> Result<String, String> {
    require_permission()?;
    let app = frontmost()?;
    let pid = app.processIdentifier();
    #[link(name = "ApplicationServices", kind = "framework")]
    extern "C" {
        fn CGEventCreateKeyboardEvent(source: Ref, key: u16, down: bool) -> Ref;
        fn CGEventSetFlags(event: Ref, flags: u64);
        fn CGEventPostToPid(pid: i32, event: Ref);
    }
    for down in [true, false] {
        if NSWorkspace::sharedWorkspace()
            .frontmostApplication()
            .is_none_or(|a| a.processIdentifier() != pid)
        {
            return Err("Application active changée : commande d’onglet annulée.".into());
        }
        let event = unsafe { CGEventCreateKeyboardEvent(ptr::null(), 48, down) };
        if event.is_null() {
            return Err("Impossible de créer la commande d’onglet.".into());
        }
        unsafe {
            CGEventSetFlags(event, (1 << 18) | if previous { 1 << 17 } else { 0 });
            CGEventPostToPid(pid, event);
            CFRelease(event);
        }
    }
    Ok("Commande Ctrl+Tab transmise à l’application active. Son comportement dépend de l’application.".into())
}

pub fn navigation_fixture_check(mtm: MainThreadMarker) -> Result<(), String> {
    if std::env::var_os("KIGI_TEST_NAVIGATION").is_none() {
        return Err("Fixture dédiée requise.".into());
    }
    require_permission()?;
    let source =
        crate::applications::running("io.kigi.focus.source").ok_or("Fixture source absente.")?;
    let target =
        crate::applications::running("io.kigi.focus.target").ok_or("Fixture cible absente.")?;
    let before = preferred_window(&source)?;
    let _ = crate::applications::request_activation(&source);
    focus_window(&source, &before)?;
    #[link(name = "CoreFoundation", kind = "framework")]
    extern "C" {
        fn CFRunLoopRunInMode(mode: Ref, seconds: f64, return_after_source: bool) -> i32;
        static kCFRunLoopDefaultMode: Ref;
    }
    let pump = || unsafe {
        CFRunLoopRunInMode(kCFRunLoopDefaultMode, 0.03, false);
    };
    let wait = |pid| {
        for _ in 0..30 {
            if NSWorkspace::sharedWorkspace()
                .frontmostApplication()
                .is_some_and(|a| a.processIdentifier() == pid)
            {
                return true;
            }
            pump();
        }
        false
    };
    if !wait(source.processIdentifier()) {
        return Err("Source non activée.".into());
    }
    let mut cycle = WindowCycle::default();
    cycle.note_app(target.processIdentifier());
    cycle.note_app(source.processIdentifier());
    cycle.cycle(false, false)?;
    let after = preferred_window(&source)?;
    if unsafe { CFEqual(before.0, after.0) } != 0 {
        return Err("La bascule dans la même application n’a pas changé de fenêtre.".into());
    }
    cycle.finish();
    let mut target_seen = false;
    for _ in 0..4 {
        cycle.cycle(true, false)?;
        if wait(target.processIdentifier()) {
            target_seen = true;
            break;
        }
    }
    if !target_seen {
        return Err("La bascule globale n’a pas atteint l’autre application.".into());
    }
    let window = preferred_window(&source)?;
    let original = window.frame()?;
    let _ = crate::applications::request_activation(&source);
    focus_window(&source, &window)?;
    if !wait(source.processIdentifier()) {
        return Err("Source non réactivée.".into());
    }
    let mut request = MaximizeRequest::start(mtm, Some(&source))?.into_right_half();
    let mut done = false;
    for _ in 0..30 {
        if request.poll(mtm)?.is_some() {
            done = true;
            break;
        }
        std::thread::sleep(Duration::from_millis(30));
    }
    if !done
        || !window
            .frame()?
            .approximately(visible_frame(mtm, original)?.right_half())
    {
        return Err("Moitié droite directe incorrecte.".into());
    }
    window.set_frame(original)?;
    Ok(())
}
