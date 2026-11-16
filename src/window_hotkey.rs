//! Opt-in Cmd+Tab only. Its event loop is independent from potentially slow AX work.
use block2::{Block, RcBlock};
use objc2_core_graphics::CGEvent;
use std::{
    ffi::c_void,
    ptr,
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc, Arc,
    },
    thread,
    time::Duration,
};
type Ref = *const c_void;
static CAPTURE_ALLOWED: AtomicBool = AtomicBool::new(true);
pub fn set_capture_allowed(allowed: bool) {
    CAPTURE_ALLOWED.store(allowed, Ordering::Release);
}
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
    fn CFRunLoopGetCurrent() -> Ref;
    fn CFRunLoopAddSource(loop_: Ref, source: Ref, mode: Ref);
    fn CFRunLoopRemoveSource(loop_: Ref, source: Ref, mode: Ref);
    fn CFRunLoopPerformBlock(loop_: Ref, mode: Ref, block: &Block<dyn Fn()>);
    fn CFRunLoopWakeUp(loop_: Ref);
    fn CFRunLoopRun();
    fn CFRunLoopStop(loop_: Ref);
    fn CFRetain(value: Ref) -> Ref;
    fn CFRelease(value: Ref);
    static kCFRunLoopCommonModes: Ref;
}
struct Context {
    swallow: bool,
    cycling: bool,
    enabled: Arc<AtomicBool>,
    disabled: Arc<AtomicBool>,
}
// Only a retained CFRunLoop handle crosses threads; public Stop/WakeUp are thread-safe.
// No AppKit/Objective-C objects cross threads and no unsafe Send/Sync is introduced.
struct LoopHandle(usize);
impl Drop for LoopHandle {
    fn drop(&mut self) {
        unsafe {
            CFRelease(self.0 as Ref);
        }
    }
}
pub struct WindowHotkey {
    loop_: LoopHandle,
    worker: Option<thread::JoinHandle<()>>,
    enabled: Arc<AtomicBool>,
    disabled: Arc<AtomicBool>,
}
impl WindowHotkey {
    pub fn start() -> Result<Self, String> {
        if !crate::windows::trusted() {
            return Err("Accessibilité requise pour remplacer ⌘Tab.".into());
        }
        let enabled = Arc::new(AtomicBool::new(true));
        let disabled = Arc::new(AtomicBool::new(false));
        let (sender, receiver) = mpsc::sync_channel::<Result<LoopHandle, String>>(1);
        let worker_enabled = enabled.clone();
        let worker_disabled = disabled.clone();
        let worker = thread::Builder::new()
            .name("kigi-window-shortcut".into())
            .spawn(move || {
                let mut context = Box::new(Context {
                    swallow: false,
                    cycling: false,
                    enabled: worker_enabled.clone(),
                    disabled: worker_disabled,
                });
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
                        let _ = sender.send(Err(
                            "macOS refuse le remplacement de ⌘Tab. Vérifiez Accessibilité.".into(),
                        ));
                        return;
                    }
                    let source = CFMachPortCreateRunLoopSource(ptr::null(), port, 0);
                    if source.is_null() {
                        CFMachPortInvalidate(port);
                        CFRelease(port);
                        let _ = sender.send(Err("Source ⌘Tab indisponible.".into()));
                        return;
                    }
                    let loop_ = CFRunLoopGetCurrent();
                    CFRunLoopAddSource(loop_, source, kCFRunLoopCommonModes);
                    CGEventTapEnable(port, true);
                    let address = loop_ as usize;
                    let ready_enabled = worker_enabled.clone();
                    let ready: RcBlock<dyn Fn()> = RcBlock::new(move || {
                        let handle = LoopHandle(CFRetain(address as Ref) as usize);
                        if sender.send(Ok(handle)).is_err()
                            || !ready_enabled.load(Ordering::Acquire)
                        {
                            CFRunLoopStop(address as Ref);
                        }
                    });
                    CFRunLoopPerformBlock(loop_, kCFRunLoopCommonModes, &ready);
                    CFRunLoopRun();
                    CGEventTapEnable(port, false);
                    CFRunLoopRemoveSource(loop_, source, kCFRunLoopCommonModes);
                    CFMachPortInvalidate(port);
                    CFRelease(source);
                    CFRelease(port);
                    if worker_enabled.swap(false, Ordering::AcqRel) {
                        on_main(|| crate::app::schedule_personal_resume());
                    }
                }
            })
            .map_err(|_| "Impossible de démarrer le raccourci système.".to_string())?;
        match receiver.recv_timeout(Duration::from_secs(3)) {
            Ok(Ok(loop_)) => Ok(Self {
                loop_,
                worker: Some(worker),
                enabled,
                disabled,
            }),
            Ok(Err(message)) => {
                enabled.store(false, Ordering::Release);
                let _ = worker.join();
                Err(message)
            }
            Err(_) => {
                enabled.store(false, Ordering::Release);
                Err("Le raccourci système ne répond pas. L’option reste inactive.".into())
            }
        }
    }
    pub fn active(&self) -> bool {
        self.enabled.load(Ordering::Acquire) && !self.disabled.load(Ordering::Acquire)
    }
}
impl Drop for WindowHotkey {
    fn drop(&mut self) {
        self.enabled.store(false, Ordering::Release);
        unsafe {
            CFRunLoopStop(self.loop_.0 as Ref);
            CFRunLoopWakeUp(self.loop_.0 as Ref);
        }
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}
fn on_main(action: impl Fn() + Send + 'static) {
    let block: RcBlock<dyn Fn()> = RcBlock::new(action);
    unsafe {
        let main = CFRunLoopGetMain();
        CFRunLoopPerformBlock(main, kCFRunLoopCommonModes, &block);
        CFRunLoopWakeUp(main);
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
        context.disabled.store(true, Ordering::Release);
        on_main(|| crate::app::schedule_personal_resume());
        return event;
    }
    if event.is_null() || !context.enabled.load(Ordering::Acquire) {
        return event;
    }
    let flags = unsafe { CGEventGetFlags(event) };
    if kind == 12 {
        if flags & (1 << 20) == 0 && context.cycling {
            context.cycling = false;
            on_main(|| {
                crate::app::queue_window_cycle(false, true);
            });
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
        && CAPTURE_ALLOWED.load(Ordering::Acquire)
    {
        let reverse = flags & (1 << 17) != 0;
        context.swallow = true;
        context.cycling = true;
        on_main(move || {
            crate::app::queue_window_cycle(reverse, false);
        });
        return ptr::null_mut();
    }
    event
}
