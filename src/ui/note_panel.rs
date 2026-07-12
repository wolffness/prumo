use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};

use crate::app::App;
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

    // Hard-wrap each buffer line into width-sized display rows. Char-exact
    // chunking (not word wrap) keeps the cursor mapping trivial: display
    // row = col / w, x = col % w — no reflow bookkeeping.
    let wrap_w = usize::from(body_area.width).max(8);
    // Publish the wrap width so vertical cursor motion (handle_note) can
    // step through the same visual rows the renderer draws.
    panel.wrap_w.set(wrap_w);
    let mut lines: Vec<Line> = Vec::new();
    let mut cursor_display_row = 0usize;
    for (i, raw) in panel.lines.iter().enumerate() {
        let chunks = chunk_chars(raw, wrap_w);
        if i == panel.row {
            let idx = (panel.col / wrap_w).min(chunks.len() - 1);
            cursor_display_row = lines.len() + idx;
        }
        for (ci, chunk) in chunks.iter().enumerate() {
            // Markdown line styling keys off the line head; continuations
            // render as plain text.
            let mut line = if ci == 0 {
                markdown_line(theme, chunk)
            } else {
                Line::from(Span::styled(
                    chunk.clone(),
                    Style::default().fg(theme.fg),
                ))
            };
            if i == panel.row && ci == (panel.col / wrap_w).min(chunks.len() - 1) {
                let x = panel.col - ((panel.col / wrap_w).min(chunks.len() - 1)) * wrap_w;
                line = apply_cursor_at(theme, line, chunk, x);
            }
            lines.push(line);
        }
    }

    let offset = crate::ui::keep_cursor_visible(
        panel.scroll.get(),
        Some(cursor_display_row),
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

/// Per-line Markdown styling shared by the note panel and the DETAIL pane.
/// Line-level only (headings, bullets, quotes, code fences) so the text on
/// screen stays byte-identical to the source and cursor columns map 1:1 onto
/// characters. Returns an owned line so callers can style transient strings.
pub(crate) fn markdown_line(theme: &Theme, raw: &str) -> Line<'static> {
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

    let is_bullet =
        trimmed.starts_with("- ") || trimmed.starts_with("* ") || trimmed.starts_with("+ ");
    if is_bullet {
        // Color the bullet marker so lists scan visually.
        let indent_len = raw.len() - trimmed.len();
        let (indent, rest) = raw.split_at(indent_len);
        let (marker, tail) = rest.split_at(1);
        Line::from(vec![
            Span::raw(indent.to_string()),
            Span::styled(marker.to_string(), Style::default().fg(theme.project)),
            Span::styled(tail.to_string(), base),
        ])
    } else {
        Line::from(Span::styled(raw.to_string(), base))
    }
}

/// Split a line into display chunks of at most `w` characters. Always
/// returns at least one (possibly empty) chunk so empty lines keep a row.
fn chunk_chars(s: &str, w: usize) -> Vec<String> {
    if s.is_empty() {
        return vec![String::new()];
    }
    let chars: Vec<char> = s.chars().collect();
    chars.chunks(w).map(|c| c.iter().collect()).collect()
}

/// Overlay the cursor cell at character column `x` of a display chunk. In
/// insert mode the cursor may sit one past the end of the chunk (append
/// position), rendered as a highlighted space.
fn apply_cursor_at(theme: &Theme, line: Line<'_>, chunk: &str, x: usize) -> Line<'static> {
    let col = x.min(chunk.chars().count());
    let cursor_style = Style::default()
        .fg(theme.bg)
        .bg(theme.cursor)
        .add_modifier(Modifier::BOLD);

    let start = chunk
        .char_indices()
        .nth(col)
        .map_or(chunk.len(), |(i, _)| i);
    let end = chunk[start..]
        .char_indices()
        .nth(1)
        .map_or(chunk.len(), |(i, _)| start + i);

    // Preserve the line-level style by reusing the first span's style for
    // the surrounding text (bullet-marker coloring is lost on the cursor
    // row — an acceptable trade for a simple, correct cursor).
    let base = line.spans.first().map_or(Style::default(), |s| s.style);
    let before = chunk[..start].to_string();
    let cursor_txt = if start == chunk.len() {
        " ".to_string()
    } else {
        chunk[start..end].to_string()
    };
    let after = chunk.get(end..).unwrap_or("").to_string();

    Line::from(vec![
        Span::styled(before, base),
        Span::styled(cursor_txt, cursor_style),
        Span::styled(after, base),
    ])
}
