//! Visão dedicada das issues abertas do GitHub do repo vinculado ao projeto
//! em foco. Read-only; ações (atualizar/abrir/importar) são teclas tratadas em
//! `main::handle_issues`. O ranking por objetivo (tier/porquê) vem na Fatia B.

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

    let project = app.filter().project.as_deref().unwrap_or("—");
    let title = format!("+{project} · issues");
    header::render(
        frame,
        header_area,
        theme,
        header::HeaderProps {
            title: Some(&title),
            count: app.issues().len(),
            sort: "gh",
            filter: None,
        },
    );

    let issues = app.issues();
    if issues.is_empty() {
        let msg = tr(
            "no open issues — press r to refresh, Esc to go back",
            "sem issues abertas — r atualiza, Esc volta",
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

    // Scroll simples: janela em torno do cursor para mantê-lo visível.
    let cursor = app.issues_cursor();
    let height = usize::from(body_area.height).max(1);
    let start = if cursor >= height {
        cursor + 1 - height
    } else {
        0
    };
    let end = (start + height).min(issues.len());

    let mut lines: Vec<Line> = Vec::with_capacity(end - start);
    for (i, row) in issues.iter().enumerate().take(end).skip(start) {
        let selected = i == cursor;
        let prefix = if selected { "▸ " } else { "  " };
        let bg = if selected { theme.selected } else { theme.bg };
        // Marcador de tier (Fatia B): `!!!`/`!!`/`!` por importância; vazio
        // enquanto não ranqueado. Símbolo por contagem, não cor (daltonismo).
        let tier = tier_symbol(row.tier);
        lines.push(
            Line::from(vec![
                Span::styled(prefix.to_string(), Style::default().fg(theme.accent)),
                Span::styled(format!("{:>3} ", tier), Style::default().fg(theme.pri_a)),
                Span::styled(
                    format!("#{:<6}", row.number),
                    Style::default().fg(theme.dim),
                ),
                Span::styled(
                    row.title.clone(),
                    Style::default().fg(theme.fg).add_modifier(if selected {
                        Modifier::BOLD
                    } else {
                        Modifier::empty()
                    }),
                ),
            ])
            .style(Style::default().bg(bg)),
        );
        // Linha de porquê (quando ranqueado), recuada e em cor dim.
        if let Some(why) = row.why.as_deref().filter(|w| !w.is_empty()) {
            lines.push(
                Line::from(Span::styled(
                    format!("        {why}"),
                    Style::default().fg(theme.dim),
                ))
                .style(Style::default().bg(bg)),
            );
        }
    }
    frame.render_widget(
        Paragraph::new(lines).style(Style::default().bg(theme.bg)),
        body_area,
    );
}

/// Símbolo de importância de uma issue ranqueada: 3=`!!!`, 2=`!!`, 1=`!`, vazio
/// quando não ranqueado. Por contagem (não cor) — acessível a daltônicos.
pub fn tier_symbol(tier: Option<u8>) -> &'static str {
    match tier {
        Some(3) => "!!!",
        Some(2) => "!!",
        Some(1) => "!",
        _ => "",
    }
}

#[cfg(test)]
mod tests {
    use super::tier_symbol;

    #[test]
    fn tier_symbol_by_count_not_color() {
        assert_eq!(tier_symbol(Some(3)), "!!!");
        assert_eq!(tier_symbol(Some(2)), "!!");
        assert_eq!(tier_symbol(Some(1)), "!");
        assert_eq!(tier_symbol(None), "");
        assert_eq!(tier_symbol(Some(9)), "");
    }
}
