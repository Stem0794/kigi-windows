use crate::{
    gestures::{Contact, CornerProbe},
    hotkey::Hotkey,
    trackpad::Tap,
    windows::{self, Snapshot},
};
use objc2::{
    define_class, msg_send,
    rc::Retained,
    runtime::{AnyObject, Sel},
    sel, DefinedClass, MainThreadOnly,
};
use objc2_app_kit::*;
use objc2_foundation::{
    ns_string, MainThreadMarker, NSObject, NSObjectProtocol, NSPoint, NSRect, NSSize, NSString,
};
use std::{
    cell::{Cell, OnceCell, RefCell},
    time::Instant,
};

thread_local! { static CONTROLLER: RefCell<Option<Retained<Controller>>> = const { RefCell::new(None) }; }

#[derive(Default)]
struct State {
    snapshot: RefCell<Option<Snapshot>>,
    status: OnceCell<Retained<NSMenuItem>>,
    local_window: RefCell<Option<Retained<NSWindow>>>,
    tap: RefCell<Option<Tap>>,
}
define_class!(
    // NSObject has no subclass requirements. All UI and ivars stay on main.
    #[unsafe(super = NSObject)]
    #[thread_kind = MainThreadOnly]
    #[ivars = State]
    struct Controller;
    unsafe impl NSObjectProtocol for Controller {}
    impl Controller {
        #[unsafe(method(permission:))]
        fn permission(&self, _: Option<&AnyObject>) {
            let granted = windows::request_permission();
            self.status(if granted { "Accessibilité autorisée." } else { "Autorisez Kigi dans Réglages Système > Confidentialité et sécurité > Accessibilité, puis réessayez." });
        }
        #[unsafe(method(inventory:))]
        fn inventory(&self, _: Option<&AnyObject>) {
            match windows::inventory() {
                Ok(text) => alert(self.mtm(), "Fenêtres détectées (sans titres)", &text),
                Err(e) => self.status(&e),
            }
        }
        #[unsafe(method(testWindow:))]
        fn test_window(&self, _: Option<&AnyObject>) {
            let mut snapshot = self.ivars().snapshot.borrow_mut();
            let result = if let Some(saved) = snapshot.as_ref() {
                let result = saved.restore();
                if result.is_ok() { snapshot.take(); }
                result
            } else {
                windows::move_frontmost(self.mtm(), &mut snapshot)
            };
            drop(snapshot);
            self.status(&result.unwrap_or_else(|e| e));
        }
        #[unsafe(method(localProbe:))]
        fn local_probe(&self, _: Option<&AnyObject>) {
            let mtm = self.mtm();
            // New probe each opening; no stale finger identity across sessions.
            let window = unsafe {
                NSWindow::initWithContentRect_styleMask_backing_defer(NSWindow::alloc(mtm),
                    NSRect::new(NSPoint::new(0.0,0.0),NSSize::new(600.0,240.0)),
                    NSWindowStyleMask::Titled | NSWindowStyleMask::Closable,
                    NSBackingStoreType::Buffered, false)
            };
            unsafe { window.setReleasedWhenClosed(false) };
            window.setTitle(ns_string!("Trackpad local — aucun geste global activé"));
            let view = TouchView::new(mtm);
            let text = NSTextField::labelWithString(ns_string!("Diagnostic local uniquement.\nGardez le pointeur dans cette fenêtre, puis levez les doigts.\nPosez un doigt dans le coin haut-gauche : les 10 % les plus à gauche. Glissez vers le bas.\nLe résultat s'affiche ici ; aucune fenêtre tierce n'est déplacée."), mtm);
            text.setFrame(NSRect::new(NSPoint::new(20.0,35.0),NSSize::new(560.0,170.0)));
            view.addSubview(&text);
            // hitTest routes the whole diagnostic area to TouchView.
            // Keep the informational label non-selectable and non-editable.
            text.setSelectable(false);
            text.setEditable(false);
            view.ivars().label.set(text).ok();
            window.setContentView(Some(&view));
            window.center();
            window.makeKeyAndOrderFront(None);
            #[allow(deprecated)]
            NSApplication::sharedApplication(mtm).activateIgnoringOtherApps(true);
            let previous = self.ivars().local_window.replace(Some(window));
            if let Some(old) = previous { old.close(); }
        }
        #[unsafe(method(globalProbe:))]
        fn global_probe(&self, _: Option<&AnyObject>) {
            let old = self.ivars().tap.borrow_mut().take();
            if let Some(tap) = old {
                let report = tap.report();
                drop(tap);
                self.status("Diagnostic global arrêté.");
                alert(self.mtm(), "Résultat du spike global", &report);
            } else {
                match Tap::start() {
                    Ok(tap) => {
                        self.ivars().tap.replace(Some(tap));
                        self.status("Diagnostic global en cours. Testez le geste dans Chrome/Teams, puis recliquez sur Démarrer/arrêter pour voir le résultat.");
                    }
                    Err(e) => self.status(&e),
                }
            }
        }
        #[unsafe(method(quit:))]
        fn quit(&self, _: Option<&AnyObject>) {
            self.ivars().tap.borrow_mut().take();
            NSApplication::sharedApplication(self.mtm()).stop(None);
        }
    }
);
impl Controller {
    fn new(mtm: MainThreadMarker) -> Retained<Self> {
        let this = Self::alloc(mtm).set_ivars(State::default());
        unsafe { msg_send![super(this), init] }
    }
    fn status(&self, text: &str) {
        if let Some(item) = self.ivars().status.get() {
            item.setTitle(&NSString::from_str(text));
        }
    }
}

struct TouchState {
    probe: RefCell<CornerProbe>,
    clock: Instant,
    count: Cell<u64>,
    last_ui: Cell<Instant>,
    label: OnceCell<Retained<NSTextField>>,
}
define_class!(
    #[unsafe(super = NSView)]
    #[thread_kind = MainThreadOnly]
    #[ivars = TouchState]
    struct TouchView;
    unsafe impl NSObjectProtocol for TouchView {}
    impl TouchView {
        // The diagnostic label must not steal the trackpad's hit-tested view.
        #[unsafe(method(hitTest:))]
        fn hit_test(&self, point: NSPoint) -> Option<&NSView> {
            let hit: Option<Retained<NSView>> = unsafe { msg_send![super(self), hitTest:point] };
            hit.map(|_| &**self)
        }
        #[unsafe(method(touchesBeganWithEvent:))]
        fn began(&self, event: &NSEvent) { self.touch(event, true); }
        #[unsafe(method(touchesMovedWithEvent:))]
        fn moved(&self, event: &NSEvent) { self.touch(event, false); }
        #[unsafe(method(touchesEndedWithEvent:))]
        fn ended(&self, _: &NSEvent) { self.ivars().probe.borrow_mut().cancel(); }
        #[unsafe(method(touchesCancelledWithEvent:))]
        fn cancelled(&self, _: &NSEvent) { self.ivars().probe.borrow_mut().cancel(); }
    }
);
impl TouchView {
    fn new(mtm: MainThreadMarker) -> Retained<Self> {
        let this = Self::alloc(mtm).set_ivars(TouchState {
            probe: RefCell::new(CornerProbe::default()),
            clock: Instant::now(),
            count: Cell::new(0),
            last_ui: Cell::new(Instant::now()),
            label: OnceCell::new(),
        });
        let view: Retained<Self> = unsafe {
            msg_send![super(this), initWithFrame:NSRect::new(NSPoint::new(0.0,0.0),NSSize::new(600.0,240.0))]
        };
        #[allow(deprecated)]
        view.setAcceptsTouchEvents(true);
        view.setAllowedTouchTypes(NSTouchTypeMask::Indirect);
        view.setWantsRestingTouches(true);
        view
    }
    fn touch(&self, event: &NSEvent, began: bool) {
        let touches = event.touchesMatchingPhase_inView(NSTouchPhase::Touching, Some(self));
        let mut probe = self.ivars().probe.borrow_mut();
        if touches.count() != 1 {
            probe.cancel();
            return;
        }
        let Some(touch) = touches.anyObject() else {
            return;
        };
        let point = touch.normalizedPosition();
        let identity = touch.identity();
        let key: usize = unsafe { msg_send![&*identity, hash] };
        let contact = Contact {
            identity: key,
            device: touch
                .device()
                .map(|d| Retained::as_ptr(&d) as usize)
                .unwrap_or(0),
            x: point.x,
            y: point.y,
            ms: self.ivars().clock.elapsed().as_millis() as u64,
        };
        if began {
            if touch.phase() == NSTouchPhase::Began {
                probe.begin(contact, 1);
            } else {
                probe.cancel();
            }
        } else if probe.moved(contact, 1) {
            self.ivars().count.set(self.ivars().count.get() + 1);
        }
        if self.ivars().last_ui.get().elapsed().as_millis() >= 33 {
            self.ivars().last_ui.set(Instant::now());
            if let Some(label) = self.ivars().label.get() {
                label.setStringValue(&NSString::from_str(&format!(
                    "Diagnostic LOCAL : x={:.2}, y={:.2}, phase={:?}.\nGeste(s) haut-gauche vers bas : {}.\nLevez les doigts pour recommencer. Aucune action globale.",
                    point.x, point.y, touch.phase(), self.ivars().count.get()
                )));
            }
        }
    }
}
fn alert(mtm: MainThreadMarker, title: &str, text: &str) {
    let alert = NSAlert::new(mtm);
    alert.setMessageText(&NSString::from_str(title));
    alert.setInformativeText(&NSString::from_str(text));
    alert.runModal();
}
fn menu_action(menu: &NSMenu, controller: &Controller, title: &str, action: Sel) {
    let item = unsafe {
        NSMenuItem::initWithTitle_action_keyEquivalent(
            NSMenuItem::alloc(controller.mtm()),
            &NSString::from_str(title),
            Some(action),
            ns_string!(""),
        )
    };
    unsafe {
        item.setTarget(Some(controller));
    }
    menu.addItem(&item);
}
pub fn schedule_test() {
    CONTROLLER.with(|slot| {
        if let Some(controller) = slot.borrow().as_ref() {
            // NSObject schedules this on its current main run loop.
            let _: () = unsafe { msg_send![&**controller, performSelector:sel!(testWindow:), withObject:std::ptr::null::<AnyObject>(), afterDelay:0.0f64] };
        }
    });
}
pub fn run() {
    let mtm = MainThreadMarker::new().expect("AppKit doit démarrer sur le thread principal");
    objc2::rc::autoreleasepool(|_| {
        let app = NSApplication::sharedApplication(mtm);
        app.setActivationPolicy(NSApplicationActivationPolicy::Accessory);
        let controller = Controller::new(mtm);
        let status_bar = NSStatusBar::systemStatusBar();
        let status_item = status_bar.statusItemWithLength(-1.0);
        if let Some(button) = status_item.button(mtm) {
            button.setTitle(ns_string!("Kigi"));
        }
        let menu = NSMenu::new(mtm);
        let status = NSMenuItem::new(mtm);
        status.setTitle(ns_string!(
            "P0 — prêt. Accessibilité demandée uniquement sur action."
        ));
        status.setEnabled(false);
        menu.addItem(&status);
        controller.ivars().status.set(status).ok();
        menu_action(
            &menu,
            &controller,
            "Autoriser / vérifier Accessibilité",
            sel!(permission:),
        );
        menu_action(
            &menu,
            &controller,
            "Détecter les fenêtres (sans titres)",
            sel!(inventory:),
        );
        menu_action(
            &menu,
            &controller,
            "Tester / restaurer la fenêtre active — ⌃⌥T",
            sel!(testWindow:),
        );
        menu.addItem(&NSMenuItem::separatorItem(mtm));
        menu_action(
            &menu,
            &controller,
            "Tester le geste dans une fenêtre locale",
            sel!(localProbe:),
        );
        menu_action(
            &menu,
            &controller,
            "Démarrer / arrêter le diagnostic global passif",
            sel!(globalProbe:),
        );
        menu.addItem(&NSMenuItem::separatorItem(mtm));
        menu_action(&menu, &controller, "Quitter", sel!(quit:));
        status_item.setMenu(Some(&menu));
        CONTROLLER.with(|c| c.replace(Some(controller.clone())));
        let hotkey = Hotkey::register();
        if let Err(e) = &hotkey {
            controller.status(e);
        }
        app.run();
        drop(hotkey);
        CONTROLLER.with(|c| c.borrow_mut().take());
        controller.ivars().tap.borrow_mut().take();
        if let Some(window) = controller.ivars().local_window.borrow_mut().take() {
            window.close();
        }
        status_bar.removeStatusItem(&status_item);
    });
}
