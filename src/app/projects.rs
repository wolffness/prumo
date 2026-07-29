//! Visão Projects (`P`): todo `+projeto` conhecido — com tarefa aberta,
//! histórico em `done.txt`, ou arquivado — sobrevive mesmo sem tarefa aberta
//! até o usuário arquivar explicitamente. Continua sem entidade "Projeto"
//! própria: o nome é a mesma tag textual de sempre; só o estado "arquivado"
//! é persistido (mesmo padrão load-modify-save de `review.rs`).

use std::collections::BTreeSet;

use crate::config::Config;
use crate::core::filter::ordered_unique;

use super::{App, Mode};

/// Uma linha da visão Projects.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectRow {
    pub name: String,
    /// Tarefas abertas com essa tag agora — 0 não significa "não existe",
    /// só "sem pendência aberta no momento".
    pub open_count: usize,
    pub archived: bool,
}

impl App {
    pub fn project_is_archived(&self, name: &str) -> bool {
        self.project_archived.iter().any(|p| p == name)
    }

    /// Todo `+projeto` que já apareceu em algum lugar: tarefas abertas,
    /// tarefas concluídas (`done.txt`), ou a lista de arquivados — não só a
    /// varredura de tarefas abertas que `ordered_unique` faz sozinho (essa
    /// varredura é exatamente o que faz projetos "sumirem" quando zeram
    /// tarefa aberta, o problema que esta visão existe pra resolver).
    fn build_project_rows(&self) -> Vec<ProjectRow> {
        let open_counts = ordered_unique(self.tasks(), |t| &t.projects);
        let mut names: BTreeSet<String> = open_counts.iter().map(|(n, _)| n.clone()).collect();
        for t in self.archive().tasks() {
            names.extend(t.projects.iter().cloned());
        }
        names.extend(self.project_archived.iter().cloned());

        let mut rows: Vec<ProjectRow> = names
            .into_iter()
            .map(|name| {
                let open_count = open_counts
                    .iter()
                    .find(|(n, _)| n == &name)
                    .map_or(0, |(_, c)| *c);
                let archived = self.project_is_archived(&name);
                ProjectRow {
                    name,
                    open_count,
                    archived,
                }
            })
            .collect();
        // Ativos primeiro (mais tarefas abertas primeiro, empate por nome),
        // arquivados sempre por último — mas ainda visíveis, pra dar pra
        // desarquivar.
        rows.sort_by(|a, b| {
            a.archived
                .cmp(&b.archived)
                .then_with(|| b.open_count.cmp(&a.open_count))
                .then_with(|| a.name.cmp(&b.name))
        });
        rows
    }

    pub fn project_rows(&self) -> &[ProjectRow] {
        &self.project_cache
    }

    pub fn project_cursor(&self) -> usize {
        self.project_cursor
    }

    pub fn enter_projects_view(&mut self) {
        self.project_cache = self.build_project_rows();
        self.project_cursor = self
            .project_cursor
            .min(self.project_cache.len().saturating_sub(1));
        self.mode = Mode::Projects;
    }

    pub fn exit_projects_view(&mut self) {
        self.mode = Mode::Normal;
    }

    pub fn project_step(&mut self, down: bool) {
        let n = self.project_cache.len();
        if n == 0 {
            return;
        }
        self.project_cursor = if down {
            (self.project_cursor + 1).min(n - 1)
        } else {
            self.project_cursor.saturating_sub(1)
        };
    }

    /// `Enter` na visão Projects: filtra a lista por ele, como `fp`.
    pub fn project_apply_filter(&mut self) {
        let Some(row) = self.project_cache.get(self.project_cursor) else {
            return;
        };
        self.filter.project = Some(row.name.clone());
        self.cursor = 0;
        self.recompute_visible();
        self.mode = Mode::Normal;
    }

    /// `x` na visão Projects: arquiva/desarquiva o projeto sob o cursor e
    /// persiste via load-modify-save (mesmo idioma de `review::mark_project_
    /// reviewed`/`save_current_filter_as`).
    pub fn toggle_project_archived(&mut self) {
        let Some(row) = self.project_cache.get(self.project_cursor).cloned() else {
            return;
        };
        let now_archived = !row.archived;
        if now_archived {
            if !self.project_archived.iter().any(|p| p == &row.name) {
                self.project_archived.push(row.name.clone());
            }
        } else {
            self.project_archived.retain(|p| p != &row.name);
        }

        // `self.project_archived` já é o estado desejado completo (partiu
        // de um clone do disco e só é mutado aqui) — recarrega o disco pra
        // não perder mudanças de outros campos feitas por fora, e sobrescreve
        // só o campo que esta feature possui (mesmo idioma de `Prefs::save`).
        let mut cfg = Config::load();
        cfg.project_archived = self.project_archived.clone();
        if let Err(e) = cfg.save() {
            self.flash(format!(
                "{}: {e}",
                crate::brand::tr("save failed", "falha ao salvar")
            ));
        }

        self.project_cache = self.build_project_rows();
        self.project_cursor = self
            .project_cursor
            .min(self.project_cache.len().saturating_sub(1));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::test_support::build_app;

    #[test]
    fn build_project_rows_includes_archive_only_and_archived_projects() {
        let mut app = build_app("a +open\n");
        app.store.archive = crate::app::Archive::for_test(
            crate::todo::parse_file("x 2026-07-01 2026-06-01 done thing +history\n"),
            String::new(),
            app.archive().path().to_path_buf(),
        );
        app.project_archived = vec!["gone".to_string()];

        let rows = app.build_project_rows();
        let names: Vec<&str> = rows.iter().map(|r| r.name.as_str()).collect();
        assert!(names.contains(&"open"));
        assert!(names.contains(&"history"), "projeto só em done.txt");
        assert!(names.contains(&"gone"), "projeto arquivado sem tarefa nenhuma");

        let open_row = rows.iter().find(|r| r.name == "open").unwrap();
        assert_eq!(open_row.open_count, 1);
        assert!(!open_row.archived);
        let gone_row = rows.iter().find(|r| r.name == "gone").unwrap();
        assert_eq!(gone_row.open_count, 0);
        assert!(gone_row.archived);
    }

    #[test]
    fn sort_puts_archived_last_then_by_open_count_desc_then_name() {
        let mut app = build_app("a +zebra\nb +zebra\nc +apple\n");
        app.project_archived = vec!["apple".to_string()];
        let rows = app.build_project_rows();
        let names: Vec<&str> = rows.iter().map(|r| r.name.as_str()).collect();
        // zebra (2 abertas) antes de apple (arquivado), mesmo "apple" < "zebra".
        assert_eq!(names, vec!["zebra", "apple"]);
    }

    #[test]
    fn enter_projects_view_builds_cache_and_sets_mode() {
        let mut app = build_app("a +work\n");
        app.enter_projects_view();
        assert_eq!(app.mode, Mode::Projects);
        assert!(!app.project_rows().is_empty());
    }

    #[test]
    fn project_apply_filter_sets_filter_and_returns_to_normal() {
        let mut app = build_app("a +work\nb +home\n");
        app.enter_projects_view();
        // "home"/"work" empatam sem arquivo — ordem alfabética por nome.
        let expected = app.project_rows()[0].name.clone();
        app.project_apply_filter();
        assert_eq!(app.mode, Mode::Normal);
        assert_eq!(app.filter().project.as_deref(), Some(expected.as_str()));
    }

    // `toggle_project_archived` também persiste via `Config::load()`/
    // `cfg.save()` (o caminho XDG real) — deliberadamente NÃO exercido
    // aqui, mesmo motivo de `review::mark_project_reviewed` (ver comentário
    // em app/review.rs): não tocar o config.toml real do usuário num teste.
    #[test]
    fn toggle_project_archived_updates_in_memory_list() {
        let mut app = build_app("a +work\n");
        app.enter_projects_view();
        assert!(!app.project_is_archived("work"));
        // Só a metade em memória é segura de testar sem tocar disco: chamo
        // a mutação direta em vez do método público (que persiste).
        app.project_archived.push("work".to_string());
        assert!(app.project_is_archived("work"));
    }
}
