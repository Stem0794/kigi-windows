#[cfg(any(target_os = "macos", test))]
mod actions;
#[cfg(target_os = "macos")]
mod app;
#[cfg(target_os = "macos")]
mod applications;
#[cfg(any(target_os = "macos", test))]
mod diagnostics;
#[cfg(any(target_os = "macos", test))]
mod geometry;
#[cfg(any(target_os = "macos", test))]
mod gestures;
#[cfg(target_os = "macos")]
mod hotkey;
#[cfg(target_os = "macos")]
mod trackpad;
#[cfg(target_os = "macos")]
mod windows;

fn main() {
    #[cfg(target_os = "macos")]
    app::run();
    #[cfg(not(target_os = "macos"))]
    {
        eprintln!("Kigi Windows nécessite macOS. Aucun backend natif disponible ici.");
        std::process::exit(1);
    }
}
