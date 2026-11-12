//! Opt-in mouse horizontal-scroll filter. Public phase-based inference, not vendor identification.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum Source {
    Mouse,
    Trackpad,
    #[default]
    Unknown,
}
#[derive(Default)]
struct Stream {
    source: Source,
    may_begin: bool,
    ended: bool,
    momentum: bool,
    last_ms: u64,
}
impl Stream {
    fn classify(&mut self, phase: u64, momentum: u64, continuous: bool, ms: u64) -> Source {
        // AppKit NSEventPhase values: began=1, changed=4, ended=8,
        // cancelled=16, mayBegin=32. Unknown/incomplete streams pass through.
        let gap = ms.saturating_sub(self.last_ms);
        if ms < self.last_ms || (gap > 2000 && !self.may_begin) {
            *self = Self::default();
        }
        self.last_ms = ms;
        if phase & 32 != 0 {
            self.source = Source::Trackpad;
            self.may_begin = true;
            self.ended = false;
            self.momentum = false;
        } else if phase & 1 != 0 {
            self.source = if self.may_begin {
                Source::Trackpad
            } else {
                Source::Mouse
            };
            self.may_begin = false;
            self.ended = false;
            self.momentum = false;
        } else if phase == 0 && momentum == 0 {
            // Legacy wheel mice are non-continuous. Precise phaseless input is ambiguous.
            self.source = if continuous {
                Source::Unknown
            } else {
                Source::Mouse
            };
            self.may_begin = false;
            self.ended = false;
            self.momentum = false;
        }
        if momentum != 0 {
            if momentum & 1 != 0 {
                if !self.ended || gap > 300 {
                    self.source = Source::Unknown;
                }
                self.momentum = true;
            } else if !self.momentum {
                self.source = Source::Unknown;
            }
        }
        let source = self.source;
        if phase & 8 != 0 {
            self.ended = true;
            self.may_begin = false;
        }
        if phase & 16 != 0 || momentum & (8 | 16) != 0 {
            *self = Self {
                last_ms: ms,
                ..Self::default()
            };
        }
        source
    }
}

#[cfg(target_os = "macos")]
mod native {
    use super::{Source, Stream};
    use objc2_app_kit::{NSEvent, NSEventPhase, NSEventType};
    use objc2_core_graphics::{CGEvent, CGEventField as Field};
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
        fn CGEventTapIsEnabled(tap: Ref) -> bool;
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
    struct State {
        clock: Instant,
        stream: Stream,
        disabled: bool,
        mouse: u64,
        trackpad: u64,
        unknown: u64,
        filtered: u64,
        dropped: u64,
    }
    pub struct MouseScrollTap {
        port: Ref,
        source: Ref,
        state: Box<State>,
    }
    impl MouseScrollTap {
        pub fn start(_: objc2::MainThreadMarker) -> Result<Self, String> {
            let mut state = Box::new(State {
                clock: Instant::now(),
                stream: Stream::default(),
                disabled: false,
                mouse: 0,
                trackpad: 0,
                unknown: 0,
                filtered: 0,
                dropped: 0,
            });
            unsafe {
                // Session tap, default (modifiable), scroll-wheel ONLY. No keys or pointer moves.
                let port = CGEventTapCreate(
                    1,
                    0,
                    0,
                    1u64 << NSEventType::ScrollWheel.0,
                    callback,
                    (&mut *state as *mut State).cast(),
                );
                if port.is_null() {
                    return Err("Filtre souris indisponible. Vérifiez Accessibilité puis désactivez/réactivez l'option.".into());
                }
                let source = CFMachPortCreateRunLoopSource(ptr::null(), port, 0);
                if source.is_null() {
                    CFMachPortInvalidate(port);
                    CFRelease(port);
                    return Err("Source du filtre souris indisponible.".into());
                }
                CFRunLoopAddSource(CFRunLoopGetMain(), source, kCFRunLoopCommonModes);
                CGEventTapEnable(port, true);
                Ok(Self {
                    port,
                    source,
                    state,
                })
            }
        }
        pub fn active(&self) -> bool {
            !self.state.disabled && unsafe { CGEventTapIsEnabled(self.port) }
        }
        pub fn report(&self) -> String {
            format!("active={} mouse={} trackpad={} unknown={} horizontal_filtered={} events_dropped={} interrupted={} classification=public_phases_no_vendor_id",
                self.active(), self.state.mouse,self.state.trackpad,self.state.unknown,self.state.filtered,self.state.dropped,self.state.disabled)
        }
    }
    impl Drop for MouseScrollTap {
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
    fn get(event: &CGEvent, field: Field) -> i64 {
        CGEvent::integer_value_field(Some(event), field)
    }
    fn set(event: &CGEvent, field: Field, value: i64) {
        CGEvent::set_integer_value_field(Some(event), field, value);
    }
    const X: [Field; 3] = [
        Field::ScrollWheelEventDeltaAxis2,
        Field::ScrollWheelEventFixedPtDeltaAxis2,
        Field::ScrollWheelEventPointDeltaAxis2,
    ];
    const OTHER: [Field; 6] = [
        Field::ScrollWheelEventDeltaAxis1,
        Field::ScrollWheelEventFixedPtDeltaAxis1,
        Field::ScrollWheelEventPointDeltaAxis1,
        Field::ScrollWheelEventDeltaAxis3,
        Field::ScrollWheelEventFixedPtDeltaAxis3,
        Field::ScrollWheelEventPointDeltaAxis3,
    ];
    /// Zero all three public representations; leave vertical/depth, flags and phase intact.
    fn clear_horizontal(event: &CGEvent) -> bool {
        let changed = X.iter().any(|field| get(&event, *field) != 0);
        if changed {
            for field in X {
                set(&event, field, 0);
            }
        }
        changed
    }
    unsafe extern "C" fn callback(
        _: Ref,
        kind: u32,
        event: *mut CGEvent,
        user: *mut c_void,
    ) -> *mut CGEvent {
        let state = &mut *user.cast::<State>();
        if kind == u32::MAX || kind == u32::MAX - 1 {
            state.disabled = true;
            state.stream = Stream::default();
            crate::app::queue_mouse_filter_interrupted(); // deferred cleanup, never free context here
            return event;
        }
        if state.disabled || kind != NSEventType::ScrollWheel.0 as u32 || event.is_null() {
            return event;
        }
        let cg = &*event;
        let Some(ns) = NSEvent::eventWithCGEvent(cg) else {
            return event;
        };
        if ns.r#type() != NSEventType::ScrollWheel {
            return event;
        }
        let mut phase = ns.phase();
        // CG's public MayBegin is 128, AppKit's is 32. Preserve it if bridging omits it.
        if get(&cg, Field::ScrollWheelEventScrollPhase) & 128 != 0 {
            phase |= NSEventPhase::MayBegin;
        }
        let momentum = ns.momentumPhase();
        let source = state.stream.classify(
            phase.0 as u64,
            momentum.0 as u64,
            get(&cg, Field::ScrollWheelEventIsContinuous) != 0,
            state.clock.elapsed().as_millis() as u64,
        );
        match source {
            Source::Trackpad => {
                state.trackpad += 1;
                return event;
            }
            Source::Unknown => {
                state.unknown += 1;
                return event;
            }
            Source::Mouse => state.mouse += 1,
        }
        if !clear_horizontal(cg) {
            return event;
        }
        state.filtered += 1;
        let boundary = (phase | momentum).intersects(
            NSEventPhase::Began
                | NSEventPhase::Ended
                | NSEventPhase::Cancelled
                | NSEventPhase::MayBegin,
        );
        if !boundary && OTHER.iter().all(|field| get(&cg, *field) == 0) {
            state.dropped += 1;
            ptr::null_mut()
        } else {
            event
        }
    }
    #[cfg(test)]
    mod tests {
        use super::*;
        #[test]
        fn cg_filter_keeps_vertical_and_phase_and_clears_every_horizontal_representation() {
            let event = CGEvent::new(None).expect("CG event allocation");
            CGEvent::set_type(Some(&event), objc2_core_graphics::CGEventType::ScrollWheel);
            for (i, field) in OTHER.iter().enumerate() {
                set(&event, *field, (i + 1) as i64);
            }
            for field in X {
                set(&event, field, 7);
            }
            set(&event, Field::ScrollWheelEventScrollPhase, 4);
            assert!(clear_horizontal(&event));
            for field in X {
                assert_eq!(get(&event, field), 0);
            }
            for (i, field) in OTHER.iter().enumerate() {
                assert_eq!(get(&event, *field), (i + 1) as i64);
            }
            assert_eq!(get(&event, Field::ScrollWheelEventScrollPhase), 4);
            assert!(!clear_horizontal(&event));
        }
    }
}
#[cfg(target_os = "macos")]
pub use native::MouseScrollTap;
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn trackpad_and_its_momentum_pass_while_mouse_and_its_momentum_are_identified() {
        let mut s = Stream::default();
        assert_eq!(s.classify(32, 0, true, 1), Source::Trackpad);
        assert_eq!(s.classify(1, 0, true, 2), Source::Trackpad);
        assert_eq!(s.classify(4, 0, true, 3), Source::Trackpad);
        assert_eq!(s.classify(8, 0, true, 4), Source::Trackpad);
        assert_eq!(s.classify(0, 1, true, 5), Source::Trackpad);
        assert_eq!(s.classify(0, 4, true, 6), Source::Trackpad);
        assert_eq!(s.classify(0, 8, true, 7), Source::Trackpad);
        assert_eq!(s.classify(1, 0, true, 10), Source::Mouse);
        assert_eq!(s.classify(4, 0, true, 11), Source::Mouse);
        assert_eq!(s.classify(8, 0, true, 12), Source::Mouse);
        assert_eq!(s.classify(0, 1, true, 13), Source::Mouse);
        assert_eq!(s.classify(0, 4, true, 14), Source::Mouse);
    }
    #[test]
    fn missing_or_stale_sequences_fail_open_and_cancel_does_not_leak_into_next_device() {
        let mut s = Stream::default();
        assert_eq!(s.classify(4, 0, true, 1), Source::Unknown);
        assert_eq!(s.classify(0, 1, true, 2), Source::Unknown);
        let mut resting = Stream::default();
        resting.classify(32, 0, true, 1);
        assert_eq!(resting.classify(1, 0, true, 5000), Source::Trackpad);
        s.classify(32, 0, true, 3);
        s.classify(16, 0, true, 4);
        assert_eq!(s.classify(1, 0, true, 5), Source::Mouse);
        assert_eq!(s.classify(4, 0, true, 3000), Source::Unknown);
        assert_eq!(s.classify(0, 0, true, 3001), Source::Unknown);
        assert_eq!(s.classify(0, 0, false, 3002), Source::Mouse);
        assert_eq!(s.classify(0, 1, true, 3003), Source::Unknown);
    }
}
