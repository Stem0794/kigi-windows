//! Small owned CoreFoundation wrapper around the public Accessibility C API.
use crate::{diagnostics, geometry::Rect};
use objc2::MainThreadMarker;
use objc2_app_kit::{NSScreen, NSWorkspace};
use objc2_foundation::NSString;
use std::{
    ffi::c_void,
    ptr,
    time::{Duration, Instant},
};

type Ref = *const c_void;
#[link(name = "ApplicationServices", kind = "framework")]
extern "C" {
    fn AXIsProcessTrusted() -> bool;
    fn AXIsProcessTrustedWithOptions(options: Ref) -> bool;
    static kAXTrustedCheckOptionPrompt: Ref;
    fn AXUIElementCreateApplication(pid: i32) -> Ref;
    fn AXUIElementSetMessagingTimeout(element: Ref, timeout: f32) -> i32;
    fn AXUIElementCopyAttributeValue(element: Ref, name: Ref, value: *mut Ref) -> i32;
    fn AXUIElementIsAttributeSettable(element: Ref, name: Ref, result: *mut u8) -> i32;
    fn AXUIElementSetAttributeValue(element: Ref, name: Ref, value: Ref) -> i32;
    fn AXValueCreate(kind: u32, value: *const c_void) -> Ref;
    fn AXValueGetValue(value: Ref, kind: u32, result: *mut c_void) -> bool;
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
    fn CFBooleanGetValue(value: Ref) -> bool;
    static kCFBooleanTrue: Ref;
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
        if unsafe { AXValueGetValue(value.0, kind, pair.as_mut_ptr().cast()) } {
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
                if unsafe { CFBooleanGetValue(value.0) } {
                    return Err("Test refusé : fenêtre minimisée ou en plein écran natif.".into());
                }
            }
        }
        Ok(())
    }
    fn set_frame(&self, frame: Rect) -> Result<Rect, String> {
        require_permission()?;
        self.ensure_normal()?;
        diagnostics::record("ax.frame.request", format!("target={frame:?}"));
        self.can_set("AXPosition")?;
        self.can_set("AXSize")?;
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
    unsafe { AXIsProcessTrusted() }
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
        AXIsProcessTrustedWithOptions(options.0)
    }
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
    let v = screen.visibleFrame();
    let target = Rect::from_appkit(
        v.origin.x,
        v.origin.y,
        v.size.width,
        v.size.height,
        primary_top,
    )
    .right_half();
    // Save BEFORE either write: partial writes and errors must stay restorable.
    *snapshot = Some(Snapshot { window, frame });
    let actual = snapshot.as_ref().unwrap().window.set_frame(target)?;
    Ok(result_message(actual, target, "déplacée à droite"))
}
