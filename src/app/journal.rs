//! Journal por projeto (`Shift+J`): linha do tempo rolável de entradas
//! datadas ("publiquei a campanha X", "aumentei a verba em Y") — não é uma
//! tarefa (não é "a fazer") nem a nota de uma tarefa específica, é um log
//! cronológico do projeto como um todo. Reusa `notes_dir`: um arquivo `.md`
//! por projeto, texto solto, sem struct de metadata estruturada.

use std::path::{Path, PathBuf};

use super::{App, Mode};

/// Uma entrada do journal: uma linha `YYYY-MM-DD  texto` no arquivo.
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

/// Parseia linhas `YYYY-MM-DD  texto` do conteúdo do arquivo, mais recente
/// primeiro (o arquivo cresce por append, então a última linha é a mais
/// nova — inverter dá a ordem de exibição).
fn parse_entries(content: &str) -> Vec<JournalEntry> {
    let mut out: Vec<JournalEntry> = content
        .lines()
        .filter_map(|l| {
            let date = l.get(0..10)?;
            chrono::NaiveDate::parse_from_str(date, "%Y-%m-%d").ok()?;
            let text = l.get(10..)?.trim();
            (!text.is_empty()).then(|| JournalEntry {
                date: date.to_string(),
                text: text.to_string(),
            })
        })
        .collect();
    out.reverse();
    out
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

    /// `n` no Journal: abre o prompt de 1 linha pra nova entrada.
    pub fn begin_journal_entry(&mut self) {
        if self.journal_project.is_none() {
            return;
        }
        self.draft_clear();
        self.mode = Mode::PromptJournalEntry;
    }

    /// Enter no prompt: grava `hoje  texto` no arquivo (cria o diretório se
    /// preciso) e recarrega a lista. Texto vazio não grava nada.
    pub fn commit_journal_entry(&mut self, text: &str) {
        let text = text.trim();
        let Some(project) = self.journal_project.clone() else {
            return;
        };
        if text.is_empty() {
            return;
        }
        let path = journal_path(self.notes_dir(), &project);
        if let Some(parent) = path.parent()
            && let Err(e) = std::fs::create_dir_all(parent)
        {
            self.flash(format!("journal mkdir failed: {e}"));
            return;
        }
        let line = format!("{}  {text}\n", self.today());
        let result = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
            .and_then(|mut f| {
                use std::io::Write;
                f.write_all(line.as_bytes())
            });
        match result {
            Ok(()) => {
                self.reload_journal_entries();
                self.journal_cursor = 0;
                self.flash(crate::brand::tr("entry logged", "entrada registrada"));
            }
            Err(e) => self.flash(format!("journal write failed: {e}")),
        }
    }
}

#[cfg(test)]
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
    fn parse_entries_reverses_to_newest_first_and_skips_garbage() {
        let content = "2026-07-01  criei o projeto\nlinha solta sem data\n2026-07-20  aumentei a verba\n";
        let entries = parse_entries(content);
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].date, "2026-07-20");
        assert_eq!(entries[0].text, "aumentei a verba");
        assert_eq!(entries[1].date, "2026-07-01");
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
    fn commit_journal_entry_appends_and_reloads_newest_first() {
        let mut app = build_app("a +work\n");
        let dir = app.file_path.parent().unwrap().to_path_buf();
        app.set_notes_dir(dir);
        app.filter.project = Some("work".to_string());
        app.enter_journal_view();
        assert!(app.journal_entries().is_empty());

        app.commit_journal_entry("publiquei a campanha");
        assert_eq!(app.journal_entries().len(), 1);
        assert_eq!(app.journal_entries()[0].text, "publiquei a campanha");

        app.commit_journal_entry("aumentei a verba");
        assert_eq!(app.journal_entries().len(), 2);
        assert_eq!(
            app.journal_entries()[0].text,
            "aumentei a verba",
            "entrada mais nova aparece primeiro"
        );
    }

    #[test]
    fn commit_journal_entry_ignores_blank_text() {
        let mut app = build_app("a +work\n");
        let dir = app.file_path.parent().unwrap().to_path_buf();
        app.set_notes_dir(dir);
        app.filter.project = Some("work".to_string());
        app.enter_journal_view();
        app.commit_journal_entry("   ");
        assert!(app.journal_entries().is_empty());
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
