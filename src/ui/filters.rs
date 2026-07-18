use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;

use crate::app::{App, ordered_unique};
use crate::brand::tr;
use crate::search::subseq_match_ci;
use crate::theme::Theme;
use crate::todo;

pub fn render(frame: &mut Frame, area: Rect, app: &App) {
    let theme = app.theme();
    super::fill_bg(frame, area, Style::default().bg(theme.panel));

    let projects = ordered_unique(app.tasks(), |t| &t.projects);
    let contexts = ordered_unique(app.tasks(), |t| &t.contexts);

    let mut lines: Vec<Line> = Vec::new();
    lines.push(line_pad(
        theme,
        vec![Span::styled(
            tr(" FILTERS", " FILTROS"),
            Style::default().fg(theme.dim).add_modifier(Modifier::BOLD),
        )],
    ));
    lines.push(line_pad(theme, vec![Span::raw(" ")]));
    lines.push(line_pad(
        theme,
        vec![Span::styled(
            tr(" PROJECTS", " PROJETOS"),
            Style::default()
                .fg(theme.project)
                .add_modifier(Modifier::BOLD),
        )],
    ));
    if projects.is_empty() {
        lines.push(hint_row(theme, "+project", theme.project));
    } else {
        for (name, count) in &projects {
            let active = app.filter.project.as_deref() == Some(name.as_str());
            let advisor_on = app.advisor_project_enabled(name);
            lines.push(filter_row(
                theme,
                "+",
                name,
                *count,
                active,
                theme.project,
                advisor_on,
            ));
        }
    }
    lines.push(line_pad(theme, vec![Span::raw(" ")]));
    lines.push(line_pad(
        theme,
        vec![Span::styled(
            tr(" CONTEXTS", " CONTEXTOS"),
            Style::default()
                .fg(theme.context)
                .add_modifier(Modifier::BOLD),
        )],
    ));
    if contexts.is_empty() {
        lines.push(hint_row(theme, "@context", theme.context));
    } else {
        for (name, count) in &contexts {
            let active = app.filter.context.as_deref() == Some(name.as_str());
            lines.push(filter_row(
                theme,
                "@",
                name,
                *count,
                active,
                theme.context,
                false,
            ));
        }
    }

    let saved = app.saved_filters();
    if !saved.is_empty() {
        lines.push(line_pad(theme, vec![Span::raw(" ")]));
        lines.push(line_pad(
            theme,
            vec![Span::styled(
                tr(" SAVED", " SALVAS"),
                Style::default()
                    .fg(theme.accent)
                    .add_modifier(Modifier::BOLD),
            )],
        ));
        for f in saved {
            let active = app.filter().search == f.query;
            let count = app
                .tasks()
                .iter()
                .filter(|t| {
                    !t.done
                        && subseq_match_ci(todo::body_after_priority(&t.raw), &f.query).is_some()
                })
                .count();
            lines.push(filter_row(
                theme,
                "",
                &f.name,
                count,
                active,
                theme.accent,
                false,
            ));
        }
    }

    let para = Paragraph::new(lines).style(Style::default().bg(theme.panel).fg(theme.fg));
    frame.render_widget(para, area);
}

#[allow(clippy::too_many_arguments)]
fn filter_row<'a>(
    theme: &Theme,
    sigil: &str,
    name: &'a str,
    count: usize,
    active: bool,
    sigil_color: ratatui::style::Color,
    advisor_on: bool,
) -> Line<'a> {
    let bg = if active { theme.selected } else { theme.panel };
    let prefix = if active { "▸ " } else { "  " };
    let mut padded = format!("{}{}", sigil, name);
    if padded.chars().count() < 16 {
        let pad = 16 - padded.chars().count();
        padded.push_str(&" ".repeat(pad));
    }
    // Coluna-marcador do advisor: `✦` quando ligado, espaço quando não —
    // presença/ausência de símbolo, não cor (acessível a daltônicos).
    let marker = advisor_marker(advisor_on);
    Line::from(vec![
        Span::styled(marker.to_string(), Style::default().fg(theme.accent)),
        Span::styled(prefix.to_string(), Style::default().fg(theme.accent)),
        Span::styled(padded, Style::default().fg(sigil_color)),
        Span::styled(format!("{:>3}", count), Style::default().fg(theme.dim)),
    ])
    .style(Style::default().bg(bg))
}

/// Símbolo do indicador de advisor de um projeto: `✦` quando ligado, espaço
/// quando não. Isolado para teste puro.
pub fn advisor_marker(on: bool) -> &'static str {
    if on { "✦" } else { " " }
}

fn hint_row<'a>(theme: &Theme, token: &'a str, token_color: ratatui::style::Color) -> Line<'a> {
    Line::from(vec![
        Span::raw("   "),
        Span::styled(
            tr("tag with ", "marque com "),
            Style::default().fg(theme.dim),
        ),
        Span::styled(token, Style::default().fg(token_color)),
    ])
    .style(Style::default().bg(theme.panel))
}

fn line_pad<'a>(theme: &Theme, spans: Vec<Span<'a>>) -> Line<'a> {
    Line::from(spans).style(Style::default().bg(theme.panel))
}

#[cfg(test)]
mod tests {
    use super::advisor_marker;

    #[test]
    fn advisor_marker_shows_star_only_when_on() {
        assert_eq!(advisor_marker(true), "✦");
        assert_eq!(advisor_marker(false), " ");
    }
}
