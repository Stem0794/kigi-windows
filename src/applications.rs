//! NSWorkspace launch/open, never shell commands or a second app instance.
use block2::RcBlock;
use objc2::{msg_send, rc::Retained, runtime::AnyObject, sel, AnyThread};
use objc2_app_kit::{
    NSApplication, NSApplicationActivationOptions, NSRunningApplication, NSWorkspace,
    NSWorkspaceOpenConfiguration,
};
use objc2_foundation::{
    NSAppleEventDescriptor, NSAppleScript, NSDictionary, NSError, NSObjectProtocol, NSString,
};
use std::sync::mpsc::{self, Receiver};

thread_local! {static CHROME_AUTOMATION:std::cell::Cell<&'static str>=const{std::cell::Cell::new("unknown")};}
pub fn chrome_automation_state() -> &'static str {
    CHROME_AUTOMATION.with(|s| s.get())
}
pub const CHROME: &str = "com.google.Chrome";
const TEAMS: [&str; 2] = ["com.microsoft.teams2", "com.microsoft.teams"];

pub fn running(bundle: &str) -> Option<Retained<NSRunningApplication>> {
    NSRunningApplication::runningApplicationsWithBundleIdentifier(&NSString::from_str(bundle))
        .iter()
        .find(|a| !a.isTerminated())
}
pub fn teams_bundle() -> Result<&'static str, String> {
    // Prefer an existing main process, then an installed main bundle. No helper/name guesses.
    TEAMS
        .into_iter()
        .find(|b| running(b).is_some())
        .or_else(|| {
            TEAMS.into_iter().find(|b| {
                NSWorkspace::sharedWorkspace()
                    .URLForApplicationWithBundleIdentifier(&NSString::from_str(b))
                    .is_some()
            })
        })
        .ok_or_else(|| "Microsoft Teams n'est pas installé (bundle principal introuvable).".into())
}
/// Request acceptance is not proof of focus. Background AppKit requests can
/// return false while the authorized AX route can still activate the window.
#[allow(deprecated)]
pub fn request_activation(app: &NSRunningApplication) -> Result<bool, String> {
    if app.isTerminated() {
        return Err("L'application vient de quitter.".into());
    }
    // Fixed native fixture only; unavailable in normal operation.
    if std::env::args().any(|a| a == "--activation-check")
        && std::env::var_os("KIGI_TEST_DENY_APPKIT_ACTIVATION").is_some()
    {
        crate::diagnostics::record(
            "application.activation.request",
            format!(
                "pid={} accepted=false fixture=true",
                app.processIdentifier()
            ),
        );
        return Ok(false);
    }
    app.unhide();
    if let Some(mtm) = objc2::MainThreadMarker::new() {
        let own = NSApplication::sharedApplication(mtm);
        if own.isActive() && own.respondsToSelector(sel!(yieldActivationToApplication:)) {
            own.yieldActivationToApplication(app);
        }
    }
    let accepted = app.activateWithOptions(
        NSApplicationActivationOptions::ActivateIgnoringOtherApps
            | NSApplicationActivationOptions::ActivateAllWindows,
    );
    crate::diagnostics::record(
        "application.activation.request",
        format!(
            "pid={} accepted={accepted} all_windows=true",
            app.processIdentifier()
        ),
    );
    Ok(accepted)
}

pub fn activate(app: &NSRunningApplication) -> Result<(), String> {
    if request_activation(app)? {
        Ok(())
    } else {
        Err("L'application ne peut pas être activée ou vient de quitter.".into())
    }
}

/// Completion runs on Apple's concurrent queue. Only send a small result to main;
/// AppKit UI, AX and the session's diagnostic log remain on the main thread.
pub fn open(bundle: &str) -> Result<Receiver<Result<i32, isize>>, String> {
    let (sender, receiver) = mpsc::channel();
    if let Some(app) = running(bundle) {
        // poll_open confirms focus and raises through Accessibility even if
        // AppKit rejects this initial request. Never reopen an existing app.
        let _ = request_activation(&app)?;
        let _ = sender.send(Ok(app.processIdentifier()));
        return Ok(receiver);
    }
    let workspace = NSWorkspace::sharedWorkspace();
    let application = workspace
        .URLForApplicationWithBundleIdentifier(&NSString::from_str(bundle))
        .ok_or("Application introuvable. Installez-la avant d'utiliser ce raccourci.")?;
    let options = NSWorkspaceOpenConfiguration::configuration();
    options.setCreatesNewApplicationInstance(false);
    options.setActivates(true);
    options.setAddsToRecentItems(false);
    let done = RcBlock::new(move |app: *mut NSRunningApplication, error: *mut NSError| {
        let result = unsafe {
            if let Some(error) = error.as_ref() {
                Err(error.code())
            } else if let Some(app) = app.as_ref() {
                Ok(app.processIdentifier())
            } else {
                Err(0)
            }
        };
        let _ = sender.send(result);
    });
    workspace.openApplicationAtURL_configuration_completionHandler(
        &application,
        &options,
        Some(&done),
    );
    Ok(receiver)
}

/// Fixed script, in-process and main-thread only. No page content/history access.
/// Return only a structural result; never export URL lists or AppleScript error text.
pub fn chatgpt(mtm: objc2::MainThreadMarker) -> Result<String, String> {
    CHROME_AUTOMATION.with(|s| s.set("unknown"));
    let _ = mtm; // enforce AppKit/main-thread execution for NSAppleScript
    if NSWorkspace::sharedWorkspace()
        .URLForApplicationWithBundleIdentifier(&NSString::from_str(CHROME))
        .is_none()
    {
        return Err("Google Chrome n'est pas installé.".into());
    }
    let script = NSAppleScript::initWithSource(
        NSAppleScript::alloc(),
        &NSString::from_str(include_str!("../resources/chatgpt.applescript")),
    )
    .ok_or("Impossible d'initialiser l'intégration Chrome.")?;
    let mut error: *mut NSDictionary<NSString, AnyObject> = std::ptr::null_mut();
    // Apple's API is nullable on failure; the generated 0.3.2 binding incorrectly
    // promises non-null. Specify nullable result and autoreleased error pointer.
    let result: Option<Retained<NSAppleEventDescriptor>> =
        unsafe { msg_send![&*script, executeAndReturnError: &mut error] };
    if !error.is_null() || result.is_none() {
        let code: isize = unsafe {
            if let Some(error) = error.as_ref() {
                let number = error.objectForKey(&NSString::from_str("NSAppleScriptErrorNumber"));
                number.map(|n| msg_send![&*n, integerValue]).unwrap_or(0)
            } else {
                0
            }
        };
        CHROME_AUTOMATION.with(|s| s.set(if code == -1743 { "denied" } else { "unknown" }));
        crate::diagnostics::record("chrome.chatgpt.error", format!("code={code}"));
        return Err(match code {
            -1743 => "Automatisation Chrome refusée. Autorisez Kigi Windows → Google Chrome dans Réglages Système > Confidentialité et sécurité > Automatisation.".into(),
            -1712 => "Chrome n'a pas répondu à temps. Réessayez ; aucun nouvel onglet de secours n'est demandé.".into(),
            _ => format!("Impossible de retrouver ChatGPT dans Chrome (code {code}). Aucun onglet de secours ouvert."),
        });
    }
    CHROME_AUTOMATION.with(|s| s.set("granted"));
    match result.unwrap().int32Value() {
        1 => {
            crate::diagnostics::record("chrome.chatgpt.reused", "existing_tab=true");
            Ok("Onglet ChatGPT existant activé.".into())
        }
        2 => {
            crate::diagnostics::record("chrome.chatgpt.created", "existing_tab=false");
            Ok("Aucun onglet ChatGPT trouvé : nouvel onglet ouvert.".into())
        }
        _ => Err("Réponse Chrome inattendue ; consultez les logs.".into()),
    }
}
