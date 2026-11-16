//! Opt-in Cmd+Tab override only. Other keys are returned unchanged and never retained/logged.
use objc2_core_graphics::CGEvent;
use std::{ffi::c_void, ptr};
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
    fn CGEventTapEnable(tap: Ref, enabled: bool);
    fn CGEventGetIntegerValueField(event: *const CGEvent, field: u32) -> i64;
    fn CGEventGetFlags(event: *const CGEvent) -> u64;
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
struct Context {
    swallow: bool,
    disabled: bool,
}
pub struct WindowHotkey {
    port: Ref,
    source: Ref,
    context: Box<Context>,
}
impl WindowHotkey {
    pub fn start() -> Result<Self, String> {
        if !crate::windows::trusted() {
            return Err("Accessibilité requise pour remplacer ⌘Tab.".into());
        }
        let mut context = Box::<Context>::default();
        unsafe {
            let port = CGEventTapCreate(
                1,
                0,
                0,
                (1 << 10) | (1 << 11) | (1 << 12),
                callback,
                (&mut *context as *mut Context).cast(),
            );
            if port.is_null() {
                return Err("macOS refuse le raccourci système ⌘Tab. Les autres raccourcis restent disponibles.".into());
            }
            let source = CFMachPortCreateRunLoopSource(ptr::null(), port, 0);
            if source.is_null() {
                CFMachPortInvalidate(port);
                CFRelease(port);
                return Err("Source ⌘Tab indisponible.".into());
            }
            CFRunLoopAddSource(CFRunLoopGetMain(), source, kCFRunLoopCommonModes);
            CGEventTapEnable(port, true);
            Ok(Self {
                port,
                source,
                context,
            })
        }
    }
    pub fn active(&self) -> bool {
        !self.context.disabled
    }
}
impl Drop for WindowHotkey {
    fn drop(&mut self) {
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
    if user.is_null() {
        return event;
    }
    let context = unsafe { &mut *user.cast::<Context>() };
    if kind == 0xffff_fffe || kind == 0xffff_ffff {
        context.disabled = true;
        crate::app::schedule_personal_resume();
        return event;
    }
    if event.is_null() {
        return event;
    }
    let flags = unsafe { CGEventGetFlags(event) };
    if kind == 12 {
        if flags & (1 << 20) == 0 {
            crate::app::queue_window_cycle(false, true);
        }
        return event;
    }
    let tab = unsafe { CGEventGetIntegerValueField(event, 9) } == 48;
    if kind == 11 && tab && context.swallow {
        context.swallow = false;
        return ptr::null_mut();
    }
    if kind == 10
        && tab
        && crate::runtime_settings::matches_window_tab(48, flags)
        && crate::app::queue_window_cycle(flags & (1 << 17) != 0, false)
    {
        context.swallow = true;
        return ptr::null_mut();
    }
    event
}
