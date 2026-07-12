//! Subtasks are Markdown checkboxes (`- [ ]` / `- [x]`) inside a task's
//! note. The note panel toggles them (Space) and appends new ones (`n`);
//! the list and DETAIL pane surface progress computed from the note body.

/// `(done, total)` checkbox counts in a note body. `None` when the note has
/// no checkboxes at all, so callers can skip progress UI entirely.
pub fn progress(body: &str) -> Option<(usize, usize)> {
    let mut done = 0usize;
    let mut total = 0usize;
    for line in body.lines() {
        match checkbox_state(line) {
            Some(true) => {
                done += 1;
                total += 1;
            }
            Some(false) => total += 1,
            None => {}
        }
    }
    (total > 0).then_some((done, total))
}

/// Checkbox state of a single line: `Some(done)` for `- [ ]`/`- [x]` lines
/// (also `*`/`+` bullets, case-insensitive `x`), `None` otherwise.
pub fn checkbox_state(line: &str) -> Option<bool> {
    let t = line.trim_start();
    let rest = t
        .strip_prefix("- ")
        .or_else(|| t.strip_prefix("* "))
        .or_else(|| t.strip_prefix("+ "))?;
    let rest = rest.trim_start();
    if let Some(inner) = rest.strip_prefix('[') {
        let mut chars = inner.chars();
        let mark = chars.next()?;
        if chars.next()? != ']' {
            return None;
        }
        match mark {
            ' ' => Some(false),
            'x' | 'X' => Some(true),
            _ => None,
        }
    } else {
        None
    }
}

/// Toggle the checkbox on `line`, preserving indentation and text. Returns
/// `None` when the line isn't a checkbox.
pub fn toggle_line(line: &str) -> Option<String> {
    let done = checkbox_state(line)?;
    let (open, close) = if done { ("[x]", "[ ]") } else { ("[ ]", "[x]") };
    // Also flip the uppercase variant; replacen(1) keeps any later brackets
    // in the text untouched.
    let flipped = if done && line.contains("[X]") {
        line.replacen("[X]", close, 1)
    } else {
        line.replacen(open, close, 1)
    };
    Some(flipped)
}

/// Compact progress bar for the DETAIL pane, e.g. `▓▓▓░░░░░`.
pub fn bar(done: usize, total: usize, width: usize) -> String {
    if total == 0 || width == 0 {
        return String::new();
    }
    let filled = (done * width + total / 2) / total;
    let filled = filled.min(width);
    "▓".repeat(filled) + &"░".repeat(width - filled)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counts_checkboxes_ignoring_other_lines() {
        let body = "# Title\n\n- [ ] one\n- [x] two\n  * [X] indented\nplain\n- not a box\n";
        assert_eq!(progress(body), Some((2, 3)));
        assert_eq!(progress("no boxes here"), None);
    }

    #[test]
    fn toggles_both_ways_preserving_indent() {
        assert_eq!(toggle_line("- [ ] task"), Some("- [x] task".to_string()));
        assert_eq!(
            toggle_line("  - [x] done thing"),
            Some("  - [ ] done thing".to_string())
        );
        assert_eq!(toggle_line("* [X] caps"), Some("* [ ] caps".to_string()));
        assert_eq!(toggle_line("plain text"), None);
    }

    #[test]
    fn toggle_only_touches_the_checkbox_bracket() {
        assert_eq!(
            toggle_line("- [ ] read [x] the manual"),
            Some("- [x] read [x] the manual".to_string())
        );
    }

    #[test]
    fn bar_scales_and_rounds() {
        assert_eq!(bar(0, 4, 8), "░░░░░░░░");
        assert_eq!(bar(2, 4, 8), "▓▓▓▓░░░░");
        assert_eq!(bar(4, 4, 8), "▓▓▓▓▓▓▓▓");
        assert_eq!(bar(1, 3, 6), "▓▓░░░░");
    }
}
