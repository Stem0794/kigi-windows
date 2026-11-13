use super::bridge;
use block2::DynBlock;
use objc2::{
    define_class, msg_send,
    rc::Retained,
    runtime::{AnyObject, ProtocolObject},
    DefinedClass, MainThreadOnly,
};
use objc2_app_kit::{NSApplication, NSBackingStoreType, NSScreen, NSWindow, NSWindowStyleMask};
use objc2_foundation::{
    MainThreadMarker, NSBundle, NSError, NSObject, NSObjectProtocol, NSPoint, NSRect, NSSize,
    NSString,
};
use objc2_web_kit::*;
use std::{cell::Cell, time::Instant};

struct HandlerState {
    entry: String,
    view: Cell<usize>,
    ready: Cell<bool>,
    opened: Instant,
}
define_class!(
    #[unsafe(super = NSObject)]
    #[thread_kind = MainThreadOnly]
    #[ivars = HandlerState]
    struct Handler;
    unsafe impl NSObjectProtocol for Handler {}
    unsafe impl WKScriptMessageHandlerWithReply for Handler {
        #[unsafe(method(userContentController:didReceiveScriptMessage:replyHandler:))]
        unsafe fn receive(
            &self,
            _: &WKUserContentController,
            message: &WKScriptMessage,
            reply: &DynBlock<dyn Fn(*mut AnyObject, *mut NSString)>,
        ) {
            let frame = message.frameInfo();
            let url = frame
                .request()
                .URL()
                .and_then(|u| u.absoluteString())
                .map(|u| u.to_string())
                .unwrap_or_default();
            let same_view = message
                .webView()
                .is_some_and(|v| (&*v as *const WKWebView as usize) == self.ivars().view.get());
            let result = if !same_view
                || !bridge::permitted_document(frame.isMainFrame(), &url, &self.ivars().entry)
            {
                bridge::error(None, "FORBIDDEN_SOURCE", "Document non autorisé.")
            } else if let Ok(raw) = message.body().downcast::<NSString>() {
                if raw.length() > 64 * 1024 {
                    bridge::error(None, "PAYLOAD_TOO_LARGE", "Requête trop volumineuse.")
                } else {
                    match bridge::parse(&raw.to_string()) {
                        Ok(request) => {
                            let result = crate::app::studio_command(&request.method);
                            if request.method == "app.bootstrap"
                                && result.is_ok()
                                && !self.ivars().ready.replace(true)
                            {
                                crate::diagnostics::record("studio.ready",format!("elapsed_ms={} rust_rss_bytes={:?} webkit_memory=unmeasured",self.ivars().opened.elapsed().as_millis(),crate::diagnostics::rss_bytes()));
                                crate::app::studio_loaded();
                                if std::env::args().any(|a| a == "--studio-check") {
                                    println!(
                                        "KIGI_STUDIO_READY {}",
                                        serde_json::json!({"version":env!("CARGO_PKG_VERSION"),"protocolVersion":1,"rust_rss_bytes":crate::diagnostics::rss_bytes()})
                                    );
                                }
                            }
                            match result {
                                Ok(data) => bridge::success(&request.id, data),
                                Err(message) => bridge::error(
                                    Some(&request.id),
                                    "BACKEND_UNAVAILABLE",
                                    &message,
                                ),
                            }
                        }
                        Err(error) => error,
                    }
                }
            } else {
                bridge::error(None, "INVALID_REQUEST", "Une enveloppe JSON est requise.")
            };
            let text = NSString::from_str(&result.to_string());
            reply.call((
                (&*text as *const NSString).cast_mut().cast(),
                std::ptr::null_mut(),
            ));
        }
    }
    unsafe impl WKNavigationDelegate for Handler {
        #[unsafe(method(webView:decidePolicyForNavigationAction:decisionHandler:))]
        unsafe fn navigation(
            &self,
            _: &WKWebView,
            action: &WKNavigationAction,
            decision: &DynBlock<dyn Fn(WKNavigationActionPolicy)>,
        ) {
            let url = action
                .request()
                .URL()
                .and_then(|u| u.absoluteString())
                .map(|u| u.to_string())
                .unwrap_or_default();
            let main = action.targetFrame().is_some_and(|f| f.isMainFrame());
            decision.call((
                if bridge::permitted_document(main, &url, &self.ivars().entry) {
                    WKNavigationActionPolicy::Allow
                } else {
                    WKNavigationActionPolicy::Cancel
                },
            ));
        }
        #[unsafe(method(webView:didFailProvisionalNavigation:withError:))]
        unsafe fn failed(&self, _: &WKWebView, _: Option<&WKNavigation>, _: &NSError) {
            crate::app::studio_failed();
        }
        #[unsafe(method(webView:didFailNavigation:withError:))]
        unsafe fn committed_failed(&self, _: &WKWebView, _: Option<&WKNavigation>, _: &NSError) {
            crate::app::studio_failed();
        }
        #[unsafe(method(webViewWebContentProcessDidTerminate:))]
        unsafe fn terminated(&self, _: &WKWebView) {
            self.ivars().ready.set(false);
            crate::app::studio_failed();
        }
    }
);
pub struct Studio {
    pub window: Retained<NSWindow>,
    view: Retained<WKWebView>,
    handler: Retained<Handler>,
    content: Retained<WKUserContentController>,
}
impl Studio {
    pub fn new(mtm: MainThreadMarker) -> Result<Self, String> {
        let root = NSBundle::mainBundle()
            .resourceURL()
            .and_then(|u| {
                u.URLByAppendingPathComponent_isDirectory(&NSString::from_str("studio"), true)
            })
            .ok_or("Ressources Studio absentes.")?;
        let entry = root
            .URLByAppendingPathComponent_isDirectory(&NSString::from_str("index.html"), false)
            .ok_or("Entrée Studio absente.")?;
        let path = entry.path().ok_or("Entrée Studio indisponible.")?;
        if !std::path::Path::new(&path.to_string()).is_file() {
            return Err("Ressources Studio absentes du bundle.".into());
        }
        let opened = Instant::now();
        let handler = Handler::alloc(mtm).set_ivars(HandlerState {
            entry: entry
                .absoluteString()
                .ok_or("URL Studio invalide.")?
                .to_string(),
            view: Cell::new(0),
            ready: Cell::new(false),
            opened,
        });
        let handler: Retained<Handler> = unsafe { msg_send![super(handler), init] };
        let (view, content) = unsafe {
            let configuration = WKWebViewConfiguration::new(mtm);
            configuration.setWebsiteDataStore(&WKWebsiteDataStore::nonPersistentDataStore(mtm));
            configuration
                .preferences()
                .setJavaScriptCanOpenWindowsAutomatically(false);
            let content = WKUserContentController::new(mtm);
            content.addScriptMessageHandlerWithReply_contentWorld_name(
                ProtocolObject::from_ref(&*handler),
                &WKContentWorld::pageWorld(mtm),
                &NSString::from_str("kigi"),
            );
            configuration.setUserContentController(&content);
            let view = WKWebView::initWithFrame_configuration(
                WKWebView::alloc(mtm),
                NSRect::new(NSPoint::new(0., 0.), NSSize::new(1240., 800.)),
                &configuration,
            );
            handler
                .ivars()
                .view
                .set(&*view as *const WKWebView as usize);
            view.setNavigationDelegate(Some(ProtocolObject::from_ref(&*handler)));
            (view, content)
        };
        let available = NSScreen::mainScreen(mtm)
            .map(|s| s.visibleFrame().size)
            .unwrap_or(NSSize::new(1240., 800.));
        let frame = NSRect::new(
            NSPoint::new(0., 0.),
            NSSize::new(
                1240_f64.min(available.width),
                800_f64.min(available.height - 30.),
            ),
        );
        let window = unsafe {
            NSWindow::initWithContentRect_styleMask_backing_defer(
                NSWindow::alloc(mtm),
                frame,
                NSWindowStyleMask::Titled
                    | NSWindowStyleMask::Closable
                    | NSWindowStyleMask::Miniaturizable
                    | NSWindowStyleMask::Resizable,
                NSBackingStoreType::Buffered,
                false,
            )
        };
        unsafe {
            window.setReleasedWhenClosed(false);
        }
        window.setTitle(&NSString::from_str("Kigi Studio"));
        window.setContentMinSize(NSSize::new(760., 520.));
        window.setContentView(Some(&view));
        window.center();
        unsafe {
            view.loadFileURL_allowingReadAccessToURL(&entry, &root);
        }
        Ok(Self {
            window,
            view,
            handler,
            content,
        })
    }
    pub fn check_render(&self) {
        // Fixed test-only expression; no frontend-provided JavaScript is executed.
        let script=NSString::from_str("JSON.stringify({title:document.getElementById('page-title')?.textContent,tabs:document.querySelectorAll('#navigation button').length,busy:document.getElementById('content').getAttribute('aria-busy'),error:!document.getElementById('error').hidden})");
        let reply = block2::RcBlock::new(|value: *mut AnyObject, error: *mut NSError| {
            if error.is_null() {
                if let Some(value) = unsafe { value.as_ref() } {
                    if let Some(text) = value.downcast_ref::<NSString>() {
                        println!("KIGI_STUDIO_RENDER {}", text);
                    }
                }
            }
        });
        unsafe {
            self.view
                .evaluateJavaScript_completionHandler(&script, Some(&reply));
        }
    }
    pub fn ready(&self) -> bool {
        self.handler.ivars().ready.get()
    }
    pub fn show(&self, mtm: MainThreadMarker) {
        if self.window.isMiniaturized() {
            self.window.deminiaturize(None);
        }
        self.window.makeKeyAndOrderFront(None);
        #[allow(deprecated)]
        NSApplication::sharedApplication(mtm).activateIgnoringOtherApps(true);
    }
}
impl Drop for Studio {
    fn drop(&mut self) {
        unsafe {
            self.view.stopLoading();
            self.view.setNavigationDelegate(None);
            self.content.removeScriptMessageHandlerForName_contentWorld(
                &NSString::from_str("kigi"),
                &WKContentWorld::pageWorld(self.window.mtm()),
            );
            self.window.close();
        }
    }
}
