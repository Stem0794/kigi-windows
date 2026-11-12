//! Opt-in passive contact delivery. Queue recognized actions; never run AX in the tap.
use crate::{
    diagnostics,
    gestures::{Contact, Edge, EdgeProbe},
};
use objc2::{msg_send, rc::Retained};
use objc2_app_kit::{NSEvent, NSEventType, NSTouchPhase};
use objc2_core_graphics::CGEvent;
use std::{ffi::c_void, ptr, time::Instant};

type Ref = *const c_void;
#[link(name = "ApplicationServices", kind = "framework")]
extern "C" {
    fn CGEventTapCreate(
        tap: u32,
        place: u32,
        options: u32,
        mask: u64,
        callback: unsafe extern "C" fn(Ref, u32, *mut CGEvent, *mut c_void) -> *mut CGEvent,
        user: *mut c_void,
    ) -> Ref;
    fn CGEventTapEnable(tap: Ref, enable: bool);
    fn CGPreflightListenEventAccess() -> bool;
    fn CGRequestListenEventAccess() -> bool;
}
#[link(name = "CoreFoundation", kind = "framework")]
extern "C" {
    fn CFMachPortCreateRunLoopSource(allocator: Ref, port: Ref, order: isize) -> Ref;
    fn CFMachPortInvalidate(port: Ref);
    fn CFRunLoopGetMain() -> Ref;
    fn CFRunLoopAddSource(loop_: Ref, source: Ref, mode: Ref);
    fn CFRunLoopRemoveSource(loop_: Ref, source: Ref, mode: Ref);
    fn CFRelease(value: Ref);
    static kCFRunLoopCommonModes: Ref;
}
#[derive(Default)]
struct Stats {
    events: u64,
    with_touches: u64,
    samples: u64,
    recognized: u64,
    disabled: bool,
    started: Option<Instant>,
    edges: EdgeProbe,
    actions: bool,
    left: u64,
    right: u64,
}

pub struct Tap {
    port: Ref,
    source: Ref,
    stats: Box<Stats>,
}
impl Tap {
    pub fn start() -> Result<Self, String> {
        Self::start_mode(false)
    }
    pub fn start_actions() -> Result<Self, String> {
        Self::start_mode(true)
    }
    pub fn active_actions(&self) -> bool {
        self.stats.actions && !self.stats.disabled
    }
    pub fn input_trusted() -> bool {
        unsafe { CGPreflightListenEventAccess() }
    }
    fn start_mode(actions: bool) -> Result<Self, String> {
        unsafe {
            // Separate, explicit opt-in from Accessibility. Available since 10.15.
            let trusted = CGPreflightListenEventAccess();
            diagnostics::record("permission.input.check", format!("trusted={trusted}"));
            if !trusted && !CGRequestListenEventAccess() {
                return Err("Surveillance de l'entrée non autorisée. Autorisez-la pour l’écoute du trackpad, puis relancez Kigi. Cela ne garantit pas l'accès aux contacts.".into());
            }
            let mut stats = Box::new(Stats {
                started: Some(Instant::now()),
                actions,
                ..Default::default()
            });
            // Gesture=29 is NSEventTypeGesture in public AppKit headers, NOT
            // a documented CGEventType. Cross-layer delivery is the hypothesis
            // under test. No numeric private event types and no keyboard mask.
            let mask = 1u64 << NSEventType::Gesture.0;
            let port =
                CGEventTapCreate(1, 0, 1, mask, callback, (&mut *stats as *mut Stats).cast());
            diagnostics::record(
                "trackpad.tap.create",
                format!("success={}", !port.is_null()),
            );
            if port.is_null() {
                return Err("Le tap passif n'a pas pu être créé. Aucun geste global disponible avec cet essai.".into());
            }
            let source = CFMachPortCreateRunLoopSource(ptr::null(), port, 0);
            if source.is_null() {
                CFMachPortInvalidate(port);
                CFRelease(port);
                return Err("Source de boucle d'événements indisponible.".into());
            }
            CFRunLoopAddSource(CFRunLoopGetMain(), source, kCFRunLoopCommonModes);
            CGEventTapEnable(port, true);
            Ok(Self {
                port,
                source,
                stats,
            })
        }
    }
    pub fn contacts(&self) -> u64 {
        self.stats.with_touches
    }
    pub fn elapsed_seconds(&self) -> u64 {
        self.stats
            .started
            .as_ref()
            .map(|t| t.elapsed().as_secs())
            .unwrap_or(0)
    }
    pub fn summary(&self) -> String {
        format!(
            "{} événements · {} avec contacts · {} gestes gauche / {} droite. {}",
            self.stats.events,
            self.stats.with_touches,
            self.stats.left,
            self.stats.right,
            if self.stats.with_touches == 0 {
                "Réception globale non confirmée (test de 12 s)."
            } else {
                "Contacts reçus ; gestes à vérifier sur ce Mac."
            }
        )
    }
    pub fn report(&self) -> String {
        if self.stats.actions {
            return format!("Gestes de bord : gauche {} / droite {} détecté(s). Événements : {} ; avec contacts : {} ; échantillons : {}. Tap interrompu : {}.\nGauche 5 % vers milieu → Teams ; droite 5 % vers milieu → Chrome. Une action/contact, cooldown 1 s.\nAprès veille/session inactive/interruption, réactivation manuelle dans Gestes et souris. Résultats propres à ce Mac.",
                self.stats.left, self.stats.right, self.stats.events, self.stats.with_touches, self.stats.samples, self.stats.disabled);
        }
        format!("Essai global passif : {} événement(s), {} avec contacts, {} échantillon(s), {} geste(s) bords 5 % → milieu. Tap interrompu : {}.\n\n{}\nAucune action de fenêtre déclenchée. Résultats propres à ce Mac et à ces permissions.",
            self.stats.events, self.stats.with_touches, self.stats.samples, self.stats.recognized,
            self.stats.disabled,
            if self.stats.with_touches == 0 { "Aucun contact reçu : geste global NON VALIDÉ. Utilisez le test local pour vérifier le trackpad." } else if self.stats.samples == 0 { "Contacts reçus, mais aucun échantillon valide à un seul doigt." } else { "Des contacts ont été reçus : validation terrain encore nécessaire (phases, veille, Intel/ARM, faux positifs)." })
    }
}
impl Drop for Tap {
    fn drop(&mut self) {
        // Main run loop only. Disable/remove before freeing callback context.
        unsafe {
            CGEventTapEnable(self.port, false);
            CFRunLoopRemoveSource(CFRunLoopGetMain(), self.source, kCFRunLoopCommonModes);
            CFMachPortInvalidate(self.port);
            CFRelease(self.source);
            CFRelease(self.port);
        }
    }
}
unsafe extern "C" fn callback(
    _: Ref,
    kind: u32,
    event: *mut CGEvent,
    user: *mut c_void,
) -> *mut CGEvent {
    let stats = &mut *user.cast::<Stats>();
    if kind == u32::MAX || kind == u32::MAX - 1 {
        // Fail closed; explicit restart rather than automatic reenable loops.
        stats.disabled = true;
        stats.edges.cancel();
        crate::app::queue_tap_interrupted();
        return event;
    }
    if stats.disabled {
        return event;
    }
    if kind != NSEventType::Gesture.0 as u32 || event.is_null() {
        return event;
    }
    stats.events += 1;
    let Some(ns) = NSEvent::eventWithCGEvent(&*event) else {
        return event;
    };
    // Never ask a mouse/scroll event for touches (AppKit can raise an exception).
    if ns.r#type() != NSEventType::Gesture {
        return event;
    }
    let touches = ns.allTouches();
    if touches.count() > 0 {
        stats.with_touches += 1;
    }
    let fingers = touches
        .iter()
        .filter(|t| t.phase().intersects(NSTouchPhase::Touching))
        .count();
    if fingers != 1 || NSEvent::pressedMouseButtons() != 0 {
        if fingers == 0 {
            stats.edges.release();
        } else {
            stats.edges.cancel();
        }
        return event;
    }
    // An ended/cancelled contact alongside an active one is an interruption,
    // never the start of another one-finger gesture.
    if touches.iter().any(|t| {
        t.phase()
            .intersects(NSTouchPhase::Ended | NSTouchPhase::Cancelled)
    }) {
        stats.edges.cancel();
        return event;
    }
    for touch in touches.iter().take(8) {
        let phase = touch.phase();
        if phase.intersects(NSTouchPhase::Ended | NSTouchPhase::Cancelled) {
            continue;
        }
        let position = touch.normalizedPosition();
        let identity = touch.identity();
        // Hash is runtime-only, never logged or persisted. No contact storage.
        let key: usize = msg_send![&*identity, hash];
        let device = touch
            .device()
            .map(|d| Retained::as_ptr(&d) as usize)
            .unwrap_or(0);
        let sample = Contact {
            identity: key,
            device,
            x: position.x,
            y: position.y,
            ms: stats.started.as_ref().unwrap().elapsed().as_millis() as u64,
        };
        stats.samples += 1;
        if phase == NSTouchPhase::Began {
            stats.edges.begin(sample, fingers);
        } else if phase == NSTouchPhase::Moved {
            if let Some(edge) = stats.edges.moved(sample, fingers) {
                stats.recognized += 1;
                match edge {
                    Edge::Left => stats.left += 1,
                    Edge::Right => stats.right += 1,
                }
                if stats.actions {
                    crate::app::queue_gesture(edge);
                }
            }
        }
    }
    // No AX, Apple Events, logs or window activation in this callback.
    // A rare recognized gesture only queues a selector for after callback return.
    event
}
