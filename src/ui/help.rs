use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};

use crate::app::App;
use crate::brand::tr;
use crate::theme::Theme;

type Section = (&'static str, Vec<(&'static str, &'static str)>);

// Sections are built at render time so every label can follow the invoked
// name's language (English as tuxedo, pt-BR as prumo) via `tr`.
fn navigation() -> Section {
    (
        tr("NAVIGATION", "NAVEGAÇÃO"),
        vec![
            ("j / ↓", tr("next task", "próxima tarefa")),
            ("k / ↑", tr("previous task", "tarefa anterior")),
            ("gg", tr("first task", "primeira tarefa")),
            ("G", tr("last task", "última tarefa")),
            (
                "Ctrl-d / Ctrl-u",
                tr("page down / up", "meia página abaixo / acima"),
            ),
        ],
    )
}

fn editing() -> Section {
    (
        tr("EDITING", "EDIÇÃO"),
        vec![
            ("n", tr("new task", "nova tarefa")),
            ("e", tr("edit line (normal)", "editar linha (normal)")),
            ("i", tr("edit line (insert)", "editar linha (inserir)")),
            ("r", tr("reschedule task", "reagendar tarefa")),
            ("x", tr("complete → archive", "concluir → arquivar")),
            ("dd", tr("delete task", "apagar tarefa")),
            (
                "p",
                tr("cycle priority A→B→C→·", "alternar prioridade A→B→C→·"),
            ),
            ("c", tr("add/remove context", "adicionar/remover contexto")),
            ("+", tr("add project", "adicionar projeto")),
            ("yy", tr("copy line to clipboard", "copiar linha")),
            ("yb", tr("copy body only", "copiar só o corpo")),
            ("u", tr("undo", "desfazer")),
        ],
    )
}

fn notes_files() -> Section {
    (
        tr("NOTES & FILES", "NOTAS E ARQUIVOS"),
        vec![
            (
                "m / N",
                tr("note panel (in-app)", "painel de nota (no app)"),
            ),
            (
                "  i / Esc",
                tr(
                    "panel: write / back·close",
                    "painel: escrever / voltar·fechar",
                ),
            ),
            (
                "  Shift+arrows",
                tr("panel: select text", "painel: selecionar texto"),
            ),
            (
                "  Del/Backspace",
                tr("panel: delete selection", "painel: apagar seleção"),
            ),
            (
                "  Space / n",
                tr(
                    "panel: toggle / new subtask",
                    "painel: alternar / nova subtarefa",
                ),
            ),
            (
                "  Ctrl-S / o",
                tr("panel: save / $EDITOR", "painel: salvar / $EDITOR"),
            ),
            (
                "o / O",
                tr(
                    "open / create note in $EDITOR",
                    "abrir / criar nota no $EDITOR",
                ),
            ),
            (
                "t",
                tr(
                    "attach file (drag or path)",
                    "anexar arquivo (arraste ou caminho)",
                ),
            ),
            ("Enter", tr("open attachments", "abrir anexos")),
        ],
    )
}

fn view() -> Section {
    (
        tr("VIEW", "VISÃO"),
        vec![
            ("/", tr("fuzzy search", "busca difusa")),
            (
                "fp / fc",
                tr("filter project/context", "filtrar projeto/contexto"),
            ),
            (
                "ff / fs",
                tr("saved filter pick/save", "filtro salvo: usar/salvar"),
            ),
            ("S", tr("cycle sort", "alternar ordenação")),
            (
                "v",
                tr("visual / multi-select", "visual / seleção múltipla"),
            ),
            ("l", tr("list view", "visão de lista")),
            ("a", tr("archive view", "visão de arquivo")),
            ("A", tr("archive completed", "arquivar concluídas")),
            ("H", tr("show done in list", "mostrar concluídas na lista")),
            ("F", tr("show future in list", "mostrar futuras na lista")),
            (
                "[ / ]",
                tr("toggle filter / detail", "alternar filtro / detalhe"),
            ),
            ("T", tr("cycle theme", "alternar tema")),
            ("D", tr("cycle density", "alternar densidade")),
            ("L", tr("toggle line numbers", "números de linha")),
        ],
    )
}

fn advisor_shell() -> Section {
    (
        tr("ADVISOR & SHELL", "ADVISOR E SHELL"),
        vec![
            ("I", tr("GitHub issues view", "visão de issues do GitHub")),
            (
                "  in view: p",
                tr("AI rank by goal", "ranquear por objetivo (IA)"),
            ),
            (
                "! advisor on/off +p",
                tr("enable/disable per project", "liga/desliga por projeto"),
            ),
            (
                "! advisor link",
                tr("link a GitHub repo", "vincular repo do GitHub"),
            ),
            (
                "! advisor goal +p",
                tr("set the ranking goal", "definir o objetivo do ranking"),
            ),
            (
                "/ then ! cmd",
                tr("run a shell command", "rodar comando de shell"),
            ),
        ],
    )
}

fn system() -> Section {
    (
        tr("SYSTEM", "SISTEMA"),
        vec![
            (": / Ctrl-P", tr("command palette", "paleta de comandos")),
            ("s", tr("share capture QR", "QR de captura")),
            ("? / ,", tr("help / settings", "ajuda / configurações")),
            ("q", tr("quit", "sair")),
        ],
    )
}

fn format_section() -> Section {
    (
        tr("FORMAT", "FORMATO"),
        vec![
            ("(A)", tr("priority A-Z", "prioridade A-Z")),
            (
                "YYYY-MM-DD",
                tr("creation / done date", "data de criação / conclusão"),
            ),
            ("+project", tr("project tag(s)", "tag(s) de projeto")),
            ("@context", tr("context tag(s)", "tag(s) de contexto")),
            ("due:YYYY-MM-DD", tr("due date", "data de vencimento")),
            (
                "rec:Nu",
                tr("recur (u in d/w/m/y/b)", "recorrência (u em d/w/m/y/b)"),
            ),
            (
                "rec:+Nu",
                tr("strict: anchor on due:", "estrita: ancora no due:"),
            ),
            (
                "x DATE BODY",
                tr("completed task prefix", "prefixo de tarefa concluída"),
            ),
        ],
    )
}

pub fn render(frame: &mut Frame, area: Rect, app: &App) {
    let theme = app.theme();
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.border).bg(theme.panel))
        .title(Line::from(vec![
            Span::raw(" "),
            Span::styled(
                crate::brand::app_name(),
                Style::default()
                    .fg(theme.accent)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!(" · {} ", tr("help", "ajuda")),
                Style::default().fg(theme.dim),
            ),
        ]))
        .style(Style::default().bg(theme.panel));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let bg = Style::default().bg(theme.panel).fg(theme.fg);

    // Keybindings (top, two columns) — divider — Format (bottom, two columns).
    // Each half splits sections across left/right; the last section in each
    // column drops its trailing blank so the divider lands tight.
    let kb_lines = two_columns(
        theme,
        inner.width,
        &[navigation(), editing(), system()],
        &[view(), notes_files(), advisor_shell()],
    );
    let kb_height = u16::try_from(kb_lines.len()).unwrap_or(u16::MAX);

    let format = format_section();
    let (fmt_left, fmt_right) = format.1.split_at(format.1.len().div_ceil(2));
    let fmt_left_section: Section = (format.0, fmt_left.to_vec());
    let fmt_right_section: Section = ("", fmt_right.to_vec());
    let fmt_lines = two_columns(
        theme,
        inner.width,
        &[fmt_left_section],
        &[fmt_right_section],
    );

    let [kb_area, divider, fmt_area] = Layout::vertical([
        Constraint::Length(kb_height),
        Constraint::Length(1),
        Constraint::Min(0),
    ])
    .areas(inner);

    frame.render_widget(Paragraph::new(kb_lines).style(bg), kb_area);
    frame.render_widget(
        Paragraph::new(Line::from(Span::styled(
            "─".repeat(usize::from(divider.width)),
            Style::default().fg(theme.border),
        )))
        .style(bg),
        divider,
    );
    frame.render_widget(Paragraph::new(fmt_lines).style(bg), fmt_area);
}

/// Render `left` and `right` section lists side-by-side. Each side gets half
/// the available width; rows are zipped so column heights stay aligned. The
/// trailing blank that `render_sections` adds after every section is dropped
/// from the last section in each column — visually that just means the column
/// ends flush with its final entry rather than carrying dead space below.
fn two_columns<'a>(
    theme: &Theme,
    total_width: u16,
    left: &[Section],
    right: &[Section],
) -> Vec<Line<'a>> {
    let left_lines = render_sections_trimmed(theme, left);
    let right_lines = render_sections_trimmed(theme, right);
    let rows = left_lines.len().max(right_lines.len());
    let half = usize::from(total_width / 2);
    let mut out: Vec<Line> = Vec::with_capacity(rows);
    for i in 0..rows {
        let mut spans: Vec<Span> = Vec::new();
        let left_spans: Vec<Span> = left_lines.get(i).map_or_else(Vec::new, |l| l.spans.clone());
        let left_width: usize = left_spans.iter().map(|s| s.content.chars().count()).sum();
        spans.extend(left_spans);
        if left_width < half {
            spans.push(Span::raw(" ".repeat(half - left_width)));
        }
        if let Some(r) = right_lines.get(i) {
            spans.extend(r.spans.clone());
        }
        out.push(Line::from(spans));
    }
    out
}

fn render_sections_trimmed<'a>(theme: &Theme, sections: &[Section]) -> Vec<Line<'a>> {
    let mut lines = render_sections(theme, sections);
    // Drop the trailing blank that `render_sections` appends after the last
    // section so columns end flush.
    if matches!(lines.last(), Some(line) if line_is_blank(line)) {
        lines.pop();
    }
    lines
}

fn line_is_blank(line: &Line) -> bool {
    line.spans
        .iter()
        .all(|s| s.content.chars().all(|c| c == ' '))
}

fn render_sections<'a>(theme: &Theme, sections: &[Section]) -> Vec<Line<'a>> {
    let mut lines: Vec<Line> = Vec::new();
    for (title, items) in sections {
        // An empty title means "this is a continuation column, skip the
        // header row" — used to align the right half of a 2-col section
        // (e.g. FORMAT) with the left half that owns the header.
        if title.is_empty() {
            lines.push(Line::raw(" "));
        } else {
            lines.push(Line::from(vec![
                Span::raw("  "),
                Span::styled(
                    (*title).to_string(),
                    Style::default()
                        .fg(theme.accent)
                        .add_modifier(Modifier::BOLD),
                ),
            ]));
        }
        for (k, d) in items {
            lines.push(Line::from(vec![
                Span::raw("    "),
                Span::styled(
                    pad_str(k, 18),
                    Style::default()
                        .fg(theme.context)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled((*d).to_string(), Style::default().fg(theme.fg)),
            ]));
        }
        lines.push(Line::raw(" "));
    }
    lines
}

fn pad_str(s: &str, w: usize) -> String {
    let len = s.chars().count();
    if len >= w {
        s.to_string()
    } else {
        let mut o = s.to_string();
        o.push_str(&" ".repeat(w - len));
        o
    }
}

#[cfg(test)]
mod tests {
    use super::advisor_shell;

    #[test]
    fn advisor_shell_section_lists_shell_and_advisor_commands() {
        let (title, items) = advisor_shell();
        assert!(title.contains("SHELL"));
        let keys: Vec<&str> = items.iter().map(|(k, _)| *k).collect();
        assert!(
            keys.iter().any(|k| k.contains('!')),
            "falta o comando shell"
        );
        assert!(keys.iter().any(|k| k.contains("advisor on/off")));
        assert!(keys.iter().any(|k| k.contains("advisor link")));
        assert!(keys.iter().any(|k| k.contains("advisor goal")));
        assert!(keys.contains(&"I"), "falta a visão de issues");
    }
}
