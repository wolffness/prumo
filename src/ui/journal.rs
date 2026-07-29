//! Journal do projeto em foco (`Shift+J`): linha do tempo rolável.

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

    let project = app.journal_project().unwrap_or("—");
    let entries = app.journal_entries();
    let title = format!("+{project} — journal");
    header::render(
        frame,
        header_area,
        theme,
        header::HeaderProps {
            title: Some(&title),
            count: entries.len(),
            sort: "date",
            filter: None,
        },
    );

    if entries.is_empty() {
        let msg = tr(
            "no entries yet — n to log the first one",
            "nenhuma entrada ainda — n pra registrar a primeira",
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

    let cursor = app.journal_cursor();
    let height = usize::from(body_area.height).max(1);
    let start = if cursor >= height { cursor + 1 - height } else { 0 };
    let end = (start + height).min(entries.len());

    let mut lines: Vec<Line> = Vec::with_capacity(end - start);
    for (i, entry) in entries.iter().enumerate().take(end).skip(start) {
        let selected = i == cursor;
        let prefix = if selected { "▸ " } else { "  " };
        let bg = if selected { theme.selected } else { theme.bg };
        // Cada entrada é 1 linha na lista mesmo quando o texto tem vários
        // parágrafos — mostra só a primeira linha como prévia, com `…`
        // indicando que há mais (símbolo, não cor, carrega a informação).
        let mut it = entry.text.lines();
        let first = it.next().unwrap_or("");
        let has_more = it.next().is_some();
        let preview = if has_more {
            format!("{first} …")
        } else {
            first.to_string()
        };
        lines.push(
            Line::from(vec![
                Span::styled(prefix.to_string(), Style::default().fg(theme.accent)),
                Span::styled(format!("{}  ", entry.date), Style::default().fg(theme.dim)),
                Span::styled(
                    preview,
                    Style::default().fg(theme.fg).add_modifier(if selected {
                        Modifier::BOLD
                    } else {
                        Modifier::empty()
                    }),
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
