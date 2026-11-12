//! Small owned CoreFoundation wrapper around the public Accessibility C API.
use crate::{diagnostics, geometry::Rect};
use objc2::MainThreadMarker;
use objc2_app_kit::{NSRunningApplication, NSScreen, NSWorkspace};
use objc2_foundation::NSString;
use std::{
    ffi::c_void,
    ptr,
    time::{Duration, Instant},
};

type Ref = *const c_void;
#[link(name = "ApplicationServices", kind = "framework")]
extern "C" {
    fn AXIsProcessTrusted() -> u8;
    fn AXIsProcessTrustedWithOptions(options: Ref) -> u8;
    static kAXTrustedCheckOptionPrompt: Ref;
    fn AXUIElementCreateApplication(pid: i32) -> Ref;
    fn AXUIElementSetMessagingTimeout(element: Ref, timeout: f32) -> i32;
    fn AXUIElementCopyAttributeValue(element: Ref, name: Ref, value: *mut Ref) -> i32;
    fn AXUIElementIsAttributeSettable(element: Ref, name: Ref, result: *mut u8) -> i32;
    fn AXUIElementSetAttributeValue(element: Ref, name: Ref, value: Ref) -> i32;
    fn AXUIElementPerformAction(element: Ref, action: Ref) -> i32;
    fn AXValueCreate(kind: u32, value: *const c_void) -> Ref;
    fn AXValueGetValue(value: Ref, kind: u32, result: *mut c_void) -> u8;
}
#[link(name = "CoreFoundation", kind = "framework")]
extern "C" {
    fn CFRelease(value: Ref);
    fn CFRetain(value: Ref) -> Ref;
    fn CFArrayGetCount(array: Ref) -> isize;
    fn CFArrayGetValueAtIndex(array: Ref, index: isize) -> Ref;
    fn CFDictionaryCreate(
        allocator: Ref,
        keys: *const Ref,
        values: *const Ref,
        count: isize,
        key_callbacks: Ref,
        value_callbacks: Ref,
    ) -> Ref;
    fn CFBooleanGetValue(value: Ref) -> u8;
    static kCFBooleanTrue: Ref;
    static kCFBooleanFalse: Ref;
    fn CFGetTypeID(value: Ref) -> usize;
    fn CFStringGetTypeID() -> usize;
}

pub struct Element(Ref);
impl Drop for Element {
    fn drop(&mut self) {
        unsafe { CFRelease(self.0) }
    }
}
impl Element {
    fn app(pid: i32) -> Self {
        // Create rule: retained, non-null AX application even if PID later exits.
        let this = Self(unsafe { AXUIElementCreateApplication(pid) });
        unsafe {
            AXUIElementSetMessagingTimeout(this.0, 0.25);
        }
        this
    }
    fn attribute(&self, name: &str) -> Result<Self, String> {
        // Apply the bound to this exact AX element, including returned windows.
        check(unsafe { AXUIElementSetMessagingTimeout(self.0, 0.25) })?;
        let key = NSString::from_str(name);
        let mut value = ptr::null();
        let code = unsafe { AXUIElementCopyAttributeValue(self.0, name_ref(&key), &mut value) };
        if code != 0 {
            diagnostics::record("ax.read_failed", format!("attribute={name} code={code}"));
        }
        check(code)?;
        if value.is_null() {
            return Err("Attribut AX absent.".into());
        }
        Ok(Self(value))
    }
    fn can_set(&self, name: &str) -> Result<(), String> {
        let mut settable = 0;
        let key = NSString::from_str(name);
        check(unsafe { AXUIElementIsAttributeSettable(self.0, name_ref(&key), &mut settable) })?;
        if settable == 0 {
            Err(format!("{name} n'est pas modifiable pour cette fenêtre."))
        } else {
            Ok(())
        }
    }
    fn pair(&self, name: &str, kind: u32) -> Result<[f64; 2], String> {
        let value = self.attribute(name)?;
        let mut pair = [0.0; 2];
        if unsafe { AXValueGetValue(value.0, kind, pair.as_mut_ptr().cast()) } != 0 {
            Ok(pair)
        } else {
            Err(format!("Type AX inattendu : {name}."))
        }
    }
    fn set_pair(&self, name: &str, kind: u32, pair: [f64; 2]) -> Result<(), String> {
        // CGPoint / CGSize are two CGFloat (f64 on both supported 64-bit targets).
        let raw = unsafe { AXValueCreate(kind, pair.as_ptr().cast()) };
        if raw.is_null() {
            return Err("Impossible de créer AXValue.".into());
        }
        let value = Self(raw);
        let key = NSString::from_str(name);
        check(unsafe { AXUIElementSetAttributeValue(self.0, name_ref(&key), value.0) })
    }
    fn frame(&self) -> Result<Rect, String> {
        let [x, y] = self.pair("AXPosition", 1)?;
        let [width, height] = self.pair("AXSize", 2)?;
        Ok(Rect {
            x,
            y,
            width,
            height,
        })
    }
    fn ensure_normal(&self) -> Result<(), String> {
        for name in ["AXMinimized", "AXFullScreen"] {
            if let Ok(value) = self.attribute(name) {
                if unsafe { CFBooleanGetValue(value.0) } != 0 {
                    return Err("Test refusé : fenêtre minimisée ou en plein écran natif.".into());
                }
            }
        }
        Ok(())
    }
    fn set_frame(&self, frame: Rect) -> Result<Rect, String> {
        require_permission()?;
        self.ensure_normal()?;
        diagnostics::record(
            "ax.frame.request",
            format!("target={frame:?} order=position,size,position"),
        );
        self.can_set("AXPosition")?;
        self.can_set("AXSize")?;
        // Anchor before expanding: resizing at the right edge can be clamped
        // to the remaining display width. Reapply after sizing because moving
        // a still-large window to the right can itself be clamped.
        self.set_pair("AXPosition", 1, [frame.x, frame.y])?;
        self.set_pair("AXSize", 2, [frame.width, frame.height])?;
        self.set_pair("AXPosition", 1, [frame.x, frame.y])?;
        let actual = self.frame()?;
        diagnostics::record(
            "ax.frame.readback",
            format!(
                "actual={actual:?} within_tolerance={}",
                actual.approximately(frame)
            ),
        );
        Ok(actual)
    }
}
fn name_ref(s: &NSString) -> Ref {
    (s as *const NSString).cast()
}
fn check(code: i32) -> Result<(), String> {
    if code != 0 {
        diagnostics::record("ax.error", format!("code={code}"));
    }
    match code {
        0 => Ok(()),
        -25211 => Err(
            "Accessibilité refusée ou révoquée. Autorisez Kigi Windows dans Réglages Système."
                .into(),
        ),
        -25202 => Err("La fenêtre n'existe plus (AX).".into()),
        -25204 => Err("L'application ne répond pas à AX (délai limité à 250 ms par appel).".into()),
        other => Err(format!(
            "Opération AX non prise en charge ou refusée ({other})."
        )),
    }
}
pub fn trusted() -> bool {
    // Apple Boolean is UInt8, not a Rust/C _Bool. Accept any non-zero value.
    unsafe { AXIsProcessTrusted() != 0 }
}
pub fn request_permission() -> bool {
    // Dictionary lives only for this synchronous call. Static CF strings/bools
    // need no retain callbacks; never mutate TCC directly.
    unsafe {
        let keys = [kAXTrustedCheckOptionPrompt];
        let values = [kCFBooleanTrue];
        let dictionary = CFDictionaryCreate(
            ptr::null(),
            keys.as_ptr(),
            values.as_ptr(),
            1,
            ptr::null(),
            ptr::null(),
        );
        if dictionary.is_null() {
            return false;
        }
        let options = Element(dictionary);
        AXIsProcessTrustedWithOptions(options.0) != 0
    }
}
/// Explicit diagnostic only: no titles/content, no writes and no TCC changes.
pub fn permission_probe() -> String {
    let raw = unsafe { AXIsProcessTrusted() };
    let raw_options = unsafe { AXIsProcessTrustedWithOptions(ptr::null()) };
    let workspace = NSWorkspace::sharedWorkspace();
    let own_pid = std::process::id() as i32;
    let app = workspace
        .frontmostApplication()
        .filter(|a| a.processIdentifier() != own_pid)
        .or_else(|| {
            workspace.runningApplications().iter().find(|a| {
                a.processIdentifier() != own_pid
                    && a.bundleIdentifier()
                        .is_some_and(|b| b.to_string() == "com.google.Chrome")
            })
        });
    let Some(app) = app else {
        return format!(
            "trusted_raw={raw} trusted_options_raw={raw_options} probe=no_foreign_application"
        );
    };
    let target = Element::app(app.processIdentifier());
    let key = NSString::from_str("AXWindows");
    let mut value = ptr::null();
    let code = unsafe { AXUIElementCopyAttributeValue(target.0, name_ref(&key), &mut value) };
    let windows = if code == 0 && !value.is_null() {
        let array = Element(value);
        Some(unsafe { CFArrayGetCount(array.0) })
    } else {
        None
    };
    let result = format!("trusted_raw={raw} trusted_options_raw={raw_options} target_bundle={} AXWindows_code={code} window_count={windows:?}",
        app.bundleIdentifier().map(|b| b.to_string()).unwrap_or_else(|| "unknown".into()));
    diagnostics::record("permission.ax.probe", &result);
    result
}

fn require_permission() -> Result<(), String> {
    if trusted() {
        Ok(())
    } else {
        Err("Autorisez d'abord l'accès Accessibilité depuis le menu Kigi.".into())
    }
}

pub fn inventory() -> Result<String, String> {
    require_permission()?;
    let workspace = NSWorkspace::sharedWorkspace();
    let mut rows = Vec::new();
    let started = Instant::now();
    // On-demand only. No titles/content read; cap the UI and total scan.
    for app in workspace
        .runningApplications()
        .iter()
        .filter(|a| a.bundleIdentifier().is_some())
        .take(64)
    {
        if started.elapsed() > Duration::from_secs(2) || rows.len() >= 24 {
            rows.push("Inventaire partiel : limite de temps ou d'affichage atteinte.".into());
            break;
        }
        let bundle = app.bundleIdentifier().unwrap().to_string();
        let element = Element::app(app.processIdentifier());
        match element.attribute("AXWindows") {
            Ok(windows) => {
                let count = unsafe { CFArrayGetCount(windows.0) };
                if count > 0 {
                    rows.push(format!("{bundle} : {count} fenêtre(s)"));
                    // Demonstrate frame reads without exposing titles.
                    let first = unsafe { CFArrayGetValueAtIndex(windows.0, 0) };
                    let window = Element(unsafe { CFRetain(first) });
                    if let Ok(r) = window.frame() {
                        rows.push(format!(
                            "  x={:.0}, y={:.0}, {:.0} × {:.0} pt",
                            r.x, r.y, r.width, r.height
                        ));
                    }
                }
            }
            Err(_) => rows.push(format!("{bundle} : AX indisponible")),
        }
    }
    if rows.is_empty() {
        Ok("Aucune fenêtre AX détectée.".into())
    } else {
        Ok(rows.join("\n"))
    }
}

pub struct Snapshot {
    window: Element,
    frame: Rect,
}
impl Snapshot {
    pub fn restore(&self) -> Result<String, String> {
        let actual = self.window.set_frame(self.frame)?;
        let message = result_message(actual, self.frame, "restaurée");
        if actual.approximately(self.frame) {
            Ok(message)
        } else {
            Err(message)
        }
    }
}
fn result_message(actual: Rect, target: Rect, action: &str) -> String {
    if actual.approximately(target) {
        format!("Fenêtre {action} (position relue via AX).")
    } else {
        format!("Application partielle : rectangle obtenu {:.0}, {:.0}, {:.0} × {:.0} pt. L'application impose peut-être une taille minimale.",actual.x,actual.y,actual.width,actual.height)
    }
}
pub fn move_frontmost(
    mtm: MainThreadMarker,
    snapshot: &mut Option<Snapshot>,
) -> Result<String, String> {
    require_permission()?;
    let app = NSWorkspace::sharedWorkspace()
        .frontmostApplication()
        .ok_or("Aucune application active.")?;
    diagnostics::record(
        "window.target",
        format!(
            "bundle={} pid={}",
            app.bundleIdentifier()
                .map(|b| b.to_string())
                .unwrap_or_else(|| "unknown".into()),
            app.processIdentifier()
        ),
    );
    if app.processIdentifier() == std::process::id() as i32 {
        return Err("Activez une fenêtre d'une autre application avant le test.".into());
    }
    let window = Element::app(app.processIdentifier()).attribute("AXFocusedWindow")?;
    window.ensure_normal()?;
    let frame = window.frame()?;
    diagnostics::record("ax.frame.before", format!("frame={frame:?}"));
    let target = visible_frame(mtm, frame)?.right_half();
    // Save BEFORE either write: partial writes and errors must stay restorable.
    *snapshot = Some(Snapshot { window, frame });
    let actual = snapshot.as_ref().unwrap().window.set_frame(target)?;
    Ok(result_message(actual, target, "déplacée à droite"))
}

/// Screen with greatest overlap, preserving AX/AppKit conversion on all displays.
pub fn visible_frame(mtm: MainThreadMarker, frame: Rect) -> Result<Rect, String> {
    let screens = NSScreen::screens(mtm);
    let primary = screens
        .firstObject()
        .ok_or("Aucun écran disponible.")?
        .frame();
    let primary_top = primary.origin.y + primary.size.height;
    let screen = screens
        .iter()
        .max_by(|a, b| {
            let rect = |s: &NSScreen| {
                let r = s.frame();
                Rect::from_appkit(
                    r.origin.x,
                    r.origin.y,
                    r.size.width,
                    r.size.height,
                    primary_top,
                )
            };
            frame.overlap(rect(a)).total_cmp(&frame.overlap(rect(b)))
        })
        .ok_or("Aucun écran disponible.")?;
    let screen = if frame.overlap({
        let r = screen.frame();
        Rect::from_appkit(
            r.origin.x,
            r.origin.y,
            r.size.width,
            r.size.height,
            primary_top,
        )
    }) > 0.0
    {
        screen
    } else {
        screens.firstObject().unwrap()
    };
    let v = screen.visibleFrame();
    Ok(Rect::from_appkit(
        v.origin.x,
        v.origin.y,
        v.size.width,
        v.size.height,
        primary_top,
    ))
}

impl Element {
    fn text(&self, name: &str) -> Option<String> {
        let value = self.attribute(name).ok()?;
        if unsafe { CFGetTypeID(value.0) != CFStringGetTypeID() } {
            return None;
        }
        // NSString/CFString are toll-free bridged; value keeps the string alive.
        Some(unsafe { &*value.0.cast::<NSString>() }.to_string())
    }
    fn is_standard(&self) -> bool {
        self.text("AXSubrole")
            .is_some_and(|s| s == "AXStandardWindow")
    }
    fn unminimize(&self) -> Result<(), String> {
        let value = self.attribute("AXMinimized")?;
        if unsafe { CFBooleanGetValue(value.0) } != 0 {
            let key = NSString::from_str("AXMinimized");
            check(unsafe {
                AXUIElementSetAttributeValue(self.0, name_ref(&key), kCFBooleanFalse)
            })?;
        }
        Ok(())
    }
    fn raise(&self) -> Result<(), String> {
        let action = NSString::from_str("AXRaise");
        check(unsafe { AXUIElementPerformAction(self.0, name_ref(&action)) })
    }
}
fn frontmost() -> Result<objc2::rc::Retained<NSRunningApplication>, String> {
    NSWorkspace::sharedWorkspace()
        .frontmostApplication()
        .filter(|a| a.processIdentifier() != std::process::id() as i32)
        .ok_or_else(|| "Activez d'abord une fenêtre d'une autre application.".into())
}
fn preferred_window(app: &NSRunningApplication) -> Result<Element, String> {
    let root = Element::app(app.processIdentifier());
    for name in ["AXFocusedWindow", "AXMainWindow"] {
        if let Ok(window) = root.attribute(name) {
            if window.is_standard() {
                return Ok(window);
            }
        }
    }
    let array = root.attribute("AXWindows")?;
    let count = unsafe { CFArrayGetCount(array.0) };
    let mut candidates = Vec::new();
    let started = Instant::now();
    for i in 0..count.min(64) {
        if started.elapsed() > Duration::from_secs(1) {
            break;
        }
        let window = Element(unsafe { CFRetain(CFArrayGetValueAtIndex(array.0, i)) });
        if window.is_standard() {
            candidates.push(window);
        }
    }
    if candidates.len() == 1 {
        Ok(candidates.remove(0))
    } else {
        Err(
            "Aucune fenêtre principale identifiable. Activez la fenêtre souhaitée et réessayez."
                .into(),
        )
    }
}
pub fn application_window_ready(app: &NSRunningApplication) -> bool {
    preferred_window(app).is_ok()
}
pub fn maximize(
    mtm: MainThreadMarker,
    app: Option<&NSRunningApplication>,
) -> Result<String, String> {
    require_permission()?;
    let foreground;
    let app = if let Some(app) = app {
        app
    } else {
        foreground = frontmost()?;
        &foreground
    };
    let window = preferred_window(app)?;
    window.unminimize()?;
    window.ensure_normal()?;
    let target = visible_frame(mtm, window.frame()?)?;
    let actual = window.set_frame(target)?;
    window.raise()?;
    Ok(result_message(actual, target, "maximisée"))
}

/// Titles exist only in memory for this explicit picker; never diagnostic logs.
pub struct Choice {
    pub label: String,
    window: Element,
    app: objc2::rc::Retained<NSRunningApplication>,
}
pub fn pair_candidates(mtm: MainThreadMarker) -> Result<(Rect, Vec<Choice>, bool), String> {
    require_permission()?;
    let active = frontmost()?;
    let anchor = preferred_window(&active)?.frame()?;
    let display = visible_frame(mtm, anchor)?; // Capture BEFORE the modal UI takes focus.
    let started = Instant::now();
    let mut choices = Vec::new();
    let mut partial = false;
    'apps: for app in NSWorkspace::sharedWorkspace().runningApplications().iter() {
        if started.elapsed() > Duration::from_secs(2) {
            partial = true;
            break;
        }
        if app.processIdentifier() == std::process::id() as i32
            || app
                .bundleIdentifier()
                .is_some_and(|b| b.to_string() == "org.hammerspoon.Hammerspoon")
        {
            continue;
        }
        if app.activationPolicy() != objc2_app_kit::NSApplicationActivationPolicy::Regular {
            continue;
        }
        let Ok(array) = Element::app(app.processIdentifier()).attribute("AXWindows") else {
            continue;
        };
        let count = unsafe { CFArrayGetCount(array.0) };
        for i in 0..count.min(64) {
            if started.elapsed() > Duration::from_secs(2) || choices.len() >= 64 {
                partial = true;
                break 'apps;
            }
            let window = Element(unsafe { CFRetain(CFArrayGetValueAtIndex(array.0, i)) });
            if !window.is_standard() || window.ensure_normal().is_err() {
                continue;
            }
            let title = window
                .text("AXTitle")
                .unwrap_or_else(|| "Sans titre".into());
            let name = app
                .localizedName()
                .map(|n| n.to_string())
                .unwrap_or_else(|| "Application".into());
            choices.push(Choice {
                label: format!("{name} — {title}"),
                window,
                app: app.clone(),
            });
        }
    }
    diagnostics::record(
        "picker.inventory",
        format!("count={} partial={partial}", choices.len()),
    );
    if choices.len() < 2 {
        return Err("Deux fenêtres standard accessibles sont nécessaires. Ouvrez-les, quittez le plein écran et déminimisez-les.".into());
    }
    Ok((display, choices, partial))
}
pub fn arrange_pair(
    display: Rect,
    left: &Choice,
    right: &Choice,
    focus_left: bool,
) -> Result<String, String> {
    // Preflight both before writing either; windows may still close after this check.
    left.window.ensure_normal()?;
    right.window.ensure_normal()?;
    for window in [&left.window, &right.window] {
        window.can_set("AXPosition")?;
        window.can_set("AXSize")?;
    }
    let first = left.window.set_frame(display.left_half());
    let second = right.window.set_frame(display.right_half());
    diagnostics::record(
        "action.pair.write",
        format!("left_ok={} right_ok={}", first.is_ok(), second.is_ok()),
    );
    let left_actual = first.map_err(|e| format!("Partage incomplet, gauche : {e}"))?;
    let right_actual = second.map_err(|e| format!("Partage incomplet, droite : {e}"))?;
    left.window.raise()?;
    right.window.raise()?;
    let focused = if focus_left { left } else { right };
    crate::applications::activate(&focused.app)?;
    focused.window.raise()?;
    if left_actual.approximately(display.left_half())
        && right_actual.approximately(display.right_half())
    {
        Ok("Deux fenêtres placées à 50 % sur l'écran choisi.".into())
    } else {
        Ok("Partage partiel : une application impose ses dimensions. Consultez les logs.".into())
    }
}


/// Manual P3 pilot: one in-memory return point for two already-open windows.
/// Geometry only; no window titles or session data are retained.
pub struct MeetingSnapshot {
    teams: LayoutFrame,
    chrome: LayoutFrame,
    prior_pid: Option<i32>,
}
struct LayoutFrame {
    window: Element,
    before: Rect,
    applied: Rect,
}
impl MeetingSnapshot {
    pub fn restore(&self) -> Result<String, String> {
        require_permission()?;
        let mut restored = 0;
        let mut skipped = 0;
        for part in [&self.teams, &self.chrome] {
            match part.window.frame() {
                Ok(current) if current.approximately(part.before) => {}
                Ok(current) if current.approximately(part.applied) => {
                    let actual = part.window.set_frame(part.before)?;
                    if !actual.approximately(part.before) {
                        return Err(
                            "Restauration partielle : l'application impose ses dimensions. Réessayez.".into()
                        );
                    }
                    restored += 1;
                }
                _ => skipped += 1, // closed or manually moved since scene applied
            }
        }
        if let Some(pid) = self.prior_pid {
            if let Some(app) =
                NSRunningApplication::runningApplicationWithProcessIdentifier(pid)
            {
                let _ = crate::applications::activate(&app);
            }
        }
        Ok(format!(
            "Disposition précédente : {restored} fenêtre(s) restaurée(s), {skipped} ignorée(s) car fermée(s) ou déplacée(s) manuellement."
        ))
    }
}

pub fn meeting_browser(
    mtm: MainThreadMarker,
    snapshot: &mut Option<MeetingSnapshot>,
) -> Result<String, String> {
    require_permission()?;
    if snapshot.is_some() {
        return Ok(
            "Scène réunion déjà active. Restaurez-la avant d'en créer une autre.".into()
        );
    }
    // This first pilot deliberately does not launch applications or guess
    // which Teams pop-out is a meeting. The user opens the wanted windows.
    let teams_bundle = crate::applications::teams_bundle()?;
    let teams_app = crate::applications::running(teams_bundle)
        .ok_or("Ouvrez Teams avec ⌥T avant d'activer la scène.")?;
    let chrome_app = crate::applications::running(crate::applications::CHROME)
        .ok_or("Ouvrez Chrome avec ⌥B avant d'activer la scène.")?;
    let teams = preferred_window(&teams_app)?;
    let chrome = preferred_window(&chrome_app)?;
    for window in [&teams, &chrome] {
        window.ensure_normal()?;
        window.can_set("AXPosition")?;
        window.can_set("AXSize")?;
    }
    let before_teams = teams.frame()?;
    let before_chrome = chrome.frame()?;
    let display = visible_frame(mtm, before_chrome)?;
    let left = display.left_half();
    let right = display.right_half();
    let prior_pid = NSWorkspace::sharedWorkspace()
        .frontmostApplication()
        .map(|app| app.processIdentifier());
    // Both windows and their positions are captured before any write.
    let teams_result = teams.set_frame(left);
    let chrome_result = chrome.set_frame(right);
    let applied_teams = teams.frame().unwrap_or(before_teams);
    let applied_chrome = chrome.frame().unwrap_or(before_chrome);
    diagnostics::record(
        "workspace.meeting.apply",
        format!(
            "teams_ok={} chrome_ok={} geometry_ok={}",
            teams_result.is_ok(),
            chrome_result.is_ok(),
            applied_teams.approximately(left) && applied_chrome.approximately(right)
        ),
    );
    *snapshot = Some(MeetingSnapshot {
        teams: LayoutFrame {
            window: teams,
            before: before_teams,
            applied: applied_teams,
        },
        chrome: LayoutFrame {
            window: chrome,
            before: before_chrome,
            applied: applied_chrome,
        },
        prior_pid,
    });
    if teams_result.is_err()
        || chrome_result.is_err()
        || !applied_teams.approximately(left)
        || !applied_chrome.approximately(right)
    {
        return Err(
            "Placement incomplet. Utilisez « Restaurer la disposition » pour revenir à l'état précédent.".into()
        );
    }
    crate::applications::activate(&chrome_app)?;
    // The Chrome half cannot cover the Teams half.
    snapshot.as_ref().unwrap().chrome.window.raise()?;
    Ok("Réunion : Teams à gauche et Chrome à droite, 50/50.".into())
}

pub fn work_browser(mtm: MainThreadMarker) -> Result<String, String> {
    let chrome_app = crate::applications::running(crate::applications::CHROME)
        .ok_or("Ouvrez Chrome avec ⌥B avant d'activer Travail.")?;
    require_permission()?;
    crate::applications::activate(&chrome_app)?;
    maximize(mtm, Some(&chrome_app))?;
    Ok("Travail : Chrome au premier plan à 100 %, Teams conservé derrière.".into())
}
