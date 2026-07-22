//! Visão Kanban do board GitHub Project v2 ("Centro de comando"). Read-only
//! nesta fatia: três colunas por Status, cada card com repo#nº, agente e
//! título. Estado sempre por símbolo+texto, nunca só cor (daltonismo).

use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;

use crate::advisor::kanban::{COLUMNS, KanbanCard};
use crate::app::App;
use crate::brand::tr;

use super::header;

/// Símbolo da coluna — identidade por forma, não por cor.
fn column_symbol(column: &str) -> &'static str {
    match column {
        "In Progress" => "▶",
        "Done" => "✔",
        _ => "⏸",
    }
}

pub fn render(frame: &mut Frame, area: Rect, app: &App) {
    let theme = app.theme();
    super::fill_bg(frame, area, Style::default().bg(theme.bg));

    let [header_area, _spacer, body_area] = Layout::vertical([
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Min(1),
    ])
    .areas(area);

    header::render(
        frame,
        header_area,
        theme,
        header::HeaderProps {
            title: Some("kanban"),
            count: app.kanban().len(),
            sort: "gh",
            filter: None,
        },
    );

    let cards = app.kanban();
    if cards.is_empty() {
        let msg = tr(
            "empty board — press r to refresh, Esc to go back",
            "board vazio — r atualiza, Esc volta",
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

    let columns: [Rect; 3] = Layout::horizontal([
        Constraint::Ratio(1, 3),
        Constraint::Ratio(1, 3),
        Constraint::Ratio(1, 3),
    ])
    .areas(body_area);

    // Card selecionado = posição do cursor na ordem visível (coluna a coluna),
    // a mesma ordem que `j`/`k` percorrem no App.
    let selected = app
        .kanban_visible_order()
        .get(app.kanban_cursor())
        .copied();
    for (col_area, col_name) in columns.into_iter().zip(COLUMNS) {
        render_column(frame, col_area, app, col_name, cards, selected);
    }
}

/// Uma coluna do board: título `⏸ Todo (n)` + cards empilhados. O card
/// selecionado ganha `▸` e negrito (símbolo, não só cor).
fn render_column(
    frame: &mut Frame,
    area: Rect,
    app: &App,
    column: &str,
    cards: &[KanbanCard],
    selected: Option<usize>,
) {
    let theme = app.theme();
    let width = usize::from(area.width).saturating_sub(3).max(8);
    let in_column: Vec<(usize, &KanbanCard)> = cards
        .iter()
        .enumerate()
        .filter(|(_, c)| c.status == column)
        .collect();

    let mut lines: Vec<Line> = Vec::with_capacity(2 + in_column.len() * 2);
    lines.push(Line::from(Span::styled(
        format!(" {} {} ({})", column_symbol(column), column, in_column.len()),
        Style::default().fg(theme.accent).add_modifier(Modifier::BOLD),
    )));
    lines.push(Line::default());

    if in_column.is_empty() {
        lines.push(Line::from(Span::styled(
            " —".to_string(),
            Style::default().fg(theme.dim),
        )));
    }
    for (idx, card) in in_column {
        let is_selected = selected == Some(idx);
        let bg = if is_selected { theme.selected } else { theme.bg };
        let prefix = if is_selected { "▸" } else { " " };
        let agent = if card.agent.is_empty() {
            tr("unassigned", "sem agente").to_string()
        } else {
            card.agent.clone()
        };
        lines.push(
            Line::from(vec![
                Span::styled(prefix.to_string(), Style::default().fg(theme.accent)),
                Span::styled(
                    format!("#{} ", card.number),
                    Style::default().fg(theme.pri_a),
                ),
                Span::styled(format!("· {agent}"), Style::default().fg(theme.dim)),
            ])
            .style(Style::default().bg(bg)),
        );
        let mut title = card.title.clone();
        if title.chars().count() > width {
            title = title.chars().take(width.saturating_sub(1)).collect::<String>() + "…";
        }
        lines.push(
            Line::from(Span::styled(
                format!("   {title}"),
                Style::default().fg(theme.fg).add_modifier(if is_selected {
                    Modifier::BOLD
                } else {
                    Modifier::empty()
                }),
            ))
            .style(Style::default().bg(bg)),
        );
    }

    frame.render_widget(
        Paragraph::new(lines).style(Style::default().bg(theme.bg)),
        area,
    );
}
