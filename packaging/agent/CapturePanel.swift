import AppKit
import Carbon.HIToolbox

/// Borderless panels refuse key status unless told otherwise.
final class KeyPanel: NSPanel {
    override var canBecomeKey: Bool { true }
}

final class CapturePanel: NSObject, NSTextFieldDelegate {
    private let panelWidth: CGFloat = 560
    private let inputHeight: CGFloat = 64
    private let suggestionHeight: CGFloat = 26

    var panel: KeyPanel!
    var root: NSView!
    var field: NSTextField!
    var inbox: URL!
    var todoFile: URL!

    private var font: NSFont!
    private var projects: [String] = []
    private var contexts: [String] = []
    private var matches: [String] = []
    private var selectedMatch = 0
    private var autocompleteSuppressed = false

    func start() {
        todoFile = resolveTodoFile()
        inbox = resolveInbox()
        buildPanel()
        registerHotkey()
    }

    func buildPanel() {
        panel = KeyPanel(
            contentRect: NSRect(x: 0, y: 0, width: panelWidth, height: inputHeight),
            styleMask: [.borderless, .nonactivatingPanel],
            backing: .buffered,
            defer: false
        )
        panel.level = .floating
        panel.isOpaque = false
        panel.backgroundColor = .clear
        panel.hidesOnDeactivate = true
        panel.collectionBehavior = [.canJoinAllSpaces, .transient]

        root = NSView(frame: NSRect(x: 0, y: 0, width: panelWidth, height: inputHeight))
        root.wantsLayer = true
        root.layer?.backgroundColor = Theme.screenBg.cgColor
        root.layer?.cornerRadius = 10
        root.layer?.borderWidth = 1.5
        root.layer?.borderColor = Theme.phosphorDim.cgColor

        font = NSFont(name: "IBMPlexMono-Regular", size: 18)
            ?? NSFont.monospacedSystemFont(ofSize: 18, weight: .regular)

        let prompt = NSTextField(labelWithString: ">")
        prompt.font = font
        prompt.textColor = Theme.phosphor
        prompt.frame = NSRect(x: 18, y: 19, width: 20, height: 26)
        prompt.identifier = NSUserInterfaceItemIdentifier("capture-prompt")
        root.addSubview(prompt)

        field = NSTextField(frame: NSRect(x: 44, y: 19, width: panelWidth - 62, height: 26))
        field.font = font
        field.textColor = Theme.phosphor
        field.isBordered = false
        field.isBezeled = false
        field.drawsBackground = false
        field.focusRingType = .none
        field.placeholderAttributedString = NSAttributedString(
            string: "nova tarefa… (amanhã, toda sexta, todo dia 2)",
            attributes: [.foregroundColor: Theme.phosphorDim, .font: font as Any]
        )
        field.delegate = self
        root.addSubview(field)

        panel.contentView = root
    }

    func registerHotkey() {
        // ⌥] toggles quick capture, ⌥[ opens/raises the app. Carbon hotkeys
        // need no accessibility permission.
        var captureRef: EventHotKeyRef?
        RegisterEventHotKey(
            UInt32(kVK_ANSI_RightBracket),
            UInt32(optionKey),
            EventHotKeyID(signature: OSType(0x5458_4344), id: 1), // "TXCD"
            GetApplicationEventTarget(),
            0,
            &captureRef
        )
        var openRef: EventHotKeyRef?
        RegisterEventHotKey(
            UInt32(kVK_ANSI_LeftBracket),
            UInt32(optionKey),
            EventHotKeyID(signature: OSType(0x5458_4344), id: 2),
            GetApplicationEventTarget(),
            0,
            &openRef
        )
        var eventType = EventTypeSpec(
            eventClass: OSType(kEventClassKeyboard),
            eventKind: UInt32(kEventHotKeyPressed)
        )
        InstallEventHandler(
            GetApplicationEventTarget(),
            { _, event, userData in
                let me = Unmanaged<CapturePanel>.fromOpaque(userData!).takeUnretainedValue()
                var hotKeyID = EventHotKeyID()
                GetEventParameter(
                    event,
                    EventParamName(kEventParamDirectObject),
                    EventParamType(typeEventHotKeyID),
                    nil,
                    MemoryLayout<EventHotKeyID>.size,
                    nil,
                    &hotKeyID
                )
                switch hotKeyID.id {
                case 1: me.toggle()
                case 2: me.openApp()
                default: break
                }
                return noErr
            },
            1,
            &eventType,
            Unmanaged.passUnretained(self).toOpaque(),
            nil
        )
    }

    /// Launch Prumo.app, or raise its existing window — the launcher's
    /// reopen path selects the terminal window already running the TUI.
    func openApp() {
        NSWorkspace.shared.openApplication(
            at: URL(fileURLWithPath: "/Applications/Prumo.app"),
            configuration: NSWorkspace.OpenConfiguration()
        )
    }

    func toggle() {
        if panel.isVisible {
            hide()
        } else {
            show()
        }
    }

    func show() {
        field.stringValue = ""
        reloadTagCorpus()
        autocompleteSuppressed = false
        selectedMatch = 0
        matches = []
        renderSuggestions()

        // Top third of the screen with the mouse, centered — the OmniFocus
        // quick-entry spot.
        let screen = NSScreen.screens.first(where: {
            NSMouseInRect(NSEvent.mouseLocation, $0.frame, false)
        }) ?? NSScreen.main
        if let f = screen?.visibleFrame {
            let x = f.midX - panel.frame.width / 2
            let y = f.minY + f.height * 0.72
            panel.setFrameOrigin(NSPoint(x: x, y: y))
        }
        panel.makeKeyAndOrderFront(nil)
        NSApp.activate(ignoringOtherApps: true)
        field.becomeFirstResponder()
    }

    func hide() {
        panel.orderOut(nil)
        NSApp.hide(nil)
    }

    func control(
        _ control: NSControl,
        textView: NSTextView,
        doCommandBy sel: Selector
    ) -> Bool {
        if sel == #selector(NSResponder.moveDown(_:)), !matches.isEmpty {
            selectedMatch = CaptureTagAutocomplete.steppedSelection(
                current: selectedMatch,
                matchCount: matches.count,
                forward: true
            )
            renderSuggestions()
            return true
        }
        if sel == #selector(NSResponder.moveUp(_:)), !matches.isEmpty {
            selectedMatch = CaptureTagAutocomplete.steppedSelection(
                current: selectedMatch,
                matchCount: matches.count,
                forward: false
            )
            renderSuggestions()
            return true
        }
        if sel == #selector(NSResponder.insertTab(_:)), !matches.isEmpty {
            acceptSuggestion(in: textView)
            return true
        }
        if sel == #selector(NSResponder.insertNewline(_:)) {
            // Enter accepts the highlighted suggestion while the popup is
            // open (matching the TUI prompts); a second Enter saves.
            if !matches.isEmpty {
                acceptSuggestion(in: textView)
            } else {
                save()
            }
            return true
        }
        if sel == #selector(NSResponder.cancelOperation(_:)) {
            hide()
            return true
        }
        return false
    }

    func controlTextDidChange(_ obj: Notification) {
        autocompleteSuppressed = false
        selectedMatch = 0
        refreshSuggestions()
    }

    private func reloadTagCorpus() {
        let saved = (try? String(contentsOf: todoFile, encoding: .utf8)) ?? ""
        let pending = (try? String(contentsOf: inbox, encoding: .utf8)) ?? ""
        let contents = saved + "\n" + pending
        projects = CaptureTagAutocomplete.values(in: contents, kind: .project)
        contexts = CaptureTagAutocomplete.values(in: contents, kind: .context)
    }

    private func refreshSuggestions() {
        guard !autocompleteSuppressed,
              let editor = field.currentEditor(),
              let target = CaptureTagAutocomplete.target(
                  in: field.stringValue,
                  cursorUTF16: editor.selectedRange.location
              ) else {
            matches = []
            renderSuggestions()
            return
        }

        let corpus = target.kind == .project ? projects : contexts
        matches = CaptureTagAutocomplete.matches(prefix: target.prefix, in: corpus)
        selectedMatch = min(selectedMatch, max(0, matches.count - 1))
        renderSuggestions()
    }

    private func acceptSuggestion(in textView: NSTextView) {
        guard let chosen = matches[safe: selectedMatch],
              let target = CaptureTagAutocomplete.target(
                  in: field.stringValue,
                  cursorUTF16: textView.selectedRange.location
              ) else { return }

        let replacement = CaptureTagAutocomplete.replacing(
            target: target,
            with: chosen,
            in: field.stringValue
        )
        field.stringValue = replacement.text
        textView.string = replacement.text
        textView.setSelectedRange(NSRange(location: replacement.cursorUTF16, length: 0))
        autocompleteSuppressed = true
        matches = []
        renderSuggestions()
    }

    private func renderSuggestions() {
        root.subviews
            .filter { $0.identifier?.rawValue.hasPrefix("capture-suggestion-") == true }
            .forEach { $0.removeFromSuperview() }

        let oldHeight = panel.frame.height
        let newHeight = inputHeight + CGFloat(matches.count) * suggestionHeight
        let top = panel.frame.maxY
        panel.setFrame(
            NSRect(x: panel.frame.minX, y: top - newHeight, width: panelWidth, height: newHeight),
            display: true
        )
        root.frame = NSRect(x: 0, y: 0, width: panelWidth, height: newHeight)
        let inputOffset = newHeight - inputHeight
        field.frame.origin.y = inputOffset + 19
        root.subviews
            .first { $0.identifier?.rawValue == "capture-prompt" }?
            .frame.origin.y = inputOffset + 19

        for (index, match) in matches.enumerated() {
            let row = NSView(frame: NSRect(
                x: 10,
                y: newHeight - inputHeight - CGFloat(index + 1) * suggestionHeight,
                width: panelWidth - 20,
                height: suggestionHeight
            ))
            row.identifier = NSUserInterfaceItemIdentifier("capture-suggestion-\(index)")
            row.wantsLayer = true
            if index == selectedMatch {
                row.layer?.backgroundColor = Theme.phosphor.withAlphaComponent(0.14).cgColor
                row.layer?.cornerRadius = 4
            }

            let label = NSTextField(labelWithString: suggestionLabel(match))
            label.frame = NSRect(x: 34, y: 2, width: panelWidth - 70, height: 22)
            label.font = NSFont.monospacedSystemFont(ofSize: 14, weight: .regular)
            label.textColor = index == selectedMatch ? Theme.phosphor : Theme.phosphorDim
            row.addSubview(label)
            root.addSubview(row)
        }

        if oldHeight != newHeight {
            panel.contentView?.needsLayout = true
        }
    }

    private func suggestionLabel(_ value: String) -> String {
        guard let editor = field.currentEditor(),
              let target = CaptureTagAutocomplete.target(
                  in: field.stringValue,
                  cursorUTF16: editor.selectedRange.location
              ) else { return value }
        return String(target.kind.sigil) + value
    }

    func save() {
        let text = field.stringValue.trimmingCharacters(in: .whitespaces)
        if !text.isEmpty {
            CaptureTagAutocomplete.appendCapture(text, to: inbox)
        }
        hide()
    }
}

private extension Collection {
    subscript(safe index: Index) -> Element? {
        indices.contains(index) ? self[index] : nil
    }
}
