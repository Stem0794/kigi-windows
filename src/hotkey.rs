//! Public Carbon hotkey registration; no keyboard event monitoring.
use crate::shortcuts::Shortcuts;
use std::{
    ffi::{c_ulong, c_void},
    ptr,
    sync::atomic::{AtomicU32, Ordering},
};

type Handle = *mut c_void;
#[repr(C, packed(2))]
struct EventType {
    class: u32,
    kind: u32,
}
#[repr(C, packed(2))]
#[derive(Clone, Copy)]
struct HotkeyId {
    signature: u32,
    id: u32,
}
static GENERATION: AtomicU32 = AtomicU32::new(u32::from_be_bytes(*b"Kigi"));
struct Context {
    down: Vec<bool>,
    signature: u32,
}
#[link(name = "Carbon", kind = "framework")]
extern "C" {
    fn GetEventDispatcherTarget() -> Handle;
    fn GetEventKind(event: Handle) -> u32;
    fn InstallEventHandler(
        target: Handle,
        callback: unsafe extern "C" fn(Handle, Handle, Handle) -> i32,
        count: c_ulong,
        types: *const EventType,
        user: Handle,
        handler: *mut Handle,
    ) -> i32;
    fn RemoveEventHandler(handler: Handle) -> i32;
    fn RegisterEventHotKey(
        key: u32,
        modifiers: u32,
        id: HotkeyId,
        target: Handle,
        options: u32,
        result: *mut Handle,
    ) -> i32;
    fn UnregisterEventHotKey(hotkey: Handle) -> i32;
    fn GetEventParameter(
        event: Handle,
        name: u32,
        kind: u32,
        actual_type: *mut u32,
        size: c_ulong,
        actual_size: *mut c_ulong,
        data: Handle,
    ) -> i32;
}

pub struct Hotkeys {
    keys: Vec<Handle>,
    handler: Handle,
    context: Box<Context>,
    pub report: String,
    pub failed: bool,
}
impl Hotkeys {
    pub fn register(personal: bool, settings: &Shortcuts) -> Self {
        let mut this = Self {
            keys: Vec::new(),
            handler: ptr::null_mut(),
            context: Box::new(Context {
                down: vec![false; settings.entries.len()],
                signature: GENERATION.fetch_add(1, Ordering::Relaxed),
            }),
            report: String::new(),
            failed: false,
        };
        unsafe {
            let target = GetEventDispatcherTarget();
            let events = [
                EventType {
                    class: u32::from_be_bytes(*b"keyb"),
                    kind: 5,
                },
                EventType {
                    class: u32::from_be_bytes(*b"keyb"),
                    kind: 6,
                },
            ];
            let code = InstallEventHandler(
                target,
                on_hotkey,
                events.len() as c_ulong,
                events.as_ptr(),
                (&mut *this.context as *mut Context).cast(),
                &mut this.handler,
            );
            if code != 0 {
                this.failed = personal;
                this.report = format!(
                    "Installation du gestionnaire refusée ({code}). Les menus restent utilisables."
                );
                return this;
            }
            for (index, binding) in settings.entries.iter().enumerate() {
                if !binding.enabled || !personal {
                    continue;
                }
                let (key, modifiers) = (binding.key, binding.modifiers);
                let mut handle = ptr::null_mut();
                let code = RegisterEventHotKey(
                    key,
                    modifiers,
                    HotkeyId {
                        signature: this.context.signature,
                        id: index as u32,
                    },
                    target,
                    1,
                    &mut handle,
                );
                let state = if code == 0 {
                    this.keys.push(handle);
                    "enregistré".to_owned()
                } else {
                    this.failed = true;
                    format!("indisponible ({code}) — conflit possible avec macOS ou Hammerspoon")
                };
                this.report
                    .push_str(&format!("Raccourci {} : {state}\n", index + 1));
            }
        }
        this
    }
}
impl Drop for Hotkeys {
    fn drop(&mut self) {
        unsafe {
            for key in &self.keys {
                UnregisterEventHotKey(*key);
            }
            if !self.handler.is_null() {
                RemoveEventHandler(self.handler);
            }
        }
    }
}
unsafe extern "C" fn on_hotkey(_: Handle, event: Handle, user: Handle) -> i32 {
    let mut id = HotkeyId {
        signature: 0,
        id: 0,
    };
    let code = GetEventParameter(
        event,
        u32::from_be_bytes(*b"----"),
        u32::from_be_bytes(*b"hkid"),
        ptr::null_mut(),
        std::mem::size_of::<HotkeyId>() as c_ulong,
        ptr::null_mut(),
        (&mut id as *mut HotkeyId).cast(),
    );
    if code != 0 {
        crate::diagnostics::record("shortcut.parameter_error", format!("code={code}"));
        return code;
    }
    if user.is_null() {
        return -9874;
    }
    // Events queued before a remap must never execute a new slot assignment.
    let context = &mut *user.cast::<Context>();
    if id.signature != context.signature {
        return -9874;
    }
    let index = id.id as usize;
    let Some(pressed) = context.down.get_mut(index) else {
        return -9874;
    };
    if press_transition(pressed, GetEventKind(event)) {
        crate::app::schedule_shortcut(index);
    }
    0
}

fn press_transition(down: &mut bool, kind: u32) -> bool {
    match kind {
        5 => !std::mem::replace(down, true),
        6 => {
            *down = false;
            false
        }
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::press_transition;
    #[test]
    fn one_action_per_press_even_when_held() {
        let mut down = false;
        assert!(!press_transition(&mut down, 6));
        assert!(press_transition(&mut down, 5));
        assert!(!press_transition(&mut down, 5));
        assert!(!press_transition(&mut down, 6));
        assert!(press_transition(&mut down, 5));
        let mut keys = [false; 7];
        assert!(press_transition(&mut keys[1], 5));
        assert!(press_transition(&mut keys[2], 5));
        assert!(!press_transition(&mut keys[1], 5));
        assert!(!press_transition(&mut keys[1], 6));
        assert!(!press_transition(&mut keys[2], 5));
    }
}
