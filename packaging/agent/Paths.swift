// Filesystem paths shared by the capture panel and the menu bar. Foundation
// only (no AppKit) so the test binary links without frameworks.
import Foundation

/// TODO_FILE from the user's login shell (LaunchAgents get no shell env),
/// falling back to ~/todo.txt.
func resolveTodoFile() -> URL {
    let p = Process()
    p.executableURL = URL(fileURLWithPath: "/bin/zsh")
    p.arguments = ["-lc", "printf %s \"${TODO_FILE:-$HOME/todo.txt}\""]
    let pipe = Pipe()
    p.standardOutput = pipe
    var todo = FileManager.default.homeDirectoryForCurrentUser
        .appendingPathComponent("todo.txt")
    if (try? p.run()) != nil {
        p.waitUntilExit()
        let data = pipe.fileHandleForReading.readDataToEndOfFile()
        if let s = String(data: data, encoding: .utf8), !s.isEmpty {
            todo = URL(fileURLWithPath: s)
        }
    }
    return todo
}

/// inbox.txt sibling of TODO_FILE (quick-capture drop point).
func resolveInbox() -> URL {
    resolveTodoFile().deletingLastPathComponent().appendingPathComponent("inbox.txt")
}

/// The `prumo` binary in the OUTER app bundle's Resources — a sibling of this
/// nested agent app. resourcePath is
/// .../Prumo.app/Contents/Resources/PrumoAgent.app/Contents/Resources ;
/// three parent hops reach .../Prumo.app/Contents/Resources.
func resolvePrumoBinary() -> URL {
    URL(fileURLWithPath: Bundle.main.resourcePath ?? "")
        .deletingLastPathComponent()   // Contents
        .deletingLastPathComponent()   // PrumoAgent.app
        .deletingLastPathComponent()   // outer Resources
        .appendingPathComponent("prumo")
}
