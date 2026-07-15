// Task model + the pure "what's due" logic. Foundation only so the test binary
// links without frameworks. Dates are ISO "YYYY-MM-DD" strings, which sort
// lexicographically — no Date parsing needed for overdue/today comparisons.
import Foundation

struct TodoTask: Decodable {
    let n: Int
    let raw: String
    let done: Bool
    let priority: String?
    let due: String?
    // The JSON has more fields (projects, contexts, rec, t, created,
    // completed); Decodable ignores keys we don't declare.
}

enum IconState { case empty, normal, alert }

struct Summary {
    let overdue: [TodoTask]
    let today: [TodoTask]
    var actionable: Int { overdue.count + today.count }
    var iconState: IconState {
        if !overdue.isEmpty { return .alert }
        if !today.isEmpty { return .normal }
        return .empty
    }
}

/// Pure: partition pending tasks into overdue (due < today) and due-today.
func computeSummary(_ tasks: [TodoTask], today: String) -> Summary {
    let pending = tasks.filter { !$0.done }
    let overdue = pending.filter { t in t.due.map { $0 < today } ?? false }
    let dueToday = pending.filter { $0.due == today }
    return Summary(overdue: overdue, today: dueToday)
}

/// Today as "YYYY-MM-DD" in the local time zone.
func todayString() -> String {
    let f = DateFormatter()
    f.dateFormat = "yyyy-MM-dd"
    f.timeZone = TimeZone.current
    return f.string(from: Date())
}

/// Side-effecting: run `tuxedo ls --json` against `todoFile` and decode the
/// task list. The agent is launched by a LaunchAgent with no shell env, so we
/// MUST pass TODO_FILE explicitly — otherwise tuxedo reads its default file
/// (wrong list). Returns [] on any failure (missing binary, bad JSON) so the UI
/// degrades to an empty/neutral icon rather than crashing.
func fetchTasks(todoFile: URL) -> [TodoTask] {
    let p = Process()
    p.executableURL = resolveTuxedoBinary()
    p.arguments = ["ls", "--json"]
    var env = ProcessInfo.processInfo.environment
    env["TODO_FILE"] = todoFile.path
    p.environment = env
    let pipe = Pipe()
    p.standardOutput = pipe
    p.standardError = FileHandle.nullDevice
    guard (try? p.run()) != nil else { return [] }
    let data = pipe.fileHandleForReading.readDataToEndOfFile()
    p.waitUntilExit()
    return (try? JSONDecoder().decode([TodoTask].self, from: data)) ?? []
}
