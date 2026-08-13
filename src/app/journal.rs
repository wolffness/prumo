//! Journal por projeto (`Shift+J`): linha do tempo rolável de entradas
//! datadas, cada uma podendo ter múltiplos parágrafos ("publiquei a
//! campanha X\n\nDetalhes do rollout...") — não é uma tarefa (não é "a
//! fazer") nem a nota de uma tarefa específica, é um log cronológico do
//! projeto como um todo. Reusa `notes_dir`: um arquivo `.md` por projeto,
//! blocos `## YYYY-MM-DD` seguidos de texto livre, sem struct de metadata
//! estruturada.

use std::path::{Path, PathBuf};

use super::{App, Mode, NotePanel};

/// Uma entrada do journal: o bloco sob um cabeçalho `## YYYY-MM-DD`. `text`
/// preserva quebras de linha internas (múltiplos parágrafos).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JournalEntry {
    pub date: String,
    pub text: String,
}

/// Caminho do journal de um projeto: `<notes_dir>/journal/<slug>.md`.
pub fn journal_path(notes_dir: &Path, project: &str) -> PathBuf {
    notes_dir
        .join("journal")
        .join(format!("{}.md", crate::note::slugify(project)))
}

/// Parseia blocos `## YYYY-MM-DD\n\n<texto>\n\n` do conteúdo do arquivo,
/// mais recente primeiro (o arquivo cresce por append no fim — inverter a
/// ordem dos blocos dá a ordem de exibição).
fn parse_entries(content: &str) -> Vec<JournalEntry> {
    let mut out = Vec::new();
    let mut current: Option<(String, Vec<&str>)> = None;
    for line in content.lines() {
        if let Some(date) = line.strip_prefix("## ")
            && chrono::NaiveDate::parse_from_str(date, "%Y-%m-%d").is_ok()
        {
            if let Some((d, body)) = current.take() {
                push_entry(&mut out, d, body);
            }
            current = Some((date.to_string(), Vec::new()));
        } else if let Some((_, body)) = current.as_mut() {
            body.push(line);
        }
    }
    if let Some((d, body)) = current {
        push_entry(&mut out, d, body);
    }
    out.reverse();
    out
}

fn push_entry(out: &mut Vec<JournalEntry>, date: String, body: Vec<&str>) {
    let text = body.join("\n").trim().to_string();
    if !text.is_empty() {
        out.push(JournalEntry { date, text });
    }
}

impl App {
    /// Projeto do journal em foco no momento (durante `Mode::Journal`).
    pub fn journal_project(&self) -> Option<&str> {
        self.journal_project.as_deref()
    }

    pub fn journal_entries(&self) -> &[JournalEntry] {
        &self.journal_entries
    }

    pub fn journal_cursor(&self) -> usize {
        self.journal_cursor
    }

    /// Abre o journal do projeto em foco (`filter.project`, senão o
    /// `+projeto` da tarefa sob o cursor — mesma semente de `enter_pick_
    /// project`). Sem projeto pra resolver, avisa e não entra no modo.
    pub fn enter_journal_view(&mut self) {
        let Some(project) = self.filter.project.clone().or_else(|| {
            self.cur_abs()
                .and_then(|i| self.tasks().get(i))
                .and_then(|t| t.projects.first().cloned())
        }) else {
            self.flash(crate::brand::tr(
                "no +project in focus — fp first",
                "sem +projeto em foco — use fp antes",
            ));
            return;
        };
        self.journal_project = Some(project);
        self.reload_journal_entries();
        self.journal_cursor = 0;
        self.mode = Mode::Journal;
    }

    pub fn exit_journal_view(&mut self) {
        self.mode = Mode::Normal;
    }

    pub fn journal_step(&mut self, down: bool) {
        let n = self.journal_entries.len();
        if n == 0 {
            return;
        }
        self.journal_cursor = if down {
            (self.journal_cursor + 1).min(n - 1)
        } else {
            self.journal_cursor.saturating_sub(1)
        };
    }

    fn reload_journal_entries(&mut self) {
        let Some(project) = &self.journal_project else {
            return;
        };
        let path = journal_path(self.notes_dir(), project);
        self.journal_entries = std::fs::read_to_string(&path)
            .map(|c| parse_entries(&c))
            .unwrap_or_default();
    }

    /// `n` no Journal: abre um composer de texto livre (mesmo motor do
    /// painel de nota — multi-linha, insert mode, sem exigir arquivo real
    /// no disco ainda). `Ctrl-S` grava, `Esc` (em modo view) cancela.
    pub fn begin_journal_entry(&mut self) {
        if self.journal_project.is_none() {
            return;
        }
        let title = crate::brand::tr("new journal entry", "nova entrada de journal").to_string();
        self.note_panel = Some(NotePanel::blank(title));
        self.journal_compose = true;
        self.mode = Mode::Note;
    }

    /// `Ctrl-S` no composer: grava `## hoje\n\n<texto>\n\n` no arquivo (cria
    /// o diretório se preciso) e recarrega a lista. Texto vazio cancela sem
    /// gravar, igual `cancel_journal_entry`.
    pub fn commit_journal_entry(&mut self, text: &str) {
        let text = text.trim();
        let Some(project) = self.journal_project.clone() else {
            self.cancel_journal_entry();
            return;
        };
        if text.is_empty() {
            self.cancel_journal_entry();
            return;
        }
        let path = journal_path(self.notes_dir(), &project);
        if let Some(parent) = path.parent()
            && let Err(e) = std::fs::create_dir_all(parent)
        {
            self.flash(format!("journal mkdir failed: {e}"));
            return;
        }
        let block = format!("## {}\n\n{text}\n\n", self.today());
        let result = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
            .and_then(|mut f| {
                use std::io::Write;
                f.write_all(block.as_bytes())
            });
        match result {
            Ok(()) => {
                self.reload_journal_entries();
                self.journal_cursor = 0;
                self.flash(crate::brand::tr("entry logged", "entrada registrada"));
            }
            Err(e) => self.flash(format!("journal write failed: {e}")),
        }
        self.note_panel = None;
        self.journal_compose = false;
        self.mode = Mode::Journal;
    }

    /// `Esc` no composer (em modo view): fecha sem gravar nada.
    pub fn cancel_journal_entry(&mut self) {
        self.note_panel = None;
        self.journal_compose = false;
        self.mode = Mode::Journal;
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use crate::app::test_support::build_app;

    #[test]
    fn journal_path_slugifies_project_name() {
        let dir = Path::new("/tmp/notes");
        assert_eq!(
            journal_path(dir, "Campanha Verão"),
            PathBuf::from("/tmp/notes/journal/campanha-verao.md")
        );
    }

    #[test]
    fn parse_entries_reverses_and_preserves_multiline_paragraphs() {
        let content = "## 2026-07-01\n\ncriei o projeto\n\n## 2026-07-20\n\naumentei a verba\nem duas linhas\n\nsegundo parágrafo\n\n";
        let entries = parse_entries(content);
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].date, "2026-07-20");
        assert_eq!(
            entries[0].text,
            "aumentei a verba\nem duas linhas\n\nsegundo parágrafo"
        );
        assert_eq!(entries[1].date, "2026-07-01");
    }

    #[test]
    fn parse_entries_skips_garbage_and_empty_bodies() {
        let content = "linha solta sem header\n\n## 2026-07-01\n\n   \n\n## 2026-07-02\n\ntexto real\n\n";
        let entries = parse_entries(content);
        assert_eq!(entries.len(), 1, "bloco de 07-01 tem corpo vazio, é descartado");
        assert_eq!(entries[0].date, "2026-07-02");
    }

    #[test]
    fn enter_journal_view_seeds_from_filter_then_cursor_task() {
        let mut app = build_app("a +work\n");
        let dir = app.file_path.parent().unwrap().to_path_buf();
        app.set_notes_dir(dir);

        app.filter.project = Some("focused".to_string());
        app.enter_journal_view();
        assert_eq!(app.journal_project(), Some("focused"));
        assert_eq!(app.mode, Mode::Journal);

        app.filter.project = None;
        app.enter_journal_view();
        assert_eq!(app.journal_project(), Some("work"), "cai pro projeto da tarefa sob o cursor");
    }

    #[test]
    fn enter_journal_view_without_project_flashes_and_stays_normal() {
        let mut app = build_app("tarefa sem projeto\n");
        app.enter_journal_view();
        assert_eq!(app.mode, Mode::Normal);
        assert!(app.flash_active().is_some());
    }

    #[test]
    fn commit_journal_entry_appends_multiline_and_reloads_newest_first() {
        let mut app = build_app("a +work\n");
        let dir = app.file_path.parent().unwrap().to_path_buf();
        app.set_notes_dir(dir);
        app.filter.project = Some("work".to_string());
        app.enter_journal_view();
        assert!(app.journal_entries().is_empty());

        app.commit_journal_entry("publiquei a campanha\ncom duas linhas");
        assert_eq!(app.journal_entries().len(), 1);
        assert_eq!(
            app.journal_entries()[0].text,
            "publiquei a campanha\ncom duas linhas"
        );

        app.commit_journal_entry("aumentei a verba");
        assert_eq!(app.journal_entries().len(), 2);
        assert_eq!(
            app.journal_entries()[0].text,
            "aumentei a verba",
            "entrada mais nova aparece primeiro"
        );
    }

    #[test]
    fn commit_journal_entry_ignores_blank_text_and_returns_to_journal() {
        let mut app = build_app("a +work\n");
        let dir = app.file_path.parent().unwrap().to_path_buf();
        app.set_notes_dir(dir);
        app.filter.project = Some("work".to_string());
        app.enter_journal_view();
        app.begin_journal_entry();
        app.commit_journal_entry("   ");
        assert!(app.journal_entries().is_empty());
        assert_eq!(app.mode, Mode::Journal);
        assert!(app.note_panel.is_none());
    }

    #[test]
    fn begin_and_cancel_journal_entry_roundtrip_mode() {
        let mut app = build_app("a +work\n");
        let dir = app.file_path.parent().unwrap().to_path_buf();
        app.set_notes_dir(dir);
        app.filter.project = Some("work".to_string());
        app.enter_journal_view();
        app.begin_journal_entry();
        assert_eq!(app.mode, Mode::Note);
        assert!(app.note_panel.is_some());
        app.cancel_journal_entry();
        assert_eq!(app.mode, Mode::Journal);
        assert!(app.note_panel.is_none());
    }

    #[test]
    fn journal_step_clamps_within_bounds() {
        let mut app = build_app("a +work\n");
        let dir = app.file_path.parent().unwrap().to_path_buf();
        app.set_notes_dir(dir);
        app.filter.project = Some("work".to_string());
        app.enter_journal_view();
        app.commit_journal_entry("um");
        app.commit_journal_entry("dois");
        assert_eq!(app.journal_entries().len(), 2);

        app.journal_step(false);
        assert_eq!(app.journal_cursor(), 0);
        app.journal_step(true);
        app.journal_step(true);
        assert_eq!(app.journal_cursor(), 1);
    }
}
