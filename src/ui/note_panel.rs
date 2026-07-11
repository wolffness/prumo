use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};

use crate::app::{App, NotePanel};
use crate::theme::Theme;

/// Render the in-TUI note panel: title bar with the task body, the note's
/// lines with lightweight per-line Markdown styling, a cursor, and a footer
/// hint row that follows the panel's view/insert mode.
pub fn render(frame: &mut Frame, area: Rect, app: &App) {
    let theme = app.theme();
    let Some(panel) = app.note_panel.as_ref() else {
        return;
    };

    let mode_label = if panel.insert { " editing " } else { " note " };
    let dirty_mark = if panel.dirty { "* " } else { "" };
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.border).bg(theme.panel))
        .title(Line::from(vec![
            Span::raw(" "),
            Span::styled(
                format!("{dirty_mark}{}", panel.title),
                Style::default()
                    .fg(theme.accent)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(format!(" ·{mode_label}"), Style::default().fg(theme.dim)),
        ]))
        .style(Style::default().bg(theme.panel));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let [body_area, footer_area] =
        Layout::vertical([Constraint::Min(1), Constraint::Length(1)]).areas(inner);

    let lines: Vec<Line> = panel
        .lines
        .iter()
        .enumerate()
        .map(|(i, raw)| style_line(theme, panel, i, raw))
        .collect();

    let offset = crate::ui::keep_cursor_visible(
        panel.scroll.get(),
        Some(panel.row),
        body_area.height,
        lines.len(),
    );
    panel.scroll.set(offset);

    frame.render_widget(
        Paragraph::new(lines)
            .style(Style::default().bg(theme.panel).fg(theme.fg))
            .scroll((offset, 0)),
        body_area,
    );

    let hint = if panel.insert {
        "Esc view · Enter newline · Ctrl-S save"
    } else {
        "i edit · o editor · Ctrl-S save · Esc/q close (saves)"
    };
    frame.render_widget(
        Paragraph::new(Line::from(Span::styled(
            format!(" {hint}"),
            Style::default().fg(theme.dim),
        )))
        .style(Style::default().bg(theme.panel)),
        footer_area,
    );
}

/// Per-line Markdown styling. Line-level only (headings, bullets, quotes,
/// code fences) so the text on screen stays byte-identical to the buffer and
/// the cursor column maps 1:1 onto characters.
fn style_line<'a>(theme: &Theme, panel: &NotePanel, idx: usize, raw: &'a str) -> Line<'a> {
    let trimmed = raw.trim_start();
    let base = if trimmed.starts_with('#') {
        Style::default()
            .fg(theme.accent)
            .add_modifier(Modifier::BOLD)
    } else if trimmed.starts_with("> ") || trimmed == ">" {
        Style::default().fg(theme.dim).add_modifier(Modifier::ITALIC)
    } else if trimmed.starts_with("```") {
        Style::default().fg(theme.dim)
    } else {
        Style::default().fg(theme.fg)
    };

    let mut spans: Vec<Span> = Vec::new();
    let is_bullet =
        trimmed.starts_with("- ") || trimmed.starts_with("* ") || trimmed.starts_with("+ ");
    if is_bullet {
        // Color the bullet marker so lists scan visually.
        let indent_len = raw.len() - trimmed.len();
        let (indent, rest) = raw.split_at(indent_len);
        let (marker, tail) = rest.split_at(1);
        spans.push(Span::raw(indent));
        spans.push(Span::styled(marker, Style::default().fg(theme.project)));
        spans.push(Span::styled(tail, base));
    } else {
        spans.push(Span::styled(raw, base));
    }

    let mut line = Line::from(spans);
    if idx == panel.row {
        line = apply_cursor(theme, panel, line, raw);
    }
    line
}

/// Overlay the cursor cell on the current row. In insert mode the cursor may
/// sit one past the end of the line (append position), rendered as a
/// highlighted space.
fn apply_cursor<'a>(theme: &Theme, panel: &NotePanel, line: Line<'a>, raw: &'a str) -> Line<'a> {
    let col = panel.col.min(raw.chars().count());
    let cursor_style = Style::default()
        .fg(theme.bg)
        .bg(theme.cursor)
        .add_modifier(Modifier::BOLD);

    // Rebuild the row as three spans: before-cursor, cursor cell, after.
    let mut chars = raw.char_indices();
    let start = chars.nth(col).map_or(raw.len(), |(i, _)| i);
    let end = raw[start..]
        .char_indices()
        .nth(1)
        .map_or(raw.len(), |(i, _)| start + i);

    // Preserve the line-level style by reusing the first span's style for
    // the surrounding text (bullet-marker coloring is lost on the cursor
    // row — an acceptable trade for a simple, correct cursor).
    let base = line.spans.first().map_or(Style::default(), |s| s.style);
    let before = &raw[..start];
    let cursor_txt = if start == raw.len() {
        " "
    } else {
        &raw[start..end]
    };
    let after = if end <= raw.len() { &raw[end..] } else { "" };

    Line::from(vec![
        Span::styled(before, base),
        Span::styled(cursor_txt, cursor_style),
        Span::styled(after, base),
    ])
}
