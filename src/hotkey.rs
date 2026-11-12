//! Public Carbon hotkey registration; no keyboard event monitoring.
use crate::actions::Action;
use std::{
    ffi::{c_ulong, c_void},
    ptr,
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
const SIGNATURE: u32 = u32::from_be_bytes(*b"Kigi");
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
    down: Box<[bool; 7]>,
    pub report: String,
}
impl Hotkeys {
    pub fn register(personal: bool) -> Self {
        let mut this = Self {
            keys: Vec::new(),
            handler: ptr::null_mut(),
            down: Box::new([false; 7]),
            report: String::new(),
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
                this.down.as_mut_ptr().cast(),
                &mut this.handler,
            );
            if code != 0 {
                this.report = format!(
                    "Installation du gestionnaire refusée ({code}). Les menus restent utilisables."
                );
                return this;
            }
            for action in Action::ALL {
                if action != Action::Test && !personal {
                    continue;
                }
                let (key, modifiers) = action.binding();
                let mut handle = ptr::null_mut();
                let code = RegisterEventHotKey(
                    key,
                    modifiers,
                    HotkeyId {
                        signature: SIGNATURE,
                        id: action as u32,
                    },
                    target,
                    1,
                    &mut handle,
                );
                let state = if code == 0 {
                    this.keys.push(handle);
                    "enregistré".to_owned()
                } else {
                    format!("indisponible ({code}) — conflit possible avec macOS ou Hammerspoon")
                };
                this.report
                    .push_str(&format!("{} : {state}\n", action.label()));
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
    if id.signature != SIGNATURE || id.id as usize >= Action::ALL.len() || user.is_null() {
        return -9874;
    }
    let index = id.id as usize;
    if press_transition(&mut *user.cast::<bool>().add(index), GetEventKind(event)) {
        crate::app::schedule_action(Action::ALL[index]);
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
