//! Resultados da busca em notas (`?query`, ver `app::note_search`). Tela
//! cheia read-only, no mesmo molde de `settings`/`issues`: `j`/`k` navega,
//! `Enter` abre a nota no note panel, `Esc`/`q` volta.

use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;

use crate::app::App;
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

    let hits = app.note_search_results();
    let title = format!("? {}", app.note_search_query());
    header::render(
        frame,
        header_area,
        theme,
        header::HeaderProps {
            title: Some(&title),
            count: hits.len(),
            sort: "notes",
            filter: None,
        },
    );

    if hits.is_empty() {
        let msg = tr(
            "no notes match — Esc to go back",
            "nenhuma nota bateu — Esc volta",
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

    let cursor = app.note_search_cursor();
    // Cada hit ocupa 2 linhas de tela (título + trecho), então a janela de
    // scroll considera o dobro de linhas por item.
    let rows_per_hit = 2;
    let height_rows = usize::from(body_area.height).max(1);
    let height_hits = (height_rows / rows_per_hit).max(1);
    let start = if cursor >= height_hits {
        cursor + 1 - height_hits
    } else {
        0
    };
    let end = (start + height_hits).min(hits.len());

    let mut lines: Vec<Line> = Vec::with_capacity((end - start) * rows_per_hit);
    for (i, hit) in hits.iter().enumerate().take(end).skip(start) {
        let selected = i == cursor;
        let prefix = if selected { "▸ " } else { "  " };
        let bg = if selected { theme.selected } else { theme.bg };
        // `▤` marca "achei numa nota" — nunca confundir com uma linha de
        // tarefa (`▸`/`(pri)`/`✓`), símbolo carrega o sentido, não a cor.
        let label = hit.task_title.as_deref().unwrap_or(&hit.rel);
        lines.push(
            Line::from(vec![
                Span::styled(prefix.to_string(), Style::default().fg(theme.accent)),
                Span::styled("▤ ", Style::default().fg(theme.project)),
                Span::styled(
                    label.to_string(),
                    Style::default().fg(theme.fg).add_modifier(if selected {
                        Modifier::BOLD
                    } else {
                        Modifier::empty()
                    }),
                ),
                Span::styled(format!("  {}", hit.rel), Style::default().fg(theme.dim)),
            ])
            .style(Style::default().bg(bg)),
        );
        if !hit.snippet.is_empty() {
            lines.push(
                Line::from(Span::styled(
                    format!("      {}", hit.snippet),
                    Style::default().fg(theme.dim),
                ))
                .style(Style::default().bg(bg)),
            );
        } else {
            lines.push(Line::from("").style(Style::default().bg(bg)));
        }
    }
    frame.render_widget(
        Paragraph::new(lines).style(Style::default().bg(theme.bg)),
        body_area,
    );
}
