// CI-only controlled windows. No screenshots, user content or synthetic input.
import AppKit
import CoreServices
import Carbon
import CoreGraphics
import Darwin

if let index = CommandLine.arguments.firstIndex(of: "--register") {
    for path in CommandLine.arguments.dropFirst(index + 1) {
        let status = LSRegisterURL(URL(fileURLWithPath: path) as CFURL, true)
        if status != noErr { exit(1) }
        guard let bundle = Bundle(url: URL(fileURLWithPath: path))?.bundleIdentifier else { exit(1) }
        let expected = URL(fileURLWithPath: path).standardizedFileURL.resolvingSymlinksInPath()
        let deadline = Date().addingTimeInterval(3)
        var found = false
        repeat {
            if let registered = NSWorkspace.shared.urlForApplication(withBundleIdentifier: bundle),
               registered.standardizedFileURL.resolvingSymlinksInPath() == expected {
                found = true; break
            }
            RunLoop.current.run(until: Date().addingTimeInterval(0.05))
        } while Date() < deadline
        if !found { fputs("Fixture registration is not discoverable\n", stderr); exit(1) }
    }
    exit(0)
}
if CommandLine.arguments.contains("--quit-test-apps") {
    for bundle in ["io.kigi.focus.source", "io.kigi.focus.target"] {
        for app in NSRunningApplication.runningApplications(withBundleIdentifier: bundle) { app.terminate() }
    }
    exit(0)
}
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
    var meetingWindow: NSWindow?
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
        if CommandLine.arguments.contains("--teams-pair") {
            window.title = "Kigi chat fixture"
            let meeting = NSWindow(contentRect: NSRect(x: 200, y: 140, width: 650, height: 450), styleMask: [.titled, .closable, .resizable, .miniaturizable], backing: .buffered, defer: false)
            meeting.title = "Kigi meeting fixture"
            if CommandLine.arguments.contains("--meeting-minimum") { meeting.contentMinSize = NSSize(width: screen.width * 0.60, height: 200) }
            meeting.isReleasedWhenClosed = false
            meeting.makeKeyAndOrderFront(nil)
            meetingWindow = meeting
        }
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
