use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;

use crate::app::{App, DialogInputMode, Mode, View};
use crate::brand::tr;
use crate::ui::dialog::draft_cursor_spans;

pub fn render(frame: &mut Frame, area: Rect, app: &App) {
    let theme = app.theme();
    let mut mode_label: std::borrow::Cow<'static, str> = match app.mode {
        Mode::Normal => "NORMAL".into(),
        Mode::Insert => match app.draft.input_mode() {
            DialogInputMode::Normal => "NORMAL",
            DialogInputMode::Insert => tr("INSERT", "INSERIR"),
        }
        .into(),
        Mode::Search if app.search_is_shell() => "SHELL".into(),
        Mode::Search => tr("SEARCH", "BUSCA").into(),
        Mode::Issues => "ISSUES".into(),
        Mode::Visual => "VISUAL".into(),
        Mode::Help => tr("HELP", "AJUDA").into(),
        Mode::Settings => tr("SETTINGS", "CONFIG").into(),
        Mode::PromptProject => tr("PROJECT", "PROJETO").into(),
        Mode::PromptContext => tr("CONTEXT", "CONTEXTO").into(),
        Mode::PickProject => tr("PICK +PROJECT", "ESCOLHER +PROJETO").into(),
        Mode::PickContext => tr("PICK @CONTEXT", "ESCOLHER @CONTEXTO").into(),
        Mode::PickSavedFilter => tr("PICK FILTER", "ESCOLHER FILTRO").into(),
        Mode::PromptSaveFilter => tr("SAVE FILTER", "SALVAR FILTRO").into(),
        Mode::PromptAttach => tr("ATTACH", "ANEXAR").into(),
        Mode::CommandPalette => tr("COMMAND", "COMANDO").into(),
        Mode::Share => tr("SHARE", "CAPTURA").into(),
        Mode::PickTheme => tr("PICK THEME", "ESCOLHER TEMA").into(),
        Mode::Welcome => tr("WELCOME", "BOAS-VINDAS").into(),
        Mode::Note => {
            if app.note_panel.as_ref().is_some_and(|p| p.insert) {
                tr("NOTE·INSERT", "NOTA·INSERIR").into()
            } else {
                tr("NOTE", "NOTA").into()
            }
        }
    };
    if matches!(app.view, View::Archive) {
        mode_label = tr("ARCHIVE", "ARQUIVO").into();
    }
    if let Some(f) = app.flash_active() {
        mode_label = format!("{mode_label} · {f}").into();
    }

    let hint = match app.mode {
        Mode::Insert => match app.draft.input_mode() {
            DialogInputMode::Normal => tr(
                "h/l navigate · w/b/e word · i/a insert · Enter save · Esc cancel",
                "h/l navegar · w/b/e palavra · i/a inserir · Enter salvar · Esc cancelar",
            ),
            DialogInputMode::Insert => tr("Enter save · Esc normal", "Enter salvar · Esc normal"),
        },
        Mode::Visual => tr(
            "space toggle · x complete · dd delete · Esc cancel",
            "espaço alternar · x concluir · dd apagar · Esc cancelar",
        ),
        Mode::Help => tr("? close help", "? fechar ajuda"),
        Mode::Settings => tr("Esc back", "Esc voltar"),
        Mode::PromptProject => tr(
            "type +project name · Enter save · Esc cancel",
            "digite o +projeto · Enter salvar · Esc cancelar",
        ),
        Mode::PromptContext => tr(
            "type @context name · Enter toggle · Esc cancel",
            "digite o @contexto · Enter alternar · Esc cancelar",
        ),
        Mode::PickProject => tr(
            "j/k or ↑↓ cycle projects · Enter keep · Esc clear",
            "j/k ou ↑↓ alternar projetos · Enter manter · Esc limpar",
        ),
        Mode::PickContext => tr(
            "j/k or ↑↓ cycle contexts · Enter keep · Esc clear",
            "j/k ou ↑↓ alternar contextos · Enter manter · Esc limpar",
        ),
        Mode::PickSavedFilter => tr(
            "j/k or ↑↓ cycle filters · Enter keep · Esc revert",
            "j/k ou ↑↓ alternar filtros · Enter manter · Esc reverter",
        ),
        Mode::PromptSaveFilter => tr(
            "type a filter name · Enter save · Esc cancel",
            "nomeie o filtro · Enter salvar · Esc cancelar",
        ),
        Mode::CommandPalette => tr(
            "type to filter · Enter run · Esc cancel",
            "digite para filtrar · Enter executar · Esc cancelar",
        ),
        Mode::Share => tr(
            "scan the QR · any key dismisses",
            "escaneie o QR · qualquer tecla fecha",
        ),
        Mode::Welcome => tr(
            "c create ./todo.txt · s open sample · q quit",
            "c criar ./todo.txt · s abrir exemplo · q sair",
        ),
        Mode::Issues => tr(
            "j/k move · g rank · r refresh · Enter open · + import · Esc back",
            "j/k mover · g ranquear · r atualizar · Enter abrir · + importar · Esc voltar",
        ),
        _ => tr(
            "j/k · n new · r reschedule · x done · / search · ? help · u undo · q quit",
            "j/k · n nova · r reagendar · x concluir · / buscar · ? ajuda · u desfazer · q sair",
        ),
    };

    let mut right_parts = Vec::new();
    if matches!(app.view, View::Archive) {
        right_parts.push(format!(
            "{} {}",
            app.archive().len(),
            tr("archived", "arquivadas")
        ));
    } else {
        right_parts.push(format!(
            "{} {}",
            app.visible_indices().len(),
            tr("open", "abertas")
        ));
    }
    if !app.selection.is_empty() {
        right_parts.push(format!(
            "{} {}",
            app.selection.len(),
            tr("selected", "selecionadas")
        ));
    }
    right_parts.push(app.today().to_string());
    right_parts.push(format!(
        "{} {}",
        crate::brand::app_name(),
        env!("CARGO_PKG_VERSION")
    ));
    // Track where the update suffix would slot in so we can paint it in the
    // accent color (the rest of the right text is dim).
    let update_suffix = app
        .update_available()
        .map(|tag| format!(" · ↑ {tag} ({} update)", crate::brand::app_name()));
    let right_text = right_parts.join(" · ");

    // Append a chord indicator (e.g. " g…") so two-key sequences like gg/dd/fp
    // give visible feedback on the first press. Only shown while armed.
    let chord_suffix = app
        .chord
        .active()
        .map(|c| format!(" {c}…"))
        .unwrap_or_default();
    // Layout: mode chip on left, hint in middle, right text right-aligned.
    let chip_text = format!(" {mode_label}{chord_suffix} ");
    let chip_w = chip_text.chars().count() as u16;
    let update_w = update_suffix
        .as_deref()
        .map(|s| s.chars().count() as u16)
        .unwrap_or(0);
    let right_w = right_text.chars().count() as u16 + update_w + 1;
    let middle_w = area.width.saturating_sub(chip_w).saturating_sub(right_w);

    let [chip_area, mid_area, right_area] = Layout::horizontal([
        Constraint::Length(chip_w),
        Constraint::Length(middle_w),
        Constraint::Length(right_w),
    ])
    .areas(area);

    let chip = Paragraph::new(Span::styled(
        chip_text,
        Style::default()
            .bg(theme.mode_bg)
            .fg(theme.mode_fg)
            .add_modifier(Modifier::BOLD),
    ))
    .style(Style::default().bg(theme.statusbar));
    frame.render_widget(chip, chip_area);

    let mid_line = Line::from(vec![
        Span::raw("  "),
        Span::styled(hint, Style::default().fg(theme.status_fg)),
    ])
    .style(Style::default().bg(theme.statusbar));
    frame.render_widget(
        Paragraph::new(mid_line).style(Style::default().bg(theme.statusbar)),
        mid_area,
    );

    let right_line = if let Some(suffix) = update_suffix {
        Line::from(vec![
            Span::styled(right_text, Style::default().fg(theme.dim)),
            Span::styled(
                suffix,
                Style::default()
                    .fg(theme.accent)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(" ", Style::default().fg(theme.dim)),
        ])
        .style(Style::default().bg(theme.statusbar))
    } else {
        Line::from(Span::styled(
            format!("{right_text} "),
            Style::default().fg(theme.dim),
        ))
        .style(Style::default().bg(theme.statusbar))
    };
    frame.render_widget(
        Paragraph::new(right_line)
            .style(Style::default().bg(theme.statusbar))
            .right_aligned(),
        right_area,
    );
}

pub fn render_command_line(frame: &mut Frame, area: Rect, app: &App) {
    let theme = app.theme();
    let is_shell = app.search_is_shell();
    let visible_count = app.visible_indices().len();
    // Texto iniciado por `!` é um comando shell: prefixo `SHELL` e dica de que
    // Enter executa (em vez de aceitar uma busca).
    let (prefix, prefix_color) = if is_shell {
        ("SHELL", theme.pri_a)
    } else {
        ("/", theme.accent)
    };
    let suggestion = if is_shell {
        format!(
            "  {} · Esc {}",
            tr("Enter runs", "Enter executa"),
            tr("cancel", "cancela")
        )
    } else {
        format!("  {visible_count} matches · Enter accept · Esc cancel")
    };
    let mut spans = vec![
        Span::raw(" "),
        Span::styled(
            prefix,
            Style::default()
                .fg(prefix_color)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(" "),
    ];
    spans.extend(draft_cursor_spans(
        app.draft.text(),
        app.draft.cursor(),
        theme.fg,
        theme.bg,
    ));
    spans.push(Span::styled(suggestion, Style::default().fg(theme.dim)));
    let line = Line::from(spans).style(Style::default().bg(theme.bg));
    frame.render_widget(
        Paragraph::new(line).style(Style::default().bg(theme.bg)),
        area,
    );
}
