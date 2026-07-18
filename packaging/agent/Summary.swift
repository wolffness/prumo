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

/// Icon urgency, worst-first: overdue → today → only upcoming → nothing dated.
enum IconState { case clear, upcoming, today, overdue }

struct Summary {
    let overdue: [TodoTask]
    let today: [TodoTask]
    let upcoming: [TodoTask]   // due > today, sorted by due ascending
    /// Every pending task that has a due date — what the icon counts.
    var totalDated: Int { overdue.count + today.count + upcoming.count }
    var iconState: IconState {
        if !overdue.isEmpty { return .overdue }
        if !today.isEmpty { return .today }
        if !upcoming.isEmpty { return .upcoming }
        return .clear
    }
}

/// Pure: partition pending dated tasks into overdue (due < today), due-today,
/// and upcoming (due > today). No-date tasks are ignored (the bar is about
/// time). Upcoming is sorted soonest-first.
func computeSummary(_ tasks: [TodoTask], today: String) -> Summary {
    let pending = tasks.filter { !$0.done }
    let overdue = pending.filter { t in t.due.map { $0 < today } ?? false }
    let dueToday = pending.filter { $0.due == today }
    let upcoming = pending.filter { t in t.due.map { $0 > today } ?? false }
        .sorted { ($0.due ?? "") < ($1.due ?? "") }
    return Summary(overdue: overdue, today: dueToday, upcoming: upcoming)
}

/// Today as "YYYY-MM-DD" in the local time zone.
func todayString() -> String {
    let f = DateFormatter()
    f.dateFormat = "yyyy-MM-dd"
    f.timeZone = TimeZone.current
    return f.string(from: Date())
}

/// Side-effecting: run `prumo ls --json` against `todoFile` and decode the
/// task list. The agent is launched by a LaunchAgent with no shell env, so we
/// MUST pass TODO_FILE explicitly — otherwise prumo reads its default file
/// (wrong list). Returns [] on any failure (missing binary, bad JSON) so the UI
/// degrades to an empty/neutral icon rather than crashing.
func fetchTasks(todoFile: URL) -> [TodoTask] {
    let p = Process()
    p.executableURL = resolvePrumoBinary()
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
