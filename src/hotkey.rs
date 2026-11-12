//! Public Carbon hotkey registration; no keyboard event monitoring.
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
const ID: HotkeyId = HotkeyId {
    signature: u32::from_be_bytes(*b"Kigi"),
    id: 1,
};
#[link(name = "Carbon", kind = "framework")]
extern "C" {
    fn GetApplicationEventTarget() -> Handle;
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

pub struct Hotkey {
    key: Handle,
    handler: Handle,
}
impl Hotkey {
    pub fn register() -> Result<Self, String> {
        unsafe {
            let target = GetApplicationEventTarget();
            // Released only: one action per press, no auto-repeat on long holds.
            let event = EventType {
                class: u32::from_be_bytes(*b"keyb"),
                kind: 6,
            };
            let mut this = Self {
                key: ptr::null_mut(),
                handler: ptr::null_mut(),
            };
            let code = InstallEventHandler(
                target,
                on_hotkey,
                1,
                &event,
                ptr::null_mut(),
                &mut this.handler,
            );
            if code != 0 {
                return Err(format!("Installation du raccourci refusée ({code})."));
            }
            // kVK_ANSI_T = 17, optionKey = 1 << 11, controlKey = 1 << 12. Physical T position.
            let code = RegisterEventHotKey(17, (1 << 11) | (1 << 12), ID, target, 1, &mut this.key);
            if code != 0 {
                return Err(format!("⌃⌥T indisponible ({code}). Vérifiez Hammerspoon et les raccourcis système. Le menu reste utilisable."));
            }
            Ok(this)
        }
    }
}
impl Drop for Hotkey {
    fn drop(&mut self) {
        unsafe {
            if !self.key.is_null() {
                UnregisterEventHotKey(self.key);
            }
            if !self.handler.is_null() {
                RemoveEventHandler(self.handler);
            }
        }
    }
}
unsafe extern "C" fn on_hotkey(_: Handle, event: Handle, _: Handle) -> i32 {
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
    if code == 0 && id.signature == ID.signature && id.id == ID.id {
        // Callback runs on the application/main event loop. Defer AX until it returns.
        crate::app::schedule_test();
        0
    } else {
        -9874
    } // eventNotHandledErr
}
