//! Visão Review (`R`): lista de `+projeto`s por atraso de revisão. Ações
//! (navegar/entrar) são teclas tratadas em `main::handle_review`.

use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;

use crate::app::{App, ReviewStatus};
use crate::brand::tr;

use super::header;

pub fn render(frame: &mut Frame, area: Rect, app: &App) {
    let theme = app.theme();
    super::fill_bg(frame, area, Style::default().bg(theme.bg));

    let [header_area, _spacer, body_area] = Layout::vertical([
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Min(1),
    ])
    .areas(area);

    let rows = app.review_rows();
    header::render(
        frame,
        header_area,
        theme,
        header::HeaderProps {
            title: Some("review"),
            count: rows.len(),
            sort: "atraso",
            filter: None,
        },
    );

    if rows.is_empty() {
        let msg = tr(
            "no +projects with open tasks",
            "nenhum +projeto com tarefas abertas",
        );
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

    let every_days = app.review_every_days();
    let cursor = app.review_cursor();
    let height = usize::from(body_area.height).max(1);
    let start = if cursor >= height { cursor + 1 - height } else { 0 };
    let end = (start + height).min(rows.len());

    let mut lines: Vec<Line> = Vec::with_capacity(end - start);
    for (i, row) in rows.iter().enumerate().take(end).skip(start) {
        let selected = i == cursor;
        let prefix = if selected { "▸ " } else { "  " };
        let bg = if selected { theme.selected } else { theme.bg };
        let (symbol, color) = match row.status(every_days) {
            ReviewStatus::Overdue => ("⚠", theme.overdue),
            ReviewStatus::Due => ("⏰", theme.pri_a),
            ReviewStatus::Fresh => ("✔", theme.done),
        };
        let since = match row.days_since {
            None => tr("never reviewed", "nunca revisado").to_string(),
            Some(0) => tr("today", "hoje").to_string(),
            Some(d) => format!("{d}{}", tr("d ago", "d atrás")),
        };
        lines.push(
            Line::from(vec![
                Span::styled(prefix.to_string(), Style::default().fg(theme.accent)),
                Span::styled(format!("{symbol} "), Style::default().fg(color)),
                Span::styled(
                    format!("+{:<14}", row.project),
                    Style::default().fg(theme.project).add_modifier(if selected {
                        Modifier::BOLD
                    } else {
                        Modifier::empty()
                    }),
                ),
                Span::styled(format!("{since:<16}"), Style::default().fg(theme.dim)),
                Span::styled(
                    format!("{} {}", row.open_count, tr("open", "abertas")),
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
