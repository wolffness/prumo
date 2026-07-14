import Foundation

func check(_ cond: Bool, _ label: String) {
    if !cond { print("FAIL: \(label)"); exit(1) }
}

// today = 2026-07-14. Task 1 overdue, 2 today, 3 future, 4 no-date, 5 done.
let tasks = [
    TodoTask(n: 1, raw: "(A) Ligar cliente due:2026-07-10", done: false, priority: "A", due: "2026-07-10"),
    TodoTask(n: 2, raw: "(B) Revisar due:2026-07-14",       done: false, priority: "B", due: "2026-07-14"),
    TodoTask(n: 3, raw: "Estudar due:2026-07-20",           done: false, priority: nil, due: "2026-07-20"),
    TodoTask(n: 4, raw: "Sem data",                          done: false, priority: nil, due: nil),
    TodoTask(n: 5, raw: "x concluida due:2026-07-10",       done: true,  priority: nil, due: "2026-07-10"),
]
let s = computeSummary(tasks, today: "2026-07-14")
check(s.overdue.count == 1, "overdue count == 1")
check(s.overdue.first?.n == 1, "overdue is task 1")
check(s.today.count == 1, "today count == 1")
check(s.today.first?.n == 2, "today is task 2")
check(s.actionable == 2, "actionable == 2")
check(s.iconState == .alert, "iconState alert when overdue present")

let onlyToday = computeSummary(
    [TodoTask(n: 2, raw: "b due:2026-07-14", done: false, priority: nil, due: "2026-07-14")],
    today: "2026-07-14")
check(onlyToday.iconState == .normal, "iconState normal when only today")

let empty = computeSummary([], today: "2026-07-14")
check(empty.iconState == .empty, "iconState empty when nothing")
check(empty.actionable == 0, "actionable 0 when empty")

print("ALL SUMMARY TESTS PASSED")
