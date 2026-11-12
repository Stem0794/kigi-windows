//! NSWorkspace launch/open, never shell commands or a second app instance.
use block2::RcBlock;
use objc2::rc::Retained;
use objc2_app_kit::{
    NSApplicationActivationOptions, NSRunningApplication, NSWorkspace, NSWorkspaceOpenConfiguration,
};
use objc2_foundation::{NSArray, NSError, NSString, NSURL};
use std::sync::mpsc::{self, Receiver};

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
#[allow(deprecated)]
pub fn activate(app: &NSRunningApplication) -> Result<(), String> {
    app.unhide();
    if app.activateWithOptions(NSApplicationActivationOptions::ActivateIgnoringOtherApps) {
        Ok(())
    } else {
        Err("L'application ne peut pas être activée ou vient de quitter.".into())
    }
}

/// Completion runs on Apple's concurrent queue. Only send a small result to main;
/// AppKit UI, AX and the session's diagnostic log remain on the main thread.
pub fn open(bundle: &'static str, chatgpt: bool) -> Result<Receiver<Result<i32, isize>>, String> {
    let (sender, receiver) = mpsc::channel();
    if !chatgpt {
        if let Some(app) = running(bundle) {
            activate(&app)?;
            let _ = sender.send(Ok(app.processIdentifier()));
            return Ok(receiver);
        }
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
    if chatgpt {
        let url = NSURL::URLWithString(&NSString::from_str("https://chatgpt.com/"))
            .ok_or("URL ChatGPT invalide.")?;
        workspace.openURLs_withApplicationAtURL_configuration_completionHandler(
            &NSArray::from_retained_slice(&[url]),
            &application,
            &options,
            Some(&done),
        );
    } else {
        workspace.openApplicationAtURL_configuration_completionHandler(
            &application,
            &options,
            Some(&done),
        );
    }
    Ok(receiver)
}
