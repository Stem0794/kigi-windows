// CI-only controlled windows. No screenshots, user content or synthetic input.
import AppKit
import Carbon
import CoreGraphics
import Darwin

if CommandLine.arguments.contains("--hotkey-reservation") {
    let app = NSApplication.shared
    app.setActivationPolicy(.accessory)
    var reference: EventHotKeyRef?
    let code = RegisterEventHotKey(18, UInt32(cmdKey | optionKey | controlKey), EventHotKeyID(signature: 0x4b465453, id: 1), GetApplicationEventTarget(), UInt32(kEventHotKeyExclusive), &reference)
    print("HOTKEY_READY \(code)")
    fflush(stdout)
    if code != 0 { exit(1) }
    app.run()
    if let reference = reference { UnregisterEventHotKey(reference) }
    exit(0)
}
if CommandLine.arguments.contains("--window-order") {
    let windows = CGWindowListCopyWindowInfo([.optionOnScreenOnly, .excludeDesktopElements], kCGNullWindowID) as? [[String: Any]] ?? []
    let owners = windows.compactMap { w -> Int? in
        guard (w[kCGWindowLayer as String] as? Int) == 0 else { return nil }
        return w[kCGWindowOwnerPID as String] as? Int
    }
    let data = try JSONSerialization.data(withJSONObject: ["owners": owners])
    print(String(data: data, encoding: .utf8)!)
    exit(0)
}
if CommandLine.arguments.contains("--window-frames") {
    let pids = Set(CommandLine.arguments.compactMap { Int($0) })
    let windows = CGWindowListCopyWindowInfo([.optionOnScreenOnly, .excludeDesktopElements], kCGNullWindowID) as? [[String: Any]] ?? []
    let frames = windows.compactMap { w -> [String: Any]? in
        guard (w[kCGWindowLayer as String] as? Int) == 0, let pid = w[kCGWindowOwnerPID as String] as? Int, pids.contains(pid), let frame = w[kCGWindowBounds as String] as? [String: Any] else { return nil }
        return ["pid": pid, "frame": frame]
    }
    let data = try JSONSerialization.data(withJSONObject: frames)
    print(String(data: data, encoding: .utf8)!)
    exit(0)
}
class Fixture: NSObject, NSApplicationDelegate {
    var window: NSWindow!
    var attempts = 0
    let source = CommandLine.arguments.contains("--source")
    let fullscreen = CommandLine.arguments.contains("--fullscreen")
    func applicationDidFinishLaunching(_ notification: Notification) {
        let screen = NSScreen.main!.visibleFrame
        window = NSWindow(contentRect: NSRect(x: 160, y: 180, width: 620, height: 420), styleMask: [.titled, .closable, .resizable, .miniaturizable], backing: .buffered, defer: false)
        window.title = source ? "Kigi focus source fixture" : "Kigi focus target fixture"
        window.collectionBehavior = [.fullScreenPrimary]
        if CommandLine.arguments.contains("--minimum") { window.contentMinSize = NSSize(width: screen.width * 0.55, height: 200) }
        if source { window.setFrame(screen, display: true) }
        window.makeKeyAndOrderFront(nil)
        NSApp.activate(ignoringOtherApps: true)
        if fullscreen { DispatchQueue.main.asyncAfter(deadline: .now() + 0.2) { self.window.toggleFullScreen(nil) } }
        DispatchQueue.main.asyncAfter(deadline: .now() + 0.4) { self.ready() }
    }
    func ready() {
        attempts += 1
        let mode = NSApp.currentSystemPresentationOptions.contains(.fullScreen)
        if (!NSApp.isActive || mode != fullscreen) && attempts < 40 {
            DispatchQueue.main.asyncAfter(deadline: .now() + 0.1) { self.ready() }; return
        }
        print("FIXTURE_READY \(ProcessInfo.processInfo.processIdentifier) \(NSApp.isActive && mode == fullscreen)")
        fflush(stdout)
    }
}
let delegate = Fixture()
let app = NSApplication.shared
app.setActivationPolicy(.regular)
app.delegate = delegate
app.run()
