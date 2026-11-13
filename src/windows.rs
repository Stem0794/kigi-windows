//! Small owned CoreFoundation wrapper around the public Accessibility C API.
use crate::{
    config::{SavedLayout, SavedWindow},
    diagnostics,
    geometry::{restore_decision, NormalizedRect, Rect, RestoreDecision, Settling},
};
use objc2::{MainThreadMarker, Message};
use objc2_app_kit::{
    NSApplication, NSApplicationPresentationOptions, NSRunningApplication, NSScreen, NSWorkspace,
};
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
    fn CFEqual(a: Ref, b: Ref) -> u8;
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
        let started = Instant::now();
        let result = self.set_frame_inner(frame);
        diagnostics::timing(
            diagnostics::Metric::Geometry,
            started.elapsed(),
            result
                .as_ref()
                .is_ok_and(|actual| actual.approximately(frame)),
        );
        result
    }
    fn set_frame_inner(&self, frame: Rect) -> Result<Rect, String> {
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
/// Raise the existing main/focused window; never open a replacement.
pub fn raise_application_window(app:&NSRunningApplication)->Result<(),String> {
    require_permission()?;
    let window=preferred_window(app)?;
    window.unminimize()?;
    let name=NSString::from_str("AXMain");let mut writable=0u8;
    let code=unsafe{AXUIElementIsAttributeSettable(window.0,name_ref(&name),&mut writable)};
    if code==0 && writable!=0 {check(unsafe{AXUIElementSetAttributeValue(window.0,name_ref(&name),kCFBooleanTrue)})?;}
    window.raise()?;
    diagnostics::record("window.activation.raised",format!("pid={}",app.processIdentifier()));
    Ok(())
}
pub fn application_window_ready(app: &NSRunningApplication) -> bool {
    preferred_window(app).is_ok()
}
pub struct MaximizeRequest {
    window: Element,
    app: objc2::rc::Retained<NSRunningApplication>,
    started: Instant,
    transitioned: bool,
    settling: Settling,
}
fn active_fullscreen(mtm: MainThreadMarker) -> bool {
    NSApplication::sharedApplication(mtm)
        .currentSystemPresentationOptions()
        .contains(NSApplicationPresentationOptions::FullScreen)
}
impl MaximizeRequest {
    pub fn start(
        mtm: MainThreadMarker,
        app: Option<&NSRunningApplication>,
    ) -> Result<Self, String> {
        require_permission()?;
        let app = if let Some(app) = app {
            app.retain()
        } else {
            frontmost()?
        };
        if !app.isActive() {
            return Err("L’application n’est plus active : placement annulé.".into());
        }
        let window = preferred_window(&app)?;
        let minimized = window
            .attribute("AXMinimized")
            .is_ok_and(|v| unsafe { CFBooleanGetValue(v.0) != 0 });
        window.unminimize()?;
        let fullscreen = active_fullscreen(mtm);
        if fullscreen {
            // Public system presentation state + public AX button/action. Never
            // toggle blindly or write the undocumented AXFullScreen attribute.
            let button=window.attribute("AXFullScreenButton").map_err(|_|"Cette app ne fournit pas de bouton plein écran accessible. Quittez le plein écran manuellement.")?;
            let press = NSString::from_str("AXPress");
            check(unsafe { AXUIElementPerformAction(button.0, name_ref(&press)) })?;
            diagnostics::record("window.fullscreen.exit_requested", "public_button=true");
        }
        Ok(Self {
            window,
            app,
            started: Instant::now(),
            transitioned: minimized || fullscreen,
            settling: Settling::default(),
        })
    }
    pub fn poll(&mut self, mtm: MainThreadMarker) -> Result<Option<String>, String> {
        require_permission()?;
        if self.app.isTerminated() || !self.app.isActive() {
            return Err("Placement annulé : la fenêtre cible a quitté le premier plan.".into());
        }
        if self.started.elapsed() >= Duration::from_secs(4) {
            return Err("La transition de fenêtre n’est pas terminée après 4 s. Aucun placement supplémentaire demandé ; réessayez après l’animation.".into());
        }
        if active_fullscreen(mtm) {
            self.settling.reset();
            return Ok(None);
        }
        if self
            .window
            .attribute("AXMinimized")
            .is_ok_and(|v| unsafe { CFBooleanGetValue(v.0) != 0 })
        {
            self.settling.reset();
            return Ok(None);
        }
        let frame = self.window.frame()?;
        if self.transitioned
            && !self
                .settling
                .ready(frame, self.started.elapsed().as_millis() as u64)
        {
            return Ok(None);
        }
        if let Ok(focused) = Element::app(self.app.processIdentifier()).attribute("AXFocusedWindow")
        {
            if focused.is_standard() && unsafe { CFEqual(focused.0, self.window.0) } == 0 {
                return Err(
                    "Placement annulé : une autre fenêtre de cette application est devenue active."
                        .into(),
                );
            }
        }
        let target = visible_frame(mtm, frame)?;
        let actual = self.window.set_frame(target)?;
        self.window.raise()?;
        if !actual.approximately(target) {
            return Err(result_message(actual, target, "maximisée"));
        }
        Ok(Some(result_message(actual, target, "maximisée")))
    }
}

/// Titles exist only in memory for this explicit picker; never diagnostic logs.
pub struct Choice {
    pub label: String,
    pub bundle_id: String,
    window: Element,
    app: objc2::rc::Retained<NSRunningApplication>,
}
pub fn pair_candidates(mtm: MainThreadMarker) -> Result<(Rect, Vec<Choice>, bool), String> {
    pair_candidates_for_app(mtm, None)
}
pub fn pair_candidates_for_app(
    mtm: MainThreadMarker,
    anchor_pid: Option<i32>,
) -> Result<(Rect, Vec<Choice>, bool), String> {
    require_permission()?;
    let active = anchor_pid
        .and_then(NSRunningApplication::runningApplicationWithProcessIdentifier)
        .or_else(|| frontmost().ok());
    let anchor = active
        .and_then(|app| preferred_window(&app).ok())
        .and_then(|w| w.frame().ok())
        .unwrap_or(Rect {
            x: 0.0,
            y: 0.0,
            width: 0.0,
            height: 0.0,
        });
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
                bundle_id: app
                    .bundleIdentifier()
                    .map(|b| b.to_string())
                    .unwrap_or_default(),
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

/// One transient return point, shared by Meeting and the saved personal layout.
/// Capture before any AX write; a failed read-back must never masquerade as before.
pub struct LayoutSnapshot {
    parts: [LayoutFrame; 2],
    prior_pid: Option<i32>,
}
struct LayoutFrame {
    window: Element,
    before: Rect,
    applied: Option<Rect>,
}
impl Element {
    fn confirmed_gone(&self) -> bool {
        let key = NSString::from_str("AXRole");
        let mut value = ptr::null();
        let code = unsafe { AXUIElementCopyAttributeValue(self.0, name_ref(&key), &mut value) };
        if !value.is_null() {
            unsafe {
                CFRelease(value);
            }
        }
        code == -25202 // invalid element, not timeout or permission refusal
    }
}
impl LayoutSnapshot {
    pub fn restore(&mut self) -> Result<String, String> {
        require_permission()?;
        let (mut restored, mut skipped, mut pending) = (0, 0, 0);
        for part in &mut self.parts {
            let current = match part.window.frame() {
                Ok(frame) => frame,
                Err(_) if part.window.confirmed_gone() => {
                    skipped += 1;
                    continue;
                }
                Err(_) => {
                    pending += 1;
                    continue;
                }
            };
            match restore_decision(current, part.before, part.applied) {
                RestoreDecision::AlreadyRestored => {}
                RestoreDecision::UserMoved => skipped += 1,
                RestoreDecision::Unknown => pending += 1,
                RestoreDecision::Restore => match part.window.set_frame(part.before) {
                    Ok(actual) if actual.approximately(part.before) => restored += 1,
                    Ok(actual) => {
                        part.applied = Some(actual);
                        pending += 1;
                    }
                    Err(_) => {
                        part.applied = part.window.frame().ok();
                        pending += 1;
                    }
                },
            }
        }
        diagnostics::record(
            "workspace.restore.result",
            format!("restored={restored} skipped={skipped} pending={pending}"),
        );
        if pending > 0 {
            return Err(format!("Restauration incomplète : {pending} fenêtre(s) en erreur ou de position incertaine. Le retour est conservé ; les autres fenêtres ont été traitées."));
        }
        if let Some(app) = self
            .prior_pid
            .and_then(NSRunningApplication::runningApplicationWithProcessIdentifier)
        {
            let _ = crate::applications::activate(&app);
        }
        Ok(format!("Disposition précédente : {restored} fenêtre(s) restaurée(s), {skipped} ignorée(s) car fermée(s) ou déplacée(s) manuellement."))
    }
}
fn apply_workspace(
    choices: [Choice; 2],
    targets: [Rect; 2],
    snapshot: &mut Option<LayoutSnapshot>,
    prior_pid: Option<i32>,
) -> Result<String, String> {
    require_permission()?;
    if snapshot.is_some() {
        return Err("Une disposition temporaire est déjà active. Restaurez-la ou choisissez Travail avant d'en appliquer une autre.".into());
    }
    if unsafe { CFEqual(choices[0].window.0, choices[1].window.0) } != 0 {
        return Err("Choisissez deux fenêtres distinctes.".into());
    }
    for choice in &choices {
        choice.window.ensure_normal()?;
        choice.window.can_set("AXPosition")?;
        choice.window.can_set("AXSize")?;
    }
    let before = [choices[0].window.frame()?, choices[1].window.frame()?];
    let focus = choices[1].app.clone();
    let [left, right] = choices;
    *snapshot = Some(LayoutSnapshot {
        parts: [
            LayoutFrame {
                window: left.window,
                before: before[0],
                applied: Some(before[0]),
            },
            LayoutFrame {
                window: right.window,
                before: before[1],
                applied: Some(before[1]),
            },
        ],
        prior_pid,
    });
    let mut complete = true;
    for (part, target) in snapshot.as_mut().unwrap().parts.iter_mut().zip(targets) {
        match part.window.set_frame(target) {
            Ok(actual) => {
                part.applied = Some(actual);
                complete &= actual.approximately(target);
            }
            Err(_) => {
                part.applied = part.window.frame().ok();
                complete = false;
            }
        }
    }
    diagnostics::record(
        "workspace.apply",
        format!("geometry_ok={complete} snapshot=true"),
    );
    if !complete {
        return Err(
            "Placement incomplet. Le retour est conservé : utilisez Restaurer la disposition."
                .into(),
        );
    }
    crate::applications::activate(&focus)?;
    snapshot.as_ref().unwrap().parts[1].window.raise()?;
    Ok("Disposition appliquée aux deux fenêtres.".into())
}
pub fn meeting_browser(
    mtm: MainThreadMarker,
    snapshot: &mut Option<LayoutSnapshot>,
) -> Result<String, String> {
    let teams_app = crate::applications::running(crate::applications::teams_bundle()?)
        .ok_or("Ouvrez Teams avec ⌥T avant d'activer la scène.")?;
    let chrome_app = crate::applications::running(crate::applications::CHROME)
        .ok_or("Ouvrez Chrome avec ⌥B avant d'activer la scène.")?;
    let teams = preferred_window(&teams_app)?;
    let chrome = preferred_window(&chrome_app)?;
    let display = visible_frame(mtm, chrome.frame()?)?;
    apply_workspace(
        [
            Choice {
                label: String::new(),
                bundle_id: String::new(),
                window: teams,
                app: teams_app,
            },
            Choice {
                label: String::new(),
                bundle_id: String::new(),
                window: chrome,
                app: chrome_app,
            },
        ],
        [display.left_half(), display.right_half()],
        snapshot,
        NSWorkspace::sharedWorkspace()
            .frontmostApplication()
            .map(|a| a.processIdentifier())
            .filter(|p| *p != std::process::id() as i32),
    )?;
    Ok("Réunion : Teams à gauche et Chrome à droite, 50/50.".into())
}
pub fn capture_layout(mtm: MainThreadMarker, choices: [&Choice; 2]) -> Result<SavedLayout, String> {
    for choice in choices {
        choice.window.ensure_normal()?;
    }
    if unsafe { CFEqual(choices[0].window.0, choices[1].window.0) } != 0 {
        return Err("Choisissez deux fenêtres distinctes.".into());
    }
    let first = choices[0].window.frame()?;
    let second = choices[1].window.frame()?;
    let display = visible_frame(mtm, first)?;
    if !display.approximately(visible_frame(mtm, second)?) {
        return Err("Cette première disposition personnelle utilise un seul écran. Placez les deux fenêtres sur le même écran avant d'enregistrer.".into());
    }
    let geometry = |frame| {
        NormalizedRect::capture(frame, display)
        .ok_or("Placez les deux fenêtres entièrement dans la zone visible de l'écran avant d'enregistrer.")
    };
    let layout = SavedLayout {
        schema_version: 1,
        windows: [
            SavedWindow {
                bundle_id: choices[0].bundle_id.clone(),
                geometry: geometry(first)?,
            },
            SavedWindow {
                bundle_id: choices[1].bundle_id.clone(),
                geometry: geometry(second)?,
            },
        ],
    };
    layout.validate()?;
    Ok(layout)
}
pub fn apply_saved_layout(
    layout: &SavedLayout,
    display: Rect,
    choices: [Choice; 2],
    snapshot: &mut Option<LayoutSnapshot>,
    prior_pid: Option<i32>,
) -> Result<String, String> {
    layout.validate()?;
    apply_workspace(
        choices,
        layout
            .windows
            .each_ref()
            .map(|w| w.geometry.on_display(display)),
        snapshot,
        prior_pid,
    )
}

pub fn work_browser(mtm: MainThreadMarker) -> Result<String, String> {
    let chrome_app = crate::applications::running(crate::applications::CHROME)
        .ok_or("Ouvrez Chrome avec ⌥B avant d'activer Travail.")?;
    require_permission()?;
    let window = preferred_window(&chrome_app)?;
    window.unminimize()?;
    window.ensure_normal()?;
    let target = visible_frame(mtm, window.frame()?)?;
    crate::applications::activate(&chrome_app)?;
    let actual = window.set_frame(target)?;
    if !actual.approximately(target) {
        return Err(
            "Chrome ne couvre pas toute la zone visible : disposition précédente conservée.".into(),
        );
    }
    window.raise()?;
    Ok("Travail : Chrome au premier plan à 100 %, Teams conservé derrière.".into())
}
