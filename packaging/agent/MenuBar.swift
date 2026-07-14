// Menu bar surface: an NSStatusItem showing the overdue+today count (amber when
// overdue) and a dropdown grouped into ATRASADAS / HOJE with check-to-complete.
import AppKit

final class MenuBarController: NSObject, NSMenuDelegate {
    private let statusItem = NSStatusBar.system.statusItem(withLength: NSStatusItem.variableLength)
    private let onNewTask: () -> Void
    private var current = Summary(overdue: [], today: [])
    private let maxPerGroup = 5
    private var watchSource: DispatchSourceFileSystemObject?
    private var watchedFD: Int32 = -1
    private var midnightTimer: Timer?

    init(onNewTask: @escaping () -> Void) {
        self.onNewTask = onNewTask
        super.init()
    }

    func start() {
        let menu = NSMenu()
        menu.delegate = self
        statusItem.menu = menu
        refresh()
        startWatching()
        scheduleMidnight()
    }

    /// Watch TODO_FILE for external writes (TUI saves, CLI edits, inbox drains
    /// land here after merge). Editors that replace the file break the fd, so we
    /// re-arm on cancel.
    private func startWatching() {
        let path = resolveTodoFile().path
        let fd = open(path, O_EVTONLY)
        guard fd >= 0 else { return }
        watchedFD = fd
        let src = DispatchSource.makeFileSystemObjectSource(
            fileDescriptor: fd, eventMask: [.write, .delete, .rename, .extend],
            queue: DispatchQueue.global())
        src.setEventHandler { [weak self] in
            self?.refresh()
            if let s = self, s.watchSource?.data.contains(.delete) == true
                || s.watchSource?.data.contains(.rename) == true {
                s.watchSource?.cancel()
            }
        }
        src.setCancelHandler { [weak self] in
            if let self, self.watchedFD >= 0 { close(self.watchedFD); self.watchedFD = -1 }
            // Atomic-save replaced the file: re-arm shortly.
            DispatchQueue.global().asyncAfter(deadline: .now() + 0.3) { [weak self] in
                self?.startWatching()
            }
        }
        watchSource = src
        src.resume()
    }

    /// Recompute overdue/today when the date rolls over.
    private func scheduleMidnight() {
        midnightTimer?.invalidate()
        let cal = Calendar.current
        guard let next = cal.nextDate(after: Date(),
              matching: DateComponents(hour: 0, minute: 0, second: 5),
              matchingPolicy: .nextTime) else { return }
        let t = Timer(fire: next, interval: 0, repeats: false) { [weak self] _ in
            self?.refresh()
            self?.scheduleMidnight()
        }
        RunLoop.main.add(t, forMode: .common)
        midnightTimer = t
    }

    /// Re-fetch tasks and repaint the icon. Safe to call from any thread; hops
    /// to main for UI.
    func refresh() {
        let tasks = fetchTasks()
        let summary = computeSummary(tasks, today: todayString())
        DispatchQueue.main.async { [weak self] in
            self?.current = summary
            self?.renderIcon()
        }
    }

    private func renderIcon() {
        guard let button = statusItem.button else { return }
        let mono = NSFont.monospacedDigitSystemFont(ofSize: 13, weight: .semibold)
        switch current.iconState {
        case .empty:
            button.attributedTitle = NSAttributedString(
                string: "☰",
                attributes: [.foregroundColor: Theme.phosphorDim, .font: mono])
        case .normal:
            button.attributedTitle = NSAttributedString(
                string: "☰ \(current.actionable)",
                attributes: [.foregroundColor: Theme.phosphor, .font: mono])
        case .alert:
            button.attributedTitle = NSAttributedString(
                string: "☰ \(current.actionable)",
                attributes: [.foregroundColor: Theme.amber, .font: mono])
        }
    }

    // NSMenuDelegate: rebuild the menu right before it opens, from fresh data.
    func menuNeedsUpdate(_ menu: NSMenu) {
        let tasks = fetchTasks()
        current = computeSummary(tasks, today: todayString())
        renderIcon()
        rebuildMenu(menu)
    }

    private func rebuildMenu(_ menu: NSMenu) {
        menu.removeAllItems()
        addGroup(menu, title: "ATRASADAS", tasks: current.overdue, overdue: true)
        addGroup(menu, title: "HOJE", tasks: current.today, overdue: false)
        if current.actionable == 0 {
            let none = NSMenuItem(title: "Nada para hoje 🎉", action: nil, keyEquivalent: "")
            none.isEnabled = false
            menu.addItem(none)
        }
        menu.addItem(.separator())
        let open = NSMenuItem(title: "Abrir Tuxedo", action: #selector(openTuxedo), keyEquivalent: "")
        open.target = self
        menu.addItem(open)
        let new = NSMenuItem(title: "Nova tarefa…", action: #selector(newTask), keyEquivalent: "")
        new.target = self
        menu.addItem(new)
        menu.addItem(.separator())
        menu.addItem(NSMenuItem(title: "Sair", action: #selector(NSApplication.terminate(_:)), keyEquivalent: "q"))
    }

    private func addGroup(_ menu: NSMenu, title: String, tasks: [TodoTask], overdue: Bool) {
        guard !tasks.isEmpty else { return }
        let header = NSMenuItem(title: title, action: nil, keyEquivalent: "")
        header.isEnabled = false
        menu.addItem(header)
        for task in tasks.prefix(maxPerGroup) {
            menu.addItem(taskItem(task, overdue: overdue))
        }
        if tasks.count > maxPerGroup {
            let more = NSMenuItem(title: "  … +\(tasks.count - maxPerGroup) mais", action: nil, keyEquivalent: "")
            more.isEnabled = false
            menu.addItem(more)
        }
    }

    /// A single task row: "☐ <text>   −Nd" for overdue, "☐ <text>" for today.
    private func taskItem(_ task: TodoTask, overdue: Bool) -> NSMenuItem {
        var label = "☐ " + displayText(task)
        if overdue, let d = task.due, let days = daysAgo(d) {
            label += "   −\(days)d"
        }
        let item = NSMenuItem(title: label, action: #selector(completeTask(_:)), keyEquivalent: "")
        item.target = self
        item.representedObject = task.raw   // matched on click to re-locate the task
        return item
    }

    /// Clean label for a menu row: drop the leading "(A) " priority and the
    /// leading creation date, and strip the metadata key:value tokens tuxedo
    /// adds (due/t/rec/note/at). Keeps +projects and @contexts, which are short
    /// and meaningful. The key list is explicit so plain URLs (http:, https:)
    /// in the task text survive.
    private func displayText(_ task: TodoTask) -> String {
        var s = task.raw
        if let p = task.priority { s = s.replacingOccurrences(of: "(\(p)) ", with: "") }
        s = s.replacingOccurrences(
            of: #"^\d{4}-\d{2}-\d{2}\s+"#, with: "",
            options: .regularExpression)
        s = s.replacingOccurrences(
            of: #"\s*\b(?:due|t|rec|note|at):\S+"#, with: "",
            options: .regularExpression)
        return s.trimmingCharacters(in: .whitespaces)
    }

    private func daysAgo(_ due: String) -> Int? {
        let f = DateFormatter(); f.dateFormat = "yyyy-MM-dd"; f.timeZone = .current
        guard let d = f.date(from: due), let t = f.date(from: todayString()) else { return nil }
        return Calendar.current.dateComponents([.day], from: d, to: t).day
    }

    /// Complete a task. Anti-race: re-fetch and match by raw text (positions
    /// shift when the file changes), then `tuxedo done <current n>`.
    @objc private func completeTask(_ sender: NSMenuItem) {
        guard let raw = sender.representedObject as? String else { return }
        DispatchQueue.global().async { [weak self] in
            let fresh = fetchTasks()
            guard let match = fresh.first(where: { $0.raw == raw && !$0.done }) else {
                self?.refresh(); return
            }
            let p = Process()
            p.executableURL = resolveTuxedoBinary()
            p.arguments = ["done", String(match.n)]
            p.standardOutput = FileHandle.nullDevice
            p.standardError = FileHandle.nullDevice
            try? p.run()
            p.waitUntilExit()
            self?.refresh()
        }
    }

    @objc private func openTuxedo() {
        NSWorkspace.shared.openApplication(
            at: URL(fileURLWithPath: "/Applications/Tuxedo.app"),
            configuration: NSWorkspace.OpenConfiguration())
    }

    @objc private func newTask() { onNewTask() }
}
