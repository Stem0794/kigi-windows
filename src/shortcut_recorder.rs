//! A focused, explicit shortcut field. No global keyboard listener or text storage.
use objc2::{define_class, msg_send, rc::Retained, DefinedClass, MainThreadOnly};
use objc2_app_kit::{
    NSAutoresizingMaskOptions, NSEvent, NSEventModifierFlags, NSTextField, NSView,
};
use objc2_foundation::{MainThreadMarker, NSObjectProtocol, NSPoint, NSRect, NSSize, NSString};
use std::cell::{Cell, OnceCell};

#[derive(Default)]
pub struct State {
    recording: Cell<bool>,
    label: OnceCell<Retained<NSTextField>>,
}
define_class!(
    #[unsafe(super = NSView)]
    #[thread_kind = MainThreadOnly]
    #[ivars = State]
    pub struct Recorder;
    unsafe impl NSObjectProtocol for Recorder {}
    impl Recorder {
        #[unsafe(method(acceptsFirstResponder))]
        fn accepts_first(&self)->bool { self.ivars().recording.get() }
        #[unsafe(method(resignFirstResponder))]
        fn resign(&self)->bool {
            if self.ivars().recording.replace(false) { crate::app::shortcut_captured(None); }
            true
        }
        #[unsafe(method(keyDown:))]
        fn key_down(&self,event:&NSEvent) { self.capture(event); }
        #[unsafe(method(performKeyEquivalent:))]
        fn equivalent(&self,event:&NSEvent)->bool {
            if self.ivars().recording.get() { self.capture(event);true } else {false}
        }
        #[unsafe(method(hitTest:))]
        fn hit_test(&self,point:NSPoint)->Option<&NSView> {
            let hit:Option<Retained<NSView>>=unsafe{msg_send![super(self),hitTest:point]};
            hit.map(|_| &**self)
        }
    }
);
impl Recorder {
    pub fn new(mtm: MainThreadMarker) -> Retained<Self> {
        let this = Self::alloc(mtm).set_ivars(State::default());
        let view: Retained<Self> = unsafe {
            msg_send![super(this),initWithFrame:NSRect::new(NSPoint::new(22.0,277.0),NSSize::new(330.0,42.0))]
        };
        let text =
            NSTextField::labelWithString(&NSString::from_str("Choisissez un raccourci"), mtm);
        text.setFrame(NSRect::new(
            NSPoint::new(8.0, 8.0),
            NSSize::new(314.0, 28.0),
        ));
        text.setAutoresizingMask(NSAutoresizingMaskOptions::ViewWidthSizable);
        view.addSubview(&text);
        view.ivars().label.set(text).ok();
        view
    }
    pub fn set_combination(&self, text: &str) {
        if let Some(label) = self.ivars().label.get() {
            label.setStringValue(&NSString::from_str(text));
        }
    }
    pub fn start(&self) -> bool {
        let Some(window) = self.window() else {
            return false;
        };
        self.ivars().recording.set(true);
        if !window.makeFirstResponder(Some(self)) {
            self.ivars().recording.set(false);
            return false;
        }
        self.set_combination("Appuyez sur le raccourci… (Échap : annuler)");
        true
    }
    pub fn cancel(&self) {
        if self.ivars().recording.replace(false) {
            crate::app::shortcut_captured(None);
        }
    }
    pub fn is_recording(&self) -> bool {
        self.ivars().recording.get()
    }
    fn capture(&self, event: &NSEvent) {
        if self.isHiddenOrHasHiddenAncestor() || self.window().is_none() {
            self.cancel();
            return;
        }
        if !self.ivars().recording.get() || event.isARepeat() {
            return;
        }
        let flags = event.modifierFlags();
        let mut modifiers = 0;
        for (flag, carbon) in [
            (NSEventModifierFlags::Command, 1 << 8),
            (NSEventModifierFlags::Shift, 1 << 9),
            (NSEventModifierFlags::Option, 1 << 11),
            (NSEventModifierFlags::Control, 1 << 12),
        ] {
            if flags.contains(flag) {
                modifiers |= carbon;
            }
        }
        let key = u32::from(event.keyCode());
        if key == 53 && modifiers == 0 {
            self.cancel();
            return;
        }
        if !crate::shortcuts::valid_combination(key, modifiers) {
            self.set_combination("Ajoutez ⌘, ⌥ ou ⌃ à la touche.");
            return;
        }
        let name = match key {
            123 => "←".into(),
            124 => "→".into(),
            125 => "↓".into(),
            126 => "↑".into(),
            36 => "Retour".into(),
            48 => "Tab".into(),
            49 => "Espace".into(),
            51 => "⌫".into(),
            53 => "Échap".into(),
            117 => "⌦".into(),
            122 => "F1".into(),
            120 => "F2".into(),
            99 => "F3".into(),
            118 => "F4".into(),
            96 => "F5".into(),
            97 => "F6".into(),
            98 => "F7".into(),
            100 => "F8".into(),
            101 => "F9".into(),
            109 => "F10".into(),
            103 => "F11".into(),
            111 => "F12".into(),
            _ => event
                .charactersIgnoringModifiers()
                .map(|s| {
                    s.to_string()
                        .chars()
                        .filter(|c| !c.is_control())
                        .collect::<String>()
                        .to_uppercase()
                })
                .filter(|s| !s.is_empty())
                .unwrap_or_else(|| format!("Touche {key}")),
        };
        self.ivars().recording.set(false);
        crate::app::shortcut_captured(Some((key, modifiers, name)));
        if let Some(window) = self.window() {
            window.makeFirstResponder(None);
        }
    }
}
