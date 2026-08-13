//! Visão Projects (`P`): todo `+projeto` conhecido, arquivável como um todo.

use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;

use crate::app::App;
use crate::brand::tr;

use super::header;

pub fn render(frame: &mut Frame, area: Rect, app: &App) {
    if let Some(detail) = app.project_detail() {
        render_detail(frame, area, app, detail);
        return;
    }
    let theme = app.theme();
    super::fill_bg(frame, area, Style::default().bg(theme.bg));

    let [header_area, _spacer, body_area] = Layout::vertical([
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Min(1),
    ])
    .areas(area);

    let rows = app.project_rows();
    header::render(
        frame,
        header_area,
        theme,
        header::HeaderProps {
            title: Some("projects"),
            count: rows.len(),
            sort: "status",
            filter: None,
        },
    );

    if rows.is_empty() {
        let msg = tr("no projects yet", "nenhum projeto ainda");
        frame.render_widget(
            Paragraph::new(Line::from(Span::styled(
                format!("   {msg}"),
                Style::default().fg(theme.dim),
            )))
            .style(Style::default().bg(theme.bg)),
            body_area,
        );
        return;
    }

    let cursor = app.project_cursor();
    let height = usize::from(body_area.height).max(1);
    let start = if cursor >= height { cursor + 1 - height } else { 0 };
    let end = (start + height).min(rows.len());

    let mut lines: Vec<Line> = Vec::with_capacity(end - start);
    for (i, row) in rows.iter().enumerate().take(end).skip(start) {
        let selected = i == cursor;
        let prefix = if selected { "▸ " } else { "  " };
        let bg = if selected { theme.selected } else { theme.bg };
        let (symbol, color) = if row.archived {
            ("⊘", theme.dim)
        } else if row.open_count == 0 {
            ("○", theme.pri_a)
        } else {
            ("·", theme.done)
        };
        lines.push(
            Line::from(vec![
                Span::styled(prefix.to_string(), Style::default().fg(theme.accent)),
                Span::styled(format!("{symbol} "), Style::default().fg(color)),
                Span::styled(
                    format!("+{:<20}", row.name),
                    Style::default()
                        .fg(if row.archived { theme.dim } else { theme.project })
                        .add_modifier(if selected { Modifier::BOLD } else { Modifier::empty() }),
                ),
                Span::styled(
                    format!(
                        "{} {} · {} {}",
                        row.open_count,
                        tr("open", "abertas"),
                        row.done_count,
                        tr("done", "concluídas")
                    ),
                    Style::default().fg(theme.dim),
                ),
            ])
            .style(Style::default().bg(bg)),
        );
    }
    frame.render_widget(
        Paragraph::new(lines).style(Style::default().bg(theme.bg)),
        body_area,
    );
}

fn render_detail(
    frame: &mut Frame,
    area: Rect,
    app: &App,
    detail: &crate::app::projects::ProjectDetail,
) {
    let theme = app.theme();
    super::fill_bg(frame, area, Style::default().bg(theme.bg));
    let [header_area, _spacer, body_area] = Layout::vertical([
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Min(1),
    ])
    .areas(area);
    let title = format!("+{}", detail.name);
    header::render(
        frame,
        header_area,
        theme,
        header::HeaderProps {
            title: Some(&title),
            count: detail.rows.len(),
            sort: "status",
            filter: None,
        },
    );
    if detail.rows.is_empty() {
        frame.render_widget(
            Paragraph::new(Line::from(Span::styled(
                tr("   no tasks yet", "   nenhuma tarefa ainda"),
                Style::default().fg(theme.dim),
            )))
            .style(Style::default().bg(theme.bg)),
            body_area,
        );
        return;
    }
    let open_count = detail.rows.iter().filter(|row| !row.completed).count();
    let mut lines = Vec::new();
    let mut selected_line = 0;
    let mut in_completed = false;
    for (index, row) in detail.rows.iter().enumerate() {
        if row.completed && !in_completed {
            if !lines.is_empty() {
                lines.push(Line::raw(""));
            }
            lines.push(section_header(
                theme,
                tr("COMPLETED", "CONCLUÍDAS"),
                detail.rows.len() - open_count,
            ));
            in_completed = true;
        } else if index == 0 {
            lines.push(section_header(theme, tr("OPEN", "ABERTAS"), open_count));
        }
        let selected = index == detail.cursor;
        if selected {
            selected_line = lines.len();
        }
        let bg = if selected { theme.selected } else { theme.bg };
        let symbol = if row.completed { "✓" } else { "·" };
        let color = if row.completed { theme.dim } else { theme.done };
        lines.push(
            Line::from(vec![
                Span::styled(if selected { "▸ " } else { "  " }, Style::default().fg(theme.accent)),
                Span::styled(format!("{symbol} "), Style::default().fg(color)),
                Span::styled(row.raw.clone(), Style::default().fg(if row.completed { theme.dim } else { theme.fg })),
            ])
            .style(Style::default().bg(bg)),
        );
    }
    let height = usize::from(body_area.height).max(1);
    let scroll = selected_line.saturating_sub(height.saturating_sub(1)) as u16;
    frame.render_widget(
        Paragraph::new(lines)
            .scroll((scroll, 0))
            .style(Style::default().bg(theme.bg)),
        body_area,
    );
}

fn section_header<'a>(theme: &crate::theme::Theme, label: &str, count: usize) -> Line<'a> {
    Line::from(vec![
        Span::styled(label.to_string(), Style::default().fg(theme.accent).add_modifier(Modifier::BOLD)),
        Span::styled(format!(" ({count})"), Style::default().fg(theme.dim)),
    ])
}
