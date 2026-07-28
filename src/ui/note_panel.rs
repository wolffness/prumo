use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};

use crate::app::App;
use crate::brand::tr;
use crate::theme::Theme;

/// Render the in-TUI note panel: title bar with the task body, the note's
/// lines with lightweight per-line Markdown styling, a cursor, and a footer
/// hint row that follows the panel's view/insert mode.
pub fn render(frame: &mut Frame, area: Rect, app: &App) {
    let theme = app.theme();
    let Some(panel) = app.note_panel.as_ref() else {
        return;
    };

    let mode_label = if panel.insert {
        tr(" editing ", " editando ")
    } else {
        tr(" note ", " nota ")
    };
    let dirty_mark = if panel.dirty { "* " } else { "" };
    let mut title_spans = vec![
        Span::raw(" "),
        Span::styled(
            format!("{dirty_mark}{}", panel.title),
            Style::default()
                .fg(theme.accent)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(format!(" ·{mode_label}"), Style::default().fg(theme.dim)),
    ];
    if app.dispatch_ctx.as_ref().is_some_and(|c| c.briefed) {
        title_spans.push(Span::styled(
            tr("· ✔ brief ready ", "· ✔ brief pronto "),
            Style::default().fg(theme.accent),
        ));
    }
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.border).bg(theme.panel))
        .title(Line::from(title_spans))
        .style(Style::default().bg(theme.panel));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    // O draft de despacho ganha um footer próprio de 3 linhas (status,
    // comandos primários, atalhos secundários).
    let footer_rows = if app.dispatch_ctx.is_some() { 3 } else { 1 };
    let [body_area, footer_area] =
        Layout::vertical([Constraint::Min(1), Constraint::Length(footer_rows)]).areas(inner);

    // Hard-wrap each buffer line into width-sized display rows. Char-exact
    // chunking (not word wrap) keeps the cursor mapping trivial: display
    // row = col / w, x = col % w — no reflow bookkeeping.
    let wrap_w = usize::from(body_area.width).max(8);
    // Publish the wrap width so vertical cursor motion (handle_note) can
    // step through the same visual rows the renderer draws.
    panel.wrap_w.set(wrap_w);
    let sel = panel.selection_range();
    let mut lines: Vec<Line> = Vec::new();
    let mut cursor_display_row = 0usize;
    // Checkbox buffer lines double as mouse targets: `(display_row, buffer
    // row)`, converted to screen rects once the scroll offset is known.
    let mut checkbox_rows: Vec<(usize, usize)> = Vec::new();
    for (i, raw) in panel.lines.iter().enumerate() {
        let chunks = chunk_chars(raw, wrap_w);
        let cursor_chunk = (panel.col / wrap_w).min(chunks.len() - 1);
        if i == panel.row {
            cursor_display_row = lines.len() + cursor_chunk;
        }
        if crate::subtasks::checkbox_state(raw).is_some() {
            checkbox_rows.push((lines.len(), i));
        }
        let line_len = raw.chars().count();
        for (ci, chunk) in chunks.iter().enumerate() {
            let chunk_start = ci * wrap_w;
            let chunk_len = chunk.chars().count();
            // Selection overlap with this chunk, in chunk-local char coords.
            let sel_local = sel.and_then(|((r1, c1), (r2, c2))| {
                if i < r1 || i > r2 {
                    return None;
                }
                let s = if i == r1 { c1 } else { 0 };
                let e = if i == r2 { c2 } else { line_len };
                let s = s.max(chunk_start);
                let e = e.min(chunk_start + chunk_len);
                (s < e).then(|| (s - chunk_start, e - chunk_start))
            });
            let cursor_local =
                (i == panel.row && ci == cursor_chunk).then(|| panel.col - cursor_chunk * wrap_w);
            // Chunks carrying the cursor or a selection get char-precise
            // spans (dropping Markdown coloring there); plain chunks keep
            // the line-level Markdown styling.
            let line = if sel_local.is_some() || cursor_local.is_some() {
                chunk_line(theme, chunk, sel_local, cursor_local)
            } else if ci == 0 {
                markdown_line(theme, chunk)
            } else {
                Line::from(Span::styled(chunk.clone(), Style::default().fg(theme.fg)))
            };
            lines.push(line);
        }
    }

    // Linha-guia (só display, fora do buffer) enquanto `## Instruções` está
    // vazia: ensina o fluxo sem obrigar o usuário a apagar um placeholder.
    if app.dispatch_ctx.is_some() && crate::app::instructions_empty(&panel.lines) {
        lines.push(Line::from(Span::styled(
            tr(
                "✎ write the raw idea; /prompt on a line turns it into a brief",
                "✎ escreva a ideia crua; /prompt numa linha vira brief",
            ),
            Style::default()
                .fg(theme.dim)
                .add_modifier(Modifier::ITALIC),
        )));
    }

    let offset = crate::ui::keep_cursor_visible(
        panel.scroll.get(),
        Some(cursor_display_row),
        body_area.height,
        lines.len(),
    );
    panel.scroll.set(offset);

    for (display_row, buffer_row) in checkbox_rows {
        let Some(rel) = display_row.checked_sub(usize::from(offset)) else {
            continue;
        };
        if rel < usize::from(body_area.height) {
            let rect = ratatui::layout::Rect {
                x: body_area.x,
                y: body_area.y + u16::try_from(rel).unwrap_or(u16::MAX),
                width: body_area.width,
                height: 1,
            };
            app.register_click_target(rect, crate::app::ClickAction::TogglePanelRow(buffer_row));
        }
    }

    frame.render_widget(
        Paragraph::new(lines)
            .style(Style::default().bg(theme.panel).fg(theme.fg))
            .scroll((offset, 0)),
        body_area,
    );

    let footer: Vec<Line> = if let Some(ctx) = app.dispatch_ctx.as_ref() {
        dispatch_footer(theme, ctx)
    } else {
        let hint = if panel.insert {
            tr(
                "Esc view · Enter newline · Ctrl-S save",
                "Esc visualizar · Enter nova linha · Ctrl-S salvar",
            )
        } else {
            tr(
                "i edit · Space toggle · n subtask · o editor · Esc/q close (saves)",
                "i editar · Espaço alternar · n subtarefa · o editor · Esc/q fechar (salva)",
            )
        };
        vec![Line::from(Span::styled(
            format!(" {hint}"),
            Style::default().fg(theme.dim),
        ))]
    };
    frame.render_widget(
        Paragraph::new(footer).style(Style::default().bg(theme.panel)),
        footer_area,
    );
}

/// Footer de 3 linhas do draft de despacho: status (agente + dir/aviso),
/// comandos `/` primários por fase, atalhos secundários. Cada estado carrega
/// um símbolo próprio (✎ ⧗ ✔ ⚠ ▶) — cor é só reforço.
fn dispatch_footer(theme: &Theme, ctx: &crate::app::DispatchCtx) -> Vec<Line<'static>> {
    let accent = Style::default()
        .fg(theme.accent)
        .add_modifier(Modifier::BOLD);
    let dim = Style::default().fg(theme.dim);
    let fg = Style::default().fg(theme.fg);

    let status = Line::from(vec![
        Span::styled(format!(" {}", ctx.agent_label()), accent),
        Span::styled(format!(" · {}", ctx.status_rest()), dim),
    ]);

    let primary = if ctx.busy {
        Line::from(Span::styled(
            format!(
                " ⧗ {}",
                tr(
                    "wait — the brief will replace ## Instructions",
                    "aguarde — o brief vai substituir a seção ## Instruções"
                )
            ),
            dim,
        ))
    } else if ctx.warning().is_some() {
        Line::from(Span::styled(
            format!(
                " ⚠ {}",
                tr(
                    "resolve the warning above before dispatching",
                    "resolva o aviso acima antes de despachar"
                )
            ),
            dim,
        ))
    } else {
        // Brief pronto promove o despacho a primeiro comando.
        let cmds: [(&str, &str); 3] = if ctx.briefed {
            [
                ("/go", tr("▶ dispatch", "▶ despachar")),
                ("/prompt", tr("✎ redo brief", "✎ refazer brief")),
                ("/codex", tr("agent", "agente")),
            ]
        } else {
            [
                ("/prompt", tr("✎ make a brief", "✎ gerar brief")),
                ("/go", tr("▶ dispatch", "▶ despachar")),
                ("/codex", tr("agent", "agente")),
            ]
        };
        let mut spans = Vec::new();
        for (cmd, label) in cmds {
            spans.push(Span::styled(format!(" {cmd}"), accent));
            spans.push(Span::styled(format!(" {label} "), fg));
        }
        spans.push(Span::styled(
            tr("(type on a line + Enter)", "(digite numa linha + Enter)").to_string(),
            dim,
        ));
        Line::from(spans)
    };

    let secondary = Line::from(Span::styled(
        format!(
            " {}",
            tr(
                "shortcuts: Ctrl-P brief · Tab agent · Ctrl-D dispatch · Ctrl-S save · o editor · Esc/q close (saves)",
                "atalhos: Ctrl-P brief · Tab agente · Ctrl-D despachar · Ctrl-S salvar · o editor · Esc/q fechar (salva)"
            )
        ),
        dim,
    ));

    vec![status, primary, secondary]
}

/// Per-line Markdown styling shared by the note panel and the DETAIL pane.
/// Line-level only (headings, bullets, quotes, code fences) so the text on
/// screen stays byte-identical to the source and cursor columns map 1:1 onto
/// characters. Returns an owned line so callers can style transient strings.
pub(crate) fn markdown_line(theme: &Theme, raw: &str) -> Line<'static> {
    let trimmed = raw.trim_start();
    // Completed subtasks fade out so open ones carry the visual weight.
    if crate::subtasks::checkbox_state(raw) == Some(true) {
        return Line::from(Span::styled(
            raw.to_string(),
            Style::default()
                .fg(theme.done)
                .add_modifier(Modifier::CROSSED_OUT),
        ));
    }
    let base = if trimmed.starts_with('#') {
        Style::default()
            .fg(theme.accent)
            .add_modifier(Modifier::BOLD)
    } else if trimmed.starts_with("> ") || trimmed == ">" {
        Style::default()
            .fg(theme.dim)
            .add_modifier(Modifier::ITALIC)
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

/// Build a display chunk with char-precise styling: optional selection
/// background over `sel` (chunk-local, end exclusive) and an optional cursor
/// cell at `cursor` (may sit one past the end — the insert append position —
/// rendered as a highlighted space).
fn chunk_line(
    theme: &Theme,
    chunk: &str,
    sel: Option<(usize, usize)>,
    cursor: Option<usize>,
) -> Line<'static> {
    let base = Style::default().fg(theme.fg);
    let sel_style = Style::default().fg(theme.fg).bg(theme.selection);
    let cursor_style = Style::default()
        .fg(theme.bg)
        .bg(theme.cursor)
        .add_modifier(Modifier::BOLD);

    let chars: Vec<char> = chunk.chars().collect();
    let cursor = cursor.map(|c| c.min(chars.len()));
    let mut spans: Vec<Span> = Vec::new();
    let mut buf = String::new();
    let mut cur_style = base;
    for idx in 0..=chars.len() {
        let style = if cursor == Some(idx) {
            cursor_style
        } else if sel.is_some_and(|(s, e)| idx >= s && idx < e) {
            sel_style
        } else {
            base
        };
        let ch = match chars.get(idx) {
            Some(c) => *c,
            // Append position: render a space cell only under the cursor.
            None if cursor == Some(idx) => ' ',
            None => break,
        };
        if style != cur_style && !buf.is_empty() {
            spans.push(Span::styled(std::mem::take(&mut buf), cur_style));
        }
        cur_style = style;
        buf.push(ch);
    }
    if !buf.is_empty() {
        spans.push(Span::styled(buf, cur_style));
    }
    Line::from(spans)
}
