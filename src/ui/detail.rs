use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;

use crate::app::App;
use crate::theme::Theme;
use crate::todo::Task;
use crate::ui::task_row::due_label;

pub fn render(frame: &mut Frame, area: Rect, app: &App) {
    let theme = app.theme();
    super::fill_bg(frame, area, Style::default().bg(theme.panel));

    let task = app.cur_task();
    // Wrap to the actual pane width minus 1-char left padding and 1-char
    // safety margin on the right. Floor at 16 so a tiny pane still wraps.
    let wrap_w = (area.width as usize).saturating_sub(2).max(16);
    let mut clicks: Vec<(usize, std::path::PathBuf)> = Vec::new();
    let lines = build_lines(theme, app, task, app.today(), wrap_w, &mut clicks);
    // Attachment rows double as mouse targets: convert their line indices
    // into absolute screen rects (the pane's Paragraph never scrolls, so
    // line index maps 1:1 to a row offset).
    for (idx, path) in clicks {
        let y = area.y + u16::try_from(idx).unwrap_or(u16::MAX);
        if y < area.y + area.height {
            let rect = Rect {
                x: area.x,
                y,
                width: area.width,
                height: 1,
            };
            app.register_click_target(rect, path);
        }
    }
    let para = Paragraph::new(lines).style(Style::default().bg(theme.panel).fg(theme.fg));
    frame.render_widget(para, area);
}

fn build_lines<'a>(
    theme: &Theme,
    app: &App,
    task: Option<&'a Task>,
    today: &'a str,
    wrap_w: usize,
    clicks: &mut Vec<(usize, std::path::PathBuf)>,
) -> Vec<Line<'a>> {
    let mut rows: Vec<Line> = Vec::new();
    rows.push(line_panel(
        theme,
        vec![Span::styled(
            " DETAIL",
            Style::default().fg(theme.dim).add_modifier(Modifier::BOLD),
        )],
    ));
    rows.push(line_panel(theme, vec![Span::raw(" ")]));
    let Some(t) = task else {
        rows.push(line_panel(
            theme,
            vec![Span::styled(" (no task)", Style::default().fg(theme.dim))],
        ));
        return rows;
    };

    let priority_value = if let Some(p) = t.priority {
        Span::styled(
            format!("({p})"),
            Style::default()
                .fg(theme.priority_color(p))
                .add_modifier(Modifier::BOLD),
        )
    } else {
        Span::raw("")
    };
    rows.push(line_panel(
        theme,
        vec![
            Span::styled(" priority  ", Style::default().fg(theme.dim)),
            priority_value,
        ],
    ));
    rows.push(line_panel(
        theme,
        vec![
            Span::styled(" created   ", Style::default().fg(theme.dim)),
            Span::styled(
                t.created_date.as_deref().unwrap_or("—"),
                Style::default().fg(theme.fg),
            ),
        ],
    ));
    if let Some(due) = &t.due {
        rows.push(line_panel(
            theme,
            vec![
                Span::styled(" due       ", Style::default().fg(theme.dim)),
                Span::styled(due.as_str(), Style::default().fg(theme.fg)),
                Span::raw("  "),
                Span::styled(due_label(due, today), Style::default().fg(theme.overdue)),
            ],
        ));
    }
    rows.push(line_panel(
        theme,
        vec![
            Span::styled(" projects  ", Style::default().fg(theme.dim)),
            Span::styled(
                t.projects
                    .iter()
                    .map(|p| format!("+{p}"))
                    .collect::<Vec<_>>()
                    .join(" "),
                Style::default().fg(theme.project),
            ),
        ],
    ));
    rows.push(line_panel(
        theme,
        vec![
            Span::styled(" contexts  ", Style::default().fg(theme.dim)),
            Span::styled(
                t.contexts
                    .iter()
                    .map(|c| format!("@{c}"))
                    .collect::<Vec<_>>()
                    .join(" "),
                Style::default().fg(theme.context),
            ),
        ],
    ));

    if t.done {
        rows.push(line_panel(
            theme,
            vec![
                Span::styled(" done      ", Style::default().fg(theme.dim)),
                Span::styled(
                    t.done_date.as_deref().unwrap_or(""),
                    Style::default().fg(theme.done),
                ),
            ],
        ));
    }
    push_note_lines(&mut rows, theme, app, t, wrap_w);
    push_attachment_lines(&mut rows, theme, app, t, clicks);
    rows
}

/// Cap on note lines shown in the pane; the panel (`m`) shows the rest.
const NOTE_PREVIEW_MAX_LINES: usize = 40;

/// FILES section: one row per `at:` attachment. Names are underlined and
/// registered both as `file://` OSC 8 link targets (Cmd-click) and as plain
/// mouse-click regions via `clicks` (line index → path).
fn push_attachment_lines<'a>(
    rows: &mut Vec<Line<'a>>,
    theme: &Theme,
    app: &App,
    t: &Task,
    clicks: &mut Vec<(usize, std::path::PathBuf)>,
) {
    let rels = crate::attach::attach_rels_from_raw(&t.raw);
    if rels.is_empty() {
        return;
    }
    rows.push(line_panel(theme, vec![Span::raw(" ")]));
    rows.push(line_panel(
        theme,
        vec![Span::styled(
            " FILES",
            Style::default().fg(theme.dim).add_modifier(Modifier::BOLD),
        )],
    ));
    let assets = crate::attach::assets_dir(&app.file_path);
    for rel in rels {
        let path = crate::attach::path_for_rel(&assets, &rel);
        if path.exists() {
            app.register_link_target(rel.clone(), crate::attach::file_uri(&path));
            clicks.push((rows.len(), path.clone()));
            rows.push(line_panel(
                theme,
                vec![
                    Span::raw(" "),
                    Span::styled(
                        rel,
                        Style::default()
                            .fg(theme.accent)
                            .add_modifier(Modifier::UNDERLINED),
                    ),
                ],
            ));
        } else {
            rows.push(line_panel(
                theme,
                vec![
                    Span::raw(" "),
                    Span::styled(format!("{rel} (missing)"), Style::default().fg(theme.dim)),
                ],
            ));
        }
    }
}

/// NOTE section: the linked note's full content, wrapped to the pane and
/// styled with the same line-level Markdown rules as the note panel.
fn push_note_lines<'a>(rows: &mut Vec<Line<'a>>, theme: &Theme, app: &App, t: &Task, wrap_w: usize) {
    let Some(rel) = crate::note::note_rel_from_raw(&t.raw) else {
        return;
    };
    let target = crate::note::target_for_task(t, app.notes_dir());
    rows.push(line_panel(theme, vec![Span::raw(" ")]));
    rows.push(line_panel(
        theme,
        vec![Span::styled(
            " NOTE",
            Style::default().fg(theme.dim).add_modifier(Modifier::BOLD),
        )],
    ));
    // Reading per frame is fine: frames render on input only, notes are
    // small, and a cache would need its own invalidation on panel saves
    // and external edits.
    let body = match std::fs::read_to_string(&target.path) {
        Ok(body) => body,
        Err(_) => {
            rows.push(line_panel(
                theme,
                vec![Span::styled(
                    format!(" {rel} (missing)"),
                    Style::default().fg(theme.dim),
                )],
            ));
            return;
        }
    };
    // Subtask progress bar (amber, CRT-style) right under the NOTE header.
    if let Some((done, total)) = crate::subtasks::progress(&body) {
        let bar_w = wrap_w.saturating_sub(10).clamp(6, 20);
        rows.push(line_panel(
            theme,
            vec![
                Span::raw(" "),
                Span::styled(
                    crate::subtasks::bar(done, total, bar_w),
                    Style::default().fg(crate::ui::task_row::AMBER),
                ),
                Span::styled(
                    format!(" {done}/{total}"),
                    Style::default().fg(crate::ui::task_row::AMBER),
                ),
            ],
        ));
    }
    let mut shown = 0usize;
    for raw_line in body.lines() {
        if shown >= NOTE_PREVIEW_MAX_LINES {
            rows.push(line_panel(
                theme,
                vec![Span::styled(
                    " … (m opens the full note)",
                    Style::default().fg(theme.dim),
                )],
            ));
            return;
        }
        if raw_line.trim().is_empty() {
            rows.push(line_panel(theme, vec![Span::raw(" ")]));
            shown += 1;
            continue;
        }
        // Wrap long lines to the pane; only the first chunk keeps the
        // Markdown line styling (continuations read as plain text).
        let chunks = wrap_words(raw_line, wrap_w.saturating_sub(1));
        for (i, chunk) in chunks.into_iter().enumerate() {
            if shown >= NOTE_PREVIEW_MAX_LINES {
                break;
            }
            let text = chunk.join(" ");
            let line = if i == 0 {
                // Preserve original indentation for the styled first chunk.
                let indent: String = raw_line.chars().take_while(|c| c.is_whitespace()).collect();
                super::note_panel::markdown_line(theme, &format!("{indent}{text}"))
            } else {
                Line::from(Span::styled(text, Style::default().fg(theme.fg)))
            };
            let mut spans = vec![Span::raw(" ")];
            spans.extend(line.spans);
            rows.push(line_panel(theme, spans));
            shown += 1;
        }
    }
}

fn line_panel<'a>(theme: &Theme, spans: Vec<Span<'a>>) -> Line<'a> {
    Line::from(spans).style(Style::default().bg(theme.panel))
}

/// Wrap `s` to roughly `width` graphemes, returning each output line as a
/// vector of borrowed words. Borrowing avoids the per-frame `String` alloc
/// that the previous `Vec<String>` form forced on every render.
fn wrap_words(s: &str, width: usize) -> Vec<Vec<&str>> {
    let mut out: Vec<Vec<&str>> = Vec::new();
    let mut acc: Vec<&str> = Vec::new();
    let mut acc_len = 0;
    for word in s.split_whitespace() {
        let wlen = word.chars().count();
        let extra = if acc.is_empty() { 0 } else { 1 };
        if acc_len + wlen + extra > width && !acc.is_empty() {
            out.push(std::mem::take(&mut acc));
            acc_len = 0;
        }
        if !acc.is_empty() {
            acc_len += 1;
        }
        acc.push(word);
        acc_len += wlen;
    }
    if !acc.is_empty() {
        out.push(acc);
    }
    out
}
