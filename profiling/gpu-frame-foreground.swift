// Target-process metadata only; no Accessibility or screen-image readback.
import AppKit
import CoreGraphics
import Foundation

func emit(_ value: [String: Any], code: Int32 = 0) -> Never {
    do {
        let data = try JSONSerialization.data(withJSONObject: value, options: [.sortedKeys])
        FileHandle.standardOutput.write(data)
        FileHandle.standardOutput.write(Data([0x0a]))
    } catch {
        FileHandle.standardError.write(Data("foreground JSON encoding failed: \(error)\n".utf8))
        exit(3)
    }
    exit(code)
}

guard CommandLine.arguments.count == 2, let pid = Int32(CommandLine.arguments[1]),
      let app = NSRunningApplication(processIdentifier: pid) else {
    emit(["frontmost": false, "error": "target process unavailable"], code: 2)
}
let activationRequested = app.activate(options: [])
let frontPid = NSWorkspace.shared.frontmostApplication?.processIdentifier
let list = CGWindowListCopyWindowInfo([.optionOnScreenOnly, .excludeDesktopElements], kCGNullWindowID) as? [[String: Any]]
let windows = (list ?? []).filter {
    ($0[kCGWindowOwnerPID as String] as? NSNumber)?.int32Value == pid &&
    ($0[kCGWindowLayer as String] as? NSNumber)?.intValue == 0 &&
    ($0[kCGWindowAlpha as String] as? NSNumber)?.doubleValue ?? 1 > 0
}
let target = windows.max { left, right in
    let a = left[kCGWindowBounds as String] as? [String: NSNumber] ?? [:]
    let b = right[kCGWindowBounds as String] as? [String: NSNumber] ?? [:]
    return (a["Width"]?.doubleValue ?? 0) * (a["Height"]?.doubleValue ?? 0) <
           (b["Width"]?.doubleValue ?? 0) * (b["Height"]?.doubleValue ?? 0)
}
let bounds = target?[kCGWindowBounds as String] as? [String: NSNumber] ?? [:]
var result: [String: Any] = [
    "api": "NSWorkspace+NSRunningApplication+CGWindowListCopyWindowInfo",
    "pid": pid, "activation_requested": activationRequested,
    "application_active": app.isActive, "frontmost_pid": frontPid.map { Int($0) } ?? NSNull(),
    "frontmost": app.isActive && frontPid == pid,
    "onscreen_target_windows": windows.count,
    "window_width": bounds["Width"]?.intValue ?? 0,
    "window_height": bounds["Height"]?.intValue ?? 0,
]
if let target = target {
    result["window_id"] = target[kCGWindowNumber as String]
    result["window_owner_pid"] = target[kCGWindowOwnerPID as String]
    result["window_layer"] = target[kCGWindowLayer as String]
    result["window_onscreen"] = target[kCGWindowIsOnscreen as String]
}
if list == nil { result["error"] = "CGWindowListCopyWindowInfo unavailable" }
emit(result)
