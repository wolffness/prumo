import Foundation

func check(_ cond: Bool, _ label: String) {
    if !cond { print("FAIL: \(label)"); exit(1) }
}

// today = 2026-07-14. 1 overdue, 2 today, 3 & 4 upcoming, 5 no-date, 6 done.
let tasks = [
    TodoTask(n: 1, raw: "(A) Ligar cliente due:2026-07-10", done: false, priority: "A", due: "2026-07-10"),
    TodoTask(n: 2, raw: "(B) Revisar due:2026-07-14",       done: false, priority: "B", due: "2026-07-14"),
    TodoTask(n: 3, raw: "Estudar due:2026-07-20",           done: false, priority: nil, due: "2026-07-20"),
    TodoTask(n: 4, raw: "Comprar due:2026-07-16",           done: false, priority: nil, due: "2026-07-16"),
    TodoTask(n: 5, raw: "Sem data",                          done: false, priority: nil, due: nil),
    TodoTask(n: 6, raw: "x concluida due:2026-07-10",       done: true,  priority: nil, due: "2026-07-10"),
]
let s = computeSummary(tasks, today: "2026-07-14")
check(s.overdue.count == 1, "overdue count == 1")
check(s.overdue.first?.n == 1, "overdue is task 1")
check(s.today.count == 1, "today count == 1")
check(s.today.first?.n == 2, "today is task 2")
check(s.upcoming.count == 2, "upcoming count == 2")
check(s.upcoming.map { $0.n } == [4, 3], "upcoming sorted soonest-first (16th before 20th)")
check(s.totalDated == 4, "totalDated == 4 (no-date and done excluded)")
check(s.iconState == .overdue, "iconState overdue when overdue present")

// Only today (no overdue) → .today
let onlyToday = computeSummary(
    [TodoTask(n: 2, raw: "b due:2026-07-14", done: false, priority: nil, due: "2026-07-14")],
    today: "2026-07-14")
check(onlyToday.iconState == .today, "iconState today when only today")

// Only upcoming → .upcoming (the case the user hit)
let onlyUpcoming = computeSummary(
    [TodoTask(n: 1, raw: "a due:2026-07-15", done: false, priority: nil, due: "2026-07-15"),
     TodoTask(n: 2, raw: "b due:2026-07-20", done: false, priority: nil, due: "2026-07-20")],
    today: "2026-07-14")
check(onlyUpcoming.iconState == .upcoming, "iconState upcoming when only future")
check(onlyUpcoming.totalDated == 2, "totalDated counts upcoming")

// Nothing dated → .clear
let clear = computeSummary(
    [TodoTask(n: 1, raw: "sem data", done: false, priority: nil, due: nil)],
    today: "2026-07-14")
check(clear.iconState == .clear, "iconState clear when nothing dated")
check(clear.totalDated == 0, "totalDated 0 when nothing dated")

let taggedTasks = """
Comprar material +Admin @Rua
x tarefa pronta +academia @Casa
Ópera revisão +Árvore @Ação
Admin duplicado +Admin @Rua
"""
let projects = CaptureTagAutocomplete.values(in: taggedTasks, kind: .project)
let contexts = CaptureTagAutocomplete.values(in: taggedTasks, kind: .context)
check(projects == ["academia", "Admin", "Árvore"], "projects extracted, sorted and deduplicated")
check(contexts == ["Ação", "Casa", "Rua"], "contexts extracted independently")

let projectMatches = CaptureTagAutocomplete.matches(prefix: "A", in: projects)
check(projectMatches == ["academia", "Admin"], "project matching is case-insensitive")
let rankedMatches = CaptureTagAutocomplete.matches(prefix: "dmi", in: ["MadMen", "Admin", "Demo"])
check(rankedMatches == ["Admin"], "contains fallback matches after prefix search")
check(CaptureTagAutocomplete.matches(prefix: "xyz", in: projects).isEmpty, "unknown project remains creatable")
let manyProjects = (0..<12).map { "A\($0)" }
check(CaptureTagAutocomplete.matches(prefix: "a", in: manyProjects).count == 8, "matches cap at eight")
check(
    CaptureTagAutocomplete.steppedSelection(current: 0, matchCount: 3, forward: false) == 2,
    "selection wraps backward"
)
check(
    CaptureTagAutocomplete.steppedSelection(current: 2, matchCount: 3, forward: true) == 0,
    "selection wraps forward"
)

let draft = "Revisar amanhã +ad @r"
let projectCursor = "Revisar amanhã +ad".utf16.count
if let target = CaptureTagAutocomplete.target(in: draft, cursorUTF16: projectCursor) {
    check(target.kind == .project, "project target detected after multibyte text")
    check(target.prefix == "ad", "project prefix detected")
    let replaced = CaptureTagAutocomplete.replacing(target: target, with: "Admin", in: draft)
    check(replaced.text == "Revisar amanhã +Admin @r", "replacement preserves NLP date text")
    check(replaced.cursorUTF16 == "Revisar amanhã +Admin".utf16.count, "cursor follows accepted project")
} else {
    check(false, "project target should exist")
}

let contextCursor = draft.utf16.count
if let target = CaptureTagAutocomplete.target(in: draft, cursorUTF16: contextCursor) {
    check(target.kind == .context, "context target detected")
    check(target.prefix == "r", "context prefix detected")
} else {
    check(false, "context target should exist")
}

let temporaryInbox = FileManager.default.temporaryDirectory
    .appendingPathComponent("tuxedo-agent-tests-\(UUID().uuidString).txt")
let unmatchedTask = "Revisar amanhã +ProjetoNovo @ContextoNovo"
check(
    CaptureTagAutocomplete.appendCapture(unmatchedTask, to: temporaryInbox),
    "unmatched tags persist as a new capture"
)
check(
    (try? String(contentsOf: temporaryInbox, encoding: .utf8)) == unmatchedTask + "\n",
    "capture persists the exact task for NLP processing"
)

print("ALL AGENT TESTS PASSED")
