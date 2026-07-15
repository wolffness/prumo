// Native launcher for Tuxedo.app: opens the tuxedo TUI in an iTerm2 (or
// Terminal.app) window and stays alive while it runs, so the Dock icon
// behaves like a real app — no infinite bounce (a bare shell-script
// launcher never reports "finished launching"), a running dot while the
// TUI is open, and clicking the Dock icon again focuses or relaunches it.
import AppKit

final class Launcher: NSObject, NSApplicationDelegate {
    var bin = ""

    func applicationDidFinishLaunching(_ note: Notification) {
        bin = Bundle.main.resourcePath! + "/tuxedo"
        openWindow()
        watchForExit()
    }

    /// Dock icon clicked while running: bring the existing Tuxedo window to the
    /// front if tuxedo is alive, otherwise open a fresh window.
    func applicationShouldHandleReopen(_ app: NSApplication, hasVisibleWindows: Bool) -> Bool {
        if tuxedoRunning() {
            focusExistingWindow()
        } else {
            openWindow()
        }
        return false
    }

    /// Raise the specific terminal window running Tuxedo. openWindow names the
    /// session "Tuxedo", so we select that exact window/tab; `activate` alone
    /// would only surface iTerm's last-used window, which may be unrelated
    /// work. If the named session isn't found (e.g. the title was overwritten),
    /// activate still brought the terminal forward.
    func focusExistingWindow() {
        let script: String
        if FileManager.default.fileExists(atPath: "/Applications/iTerm.app") {
            script = """
            tell application "iTerm2"
                activate
                repeat with w in windows
                    repeat with t in tabs of w
                        repeat with s in sessions of t
                            if name of s contains "Tuxedo" then
                                select w
                                select t
                                return
                            end if
                        end repeat
                    end repeat
                end repeat
            end tell
            """
        } else {
            script = "tell application \"Terminal\" to activate"
        }
        DispatchQueue.global().async {
            var error: NSDictionary?
            NSAppleScript(source: script)?.executeAndReturnError(&error)
        }
    }

    func openWindow() {
        let script: String
        if FileManager.default.fileExists(atPath: "/Applications/iTerm.app") {
            installDynamicProfile()
            script = """
            tell application "iTerm2"
                activate
                set opened to false
                repeat with i from 1 to 20
                    try
                        set tuxWin to (create window with profile "Tuxedo")
                        tell current session of tuxWin to set name to "Tuxedo"
                        set opened to true
                        exit repeat
                    on error
                        delay 0.25
                    end try
                end repeat
                if not opened then
                    set w to (create window with default profile)
                    tell current session of w
                        set name to "Tuxedo"
                        write text "cd \\"$HOME\\"; clear; exec '\(bin)'"
                    end tell
                end if
            end tell
            """
        } else {
            script = """
            tell application "Terminal"
                activate
                set t to do script "cd \\"$HOME\\"; clear; exec '\(bin)'"
                if exists settings set "Tuxedo" then
                    set current settings of t to settings set "Tuxedo"
                end if
            end tell
            """
        }
        DispatchQueue.global().async { [weak self] in
            var error: NSDictionary?
            NSAppleScript(source: script)?.executeAndReturnError(&error)
            if let error, let code = error[NSAppleScript.errorNumber] as? Int, code == -1743 {
                // Automation permission denied: tell the user how to fix it
                // instead of dying silently.
                DispatchQueue.main.async { self?.permissionAlert() }
            }
        }
    }

    func permissionAlert() {
        let a = NSAlert()
        a.messageText = "Tuxedo precisa de permissão"
        a.informativeText = """
        Autorize o Tuxedo a controlar o iTerm2 em:
        Ajustes do Sistema → Privacidade e Segurança → Automação → Tuxedo
        Depois, abra o Tuxedo de novo.
        """
        a.addButton(withTitle: "Abrir Ajustes")
        a.addButton(withTitle: "OK")
        NSApp.activate(ignoringOtherApps: true)
        if a.runModal() == .alertFirstButtonReturn,
            let url = URL(string: "x-apple.systempreferences:com.apple.preference.security?Privacy_Automation")
        {
            NSWorkspace.shared.open(url)
        }
        NSApp.terminate(nil)
    }

    /// Refresh the iTerm2 dynamic profile so its Command always points at
    /// this bundle's binary (stale absolute paths were a recurring trap).
    func installDynamicProfile() {
        let dir = FileManager.default.homeDirectoryForCurrentUser
            .appendingPathComponent("Library/Application Support/iTerm2/DynamicProfiles")
        try? FileManager.default.createDirectory(at: dir, withIntermediateDirectories: true)
        let json = """
        {
          "Profiles": [
            {
              "Name": "Tuxedo",
              "Guid": "tuxedo-phosphor-green",
              "Normal Font": "IBMPlexMono-Regular 15",
              "Use Non-ASCII Font": false,
              "Custom Command": "Yes",
              "Command": "/bin/zsh -lc 'cd \\"$HOME\\"; exec \\"\(bin)\\"'",
              "Background Color": { "Red Component": 0.008, "Green Component": 0.04, "Blue Component": 0.008 },
              "Foreground Color": { "Red Component": 0.2, "Green Component": 1.0, "Blue Component": 0.2 },
              "Bold Color": { "Red Component": 0.4, "Green Component": 1.0, "Blue Component": 0.4 },
              "Cursor Color": { "Red Component": 0.2, "Green Component": 1.0, "Blue Component": 0.2 },
              "Cursor Text Color": { "Red Component": 0.008, "Green Component": 0.04, "Blue Component": 0.008 },
              "Silence Bell": true
            }
          ]
        }
        """
        try? Data(json.utf8).write(to: dir.appendingPathComponent("tuxedo.json"))
    }

    func tuxedoRunning() -> Bool {
        let p = Process()
        p.executableURL = URL(fileURLWithPath: "/usr/bin/pgrep")
        // Anchor to end-of-line ($) so we match ONLY the bare TUI binary and
        // not siblings whose command line merely *starts* with this path —
        // e.g. a stray `.../Resources/tuxedo-capture.sh`. Without the anchor,
        // pgrep's substring match false-positives on such a process, which
        // makes the launcher think the TUI is alive forever: it never quits
        // (stale Dock app) and Dock-icon reopens just focus the terminal
        // instead of opening a window.
        p.arguments = ["-f", bin + "$"]
        p.standardOutput = FileHandle.nullDevice
        p.standardError = FileHandle.nullDevice
        guard (try? p.run()) != nil else { return false }
        p.waitUntilExit()
        return p.terminationStatus == 0
    }

    /// Quit the launcher (clearing the Dock running-dot) a little after the
    /// TUI exits. The startup grace period covers the window spawn.
    func watchForExit() {
        DispatchQueue.global().async { [weak self] in
            Thread.sleep(forTimeInterval: 45)
            while let self, self.tuxedoRunning() {
                Thread.sleep(forTimeInterval: 5)
            }
            DispatchQueue.main.async { NSApp.terminate(nil) }
        }
    }
}

let app = NSApplication.shared
let delegate = Launcher()
app.delegate = delegate
app.setActivationPolicy(.regular)
app.run()
