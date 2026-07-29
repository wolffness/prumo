//! Busca full-text no conteúdo das notas (`?query` na barra de busca `/`),
//! separada da busca de linha existente (que só filtra `task.raw` em
//! memória). Notas são arquivos `.md` reais em disco — sem índice: grep sob
//! demanda, disparado só no Enter (nunca por tecla, que seria caro em I/O).

use std::path::{Path, PathBuf};

use super::App;

/// Um resultado da busca em notas: o arquivo, o trecho que bateu, e a
/// tarefa que o linka (via token `note:`), se houver.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NoteSearchHit {
    pub path: PathBuf,
    /// Caminho relativo a `notes_dir`, para exibição.
    pub rel: String,
    /// Primeira linha que bateu a busca (trim'ada); vazia se o match foi só
    /// no nome do arquivo.
    pub snippet: String,
    /// Corpo da tarefa que aponta pra esta nota, se alguma aponta.
    pub task_title: Option<String>,
}

impl App {
    /// Roda a busca (case-insensitive) no conteúdo de todo `.md` sob
    /// `notes_dir`, guarda os resultados e entra na tela de resultados.
    pub fn run_note_search(&mut self, query: &str) {
        let notes_dir = self.notes_dir().clone();
        let needle = query.to_lowercase();
        let mut files = Vec::new();
        collect_markdown_files(&notes_dir, &mut files);
        files.sort();

        let mut hits = Vec::new();
        for path in files {
            let Ok(content) = std::fs::read_to_string(&path) else {
                continue;
            };
            let rel = path
                .strip_prefix(&notes_dir)
                .unwrap_or(&path)
                .to_string_lossy()
                .into_owned();
            let matched_line = content.lines().find(|l| l.to_lowercase().contains(&needle));
            let filename_matches = rel.to_lowercase().contains(&needle);
            if matched_line.is_none() && !filename_matches {
                continue;
            }
            let task_title = self
                .tasks()
                .iter()
                .find(|t| crate::note::target_for_task(t, &notes_dir).path == path)
                .map(|t| crate::todo::body_only(&t.raw));
            hits.push(NoteSearchHit {
                path,
                rel,
                snippet: matched_line.unwrap_or("").trim().to_string(),
                task_title,
            });
        }

        self.note_search_query = query.to_string();
        self.note_search_results = hits;
        self.note_search_cursor = 0;
        self.mode = super::Mode::NoteSearchResults;
    }

    pub fn note_search_query(&self) -> &str {
        &self.note_search_query
    }

    pub fn note_search_results(&self) -> &[NoteSearchHit] {
        &self.note_search_results
    }

    pub fn note_search_cursor(&self) -> usize {
        self.note_search_cursor
    }

    pub fn note_search_step(&mut self, down: bool) {
        let n = self.note_search_results.len();
        if n == 0 {
            return;
        }
        self.note_search_cursor = if down {
            (self.note_search_cursor + 1).min(n - 1)
        } else {
            self.note_search_cursor.saturating_sub(1)
        };
    }

    /// Abre o resultado sob o cursor no note panel (mesmo painel da tecla `m`).
    pub fn open_note_search_result(&mut self) {
        let Some(hit) = self.note_search_results.get(self.note_search_cursor) else {
            return;
        };
        let title = hit.task_title.clone().unwrap_or_else(|| hit.rel.clone());
        match super::NotePanel::load(hit.path.clone(), title) {
            Ok(panel) => {
                self.note_panel = Some(panel);
                self.mode = super::Mode::Note;
            }
            Err(e) => self.flash(format!("note read failed: {e}")),
        }
    }

    pub fn exit_note_search_results(&mut self) {
        self.mode = super::Mode::Normal;
    }
}

/// Coleta recursivamente todo `.md` sob `dir`. Silenciosamente ignora
/// subpastas ilegíveis (permissão, symlink quebrado) em vez de abortar a
/// busca inteira por um canto do diretório de notas.
fn collect_markdown_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_markdown_files(&path, out);
        } else if path.extension().is_some_and(|e| e == "md") {
            out.push(path);
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::app::test_support::build_app;

    fn tmp_notes_dir(tag: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "tuxedo-note-search-{tag}-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn finds_match_in_note_body_and_links_task() {
        let notes_dir = tmp_notes_dir("body");
        std::fs::write(
            notes_dir.join("orcamento.md"),
            "# Notas\n\no orçamento revisado ficou acima do previsto\n",
        )
        .unwrap();

        let mut app = build_app("Revisar orcamento +work note:orcamento.md\n");
        app.set_notes_dir(notes_dir.clone());
        app.run_note_search("orçamento");

        assert_eq!(app.mode, crate::app::Mode::NoteSearchResults);
        let hits = app.note_search_results();
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].rel, "orcamento.md");
        assert!(hits[0].snippet.contains("revisado"));
        assert_eq!(hits[0].task_title.as_deref(), Some("Revisar orcamento"));

        let _ = std::fs::remove_dir_all(&notes_dir);
    }

    #[test]
    fn finds_match_in_filename_of_orphan_note() {
        let notes_dir = tmp_notes_dir("orphan");
        std::fs::create_dir_all(notes_dir.join("projects")).unwrap();
        std::fs::write(notes_dir.join("projects/casa-reforma.md"), "sem conteudo relevante\n").unwrap();

        let mut app = build_app("tarefa qualquer\n");
        app.set_notes_dir(notes_dir.clone());
        app.run_note_search("reforma");

        let hits = app.note_search_results();
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].rel, "projects/casa-reforma.md");
        assert!(hits[0].task_title.is_none(), "nota órfã, sem tarefa");

        let _ = std::fs::remove_dir_all(&notes_dir);
    }

    #[test]
    fn no_match_returns_empty_results() {
        let notes_dir = tmp_notes_dir("empty");
        std::fs::write(notes_dir.join("a.md"), "conteúdo qualquer\n").unwrap();

        let mut app = build_app("tarefa\n");
        app.set_notes_dir(notes_dir.clone());
        app.run_note_search("inexistente-xyz");

        assert!(app.note_search_results().is_empty());
        assert_eq!(app.mode, crate::app::Mode::NoteSearchResults);

        let _ = std::fs::remove_dir_all(&notes_dir);
    }

    #[test]
    fn step_clamps_within_bounds() {
        let notes_dir = tmp_notes_dir("step");
        std::fs::write(notes_dir.join("a.md"), "hit aqui\n").unwrap();
        std::fs::write(notes_dir.join("b.md"), "hit aqui também\n").unwrap();

        let mut app = build_app("t\n");
        app.set_notes_dir(notes_dir.clone());
        app.run_note_search("hit");
        assert_eq!(app.note_search_results().len(), 2);

        app.note_search_step(false);
        assert_eq!(app.note_search_cursor(), 0, "não desce abaixo de 0");
        app.note_search_step(true);
        app.note_search_step(true);
        assert_eq!(app.note_search_cursor(), 1, "não passa do último");

        let _ = std::fs::remove_dir_all(&notes_dir);
    }
}
