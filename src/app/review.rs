//! Visão Review (`R`): revisão periódica de `+projeto`s, inspirada na Review
//! do OmniFocus, adaptada à realidade do Prumo — projeto é só uma tag em
//! tarefas soltas do todo.txt, não uma entidade própria. Revisar um projeto
//! é entrar na `View::List` já filtrada por ele (reusando toda a UI e as
//! ações normais de tarefa) e, ao sair, marcar a data em `config.toml`.

use chrono::NaiveDate;

use crate::config::Config;
use crate::core::filter::ordered_unique;

use super::{App, Mode, View};

/// Cadência default quando o usuário não configurou `review_every_days`.
pub const DEFAULT_REVIEW_EVERY_DAYS: u32 = 14;

/// Uma linha da lista de projetos da Review.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReviewRow {
    pub project: String,
    pub open_count: usize,
    /// Dias desde a última revisão; `None` = nunca revisado.
    pub days_since: Option<i64>,
}

/// Severidade visual da linha — o símbolo carrega o estado (daltonismo:
/// nunca só cor), a cor é reforço.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReviewStatus {
    /// Nunca revisado, ou o dobro (ou mais) da cadência sem revisar.
    Overdue,
    /// Já passou da cadência, mas não tanto quanto `Overdue`.
    Due,
    /// Dentro da cadência.
    Fresh,
}

impl ReviewRow {
    pub fn status(&self, every_days: u32) -> ReviewStatus {
        match self.days_since {
            None => ReviewStatus::Overdue,
            Some(d) if d >= i64::from(every_days) * 2 => ReviewStatus::Overdue,
            Some(d) if d >= i64::from(every_days) => ReviewStatus::Due,
            _ => ReviewStatus::Fresh,
        }
    }
}

impl App {
    /// Cadência efetiva (configurada, ou o default).
    pub fn review_every_days(&self) -> u32 {
        self.review_every_days.unwrap_or(DEFAULT_REVIEW_EVERY_DAYS)
    }

    /// Dias desde a última revisão do projeto. `None` se nunca revisado, ou
    /// se a data salva não é parseável (trata como nunca revisado — não é
    /// motivo pra travar a Review).
    pub fn days_since_review(&self, project: &str) -> Option<i64> {
        let last = self
            .review_last
            .iter()
            .find(|(p, _)| p == project)
            .map(|(_, d)| d.as_str())?;
        let last = NaiveDate::parse_from_str(last, "%Y-%m-%d").ok()?;
        let today = NaiveDate::parse_from_str(self.today(), "%Y-%m-%d").ok()?;
        Some((today - last).num_days())
    }

    /// Linhas da Review: um `+projeto` por tag usada em tarefas abertas,
    /// do mais atrasado (nunca revisado primeiro) ao mais em dia.
    fn build_review_rows(&self) -> Vec<ReviewRow> {
        let mut rows: Vec<ReviewRow> = ordered_unique(self.tasks(), |t| &t.projects)
            .into_iter()
            .map(|(project, open_count)| {
                let days_since = self.days_since_review(&project);
                ReviewRow {
                    project,
                    open_count,
                    days_since,
                }
            })
            .collect();
        rows.sort_by(|a, b| {
            b.days_since
                .unwrap_or(i64::MAX)
                .cmp(&a.days_since.unwrap_or(i64::MAX))
                .then_with(|| a.project.cmp(&b.project))
        });
        rows
    }

    /// Cache atual da lista de projetos da Review.
    pub fn review_rows(&self) -> &[ReviewRow] {
        &self.review_cache
    }

    pub fn review_cursor(&self) -> usize {
        self.review_cursor
    }

    /// Projeto sendo revisado no momento (`Some` enquanto a `View::List`
    /// está filtrada pela Review) — usado pelo footer/status bar.
    pub fn reviewing_project(&self) -> Option<&str> {
        self.reviewing_project.as_deref()
    }

    /// Entra na visão Review (`R`), (re)computando a lista de projetos.
    pub fn enter_review_view(&mut self) {
        self.review_cache = self.build_review_rows();
        self.review_cursor = self.review_cursor.min(self.review_cache.len().saturating_sub(1));
        self.set_view(View::Review);
        self.mode = Mode::Review;
    }

    /// Sai da Review de volta pra Lista, sem filtro nenhum pendurado.
    /// `set_view` já recomputa o cache ao trocar de `View::Review`.
    pub fn exit_review_view(&mut self) {
        self.filter.clear();
        self.set_view(View::List);
        self.mode = Mode::Normal;
    }

    pub fn review_step(&mut self, down: bool) {
        let n = self.review_cache.len();
        if n == 0 {
            return;
        }
        self.review_cursor = if down {
            (self.review_cursor + 1).min(n - 1)
        } else {
            self.review_cursor.saturating_sub(1)
        };
    }

    /// `Enter` num projeto da Review: filtra a `View::List` por ele e entra
    /// no "modo revisão" — sem state machine dedicada, as ações normais de
    /// tarefa (completar, editar, despachar) continuam funcionando.
    pub fn review_enter_project(&mut self) {
        let Some(row) = self.review_cache.get(self.review_cursor) else {
            return;
        };
        let project = row.project.clone();
        self.filter.clear();
        self.filter.project = Some(project.clone());
        self.reviewing_project = Some(project);
        self.set_view(View::List);
        self.mode = Mode::Normal;
    }

    /// `Esc` durante a revisão de um projeto: pergunta se marca como
    /// revisado (`Mode::ConfirmReview`) em vez do `Esc` normal da lista.
    pub fn review_request_finish(&mut self) {
        if self.reviewing_project.is_some() {
            self.mode = Mode::ConfirmReview;
        }
    }

    /// Confirmação aceita: grava a data de hoje e volta pra lista de
    /// projetos da Review. `enter_review_view` já reconstrói o cache.
    pub fn confirm_review_mark(&mut self) {
        if let Some(project) = self.reviewing_project.take() {
            self.mark_project_reviewed(&project);
        }
        self.filter.clear();
        self.enter_review_view();
    }

    /// Confirmação recusada: mantém a data anterior, mesma volta.
    pub fn confirm_review_skip(&mut self) {
        self.reviewing_project = None;
        self.filter.clear();
        self.enter_review_view();
    }

    /// Atualiza (in-memory) `review_last.<project>` para hoje. Separado de
    /// `mark_project_reviewed` para ficar testável sem tocar o config.toml
    /// real do usuário — só a parte pura.
    fn upsert_review_last_today(&mut self, project: &str) -> String {
        let today = self.today().to_string();
        match self.review_last.iter_mut().find(|(p, _)| p == project) {
            Some((_, d)) => *d = today.clone(),
            None => self.review_last.push((project.to_string(), today.clone())),
        }
        today
    }

    /// Grava `review_last.<project> = hoje` em memória e persiste via
    /// load-modify-save (mesmo idioma de `save_current_filter_as`):
    /// recarrega o disco pra não perder mudanças externas, funde só a
    /// entrada deste projeto.
    fn mark_project_reviewed(&mut self, project: &str) {
        self.upsert_review_last_today(project);
        let mut cfg = Config::load();
        cfg.review_last = merge_review_last(&cfg.review_last, &self.review_last);
        if let Err(e) = cfg.save() {
            self.flash(format!(
                "{}: {e}",
                crate::brand::tr("review save failed", "falha ao salvar a revisão")
            ));
        } else {
            self.flash(format!(
                "✔ {project} {}",
                crate::brand::tr("reviewed", "revisado")
            ));
        }
    }
}

/// Sobrepõe a entrada em memória (uma só, a que acabou de ser revisada) no
/// que já está em disco — preserva outras linhas gravadas externamente.
fn merge_review_last(disk: &[(String, String)], mem: &[(String, String)]) -> Vec<(String, String)> {
    let mut out: Vec<(String, String)> = disk.to_vec();
    for (name, date) in mem {
        match out.iter_mut().find(|(n, _)| n == name) {
            Some((_, d)) => *d = date.clone(),
            None => out.push((name.clone(), date.clone())),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::test_support::build_app;

    #[test]
    fn days_since_review_none_when_never_reviewed() {
        let app = build_app("a +proj\n");
        assert_eq!(app.days_since_review("proj"), None);
    }

    #[test]
    fn review_rows_orders_never_reviewed_first_then_most_stale() {
        let mut app = build_app("a +fresh\nb +stale\nc +never\n");
        app.review_last = vec![
            ("fresh".into(), app.today().to_string()),
            ("stale".into(), "2026-01-01".into()),
        ];
        app.enter_review_view();
        let names: Vec<&str> = app.review_rows().iter().map(|r| r.project.as_str()).collect();
        assert_eq!(names, vec!["never", "stale", "fresh"]);
    }

    #[test]
    fn review_status_bands_by_cadence() {
        let mut app = build_app("a +p\n");
        app.review_every_days = Some(10);
        let row = |days: Option<i64>| ReviewRow {
            project: "p".into(),
            open_count: 1,
            days_since: days,
        };
        assert_eq!(row(None).status(app.review_every_days()), ReviewStatus::Overdue);
        assert_eq!(row(Some(25)).status(app.review_every_days()), ReviewStatus::Overdue);
        assert_eq!(row(Some(12)).status(app.review_every_days()), ReviewStatus::Due);
        assert_eq!(row(Some(3)).status(app.review_every_days()), ReviewStatus::Fresh);
    }

    #[test]
    fn enter_project_filters_list_and_sets_reviewing() {
        // "home" sorts before "work" alphabetically once both tie as
        // "never reviewed" — cursor 0 is whichever comes first, not a fixed
        // name, so read it off the row instead of assuming "work".
        let mut app = build_app("a +work\nb +home\n");
        app.enter_review_view();
        let expected = app.review_rows()[0].project.clone();
        app.review_enter_project();
        assert_eq!(app.view(), View::List);
        assert_eq!(app.filter().project.as_deref(), Some(expected.as_str()));
        assert_eq!(app.reviewing_project(), Some(expected.as_str()));
        assert_eq!(app.visible_indices().len(), 1);
    }

    #[test]
    fn esc_while_reviewing_asks_confirmation_instead_of_clearing_filter() {
        let mut app = build_app("a +work\n");
        app.enter_review_view();
        app.review_enter_project();
        app.review_request_finish();
        assert_eq!(app.mode, Mode::ConfirmReview);
    }

    // `confirm_review_mark` also persists via `Config::load()`/`cfg.save()`
    // (the real XDG path) — deliberately NOT exercised here, matching the
    // codebase's existing convention of not unit-testing disk-touching App
    // methods (see `save_current_filter_as` in app/saved.rs, which has the
    // same shape and is untested at that level for the same reason). The
    // pure in-memory half is what `upsert_review_last_today` covers below.
    #[test]
    fn upsert_review_last_today_updates_in_memory_and_days_since() {
        let mut app = build_app("a +work\n");
        assert_eq!(app.days_since_review("work"), None);
        let today = app.upsert_review_last_today("work");
        assert_eq!(today, app.today());
        assert_eq!(app.days_since_review("work"), Some(0));
        // Re-marking updates in place instead of duplicating the entry.
        app.upsert_review_last_today("work");
        assert_eq!(app.review_last.iter().filter(|(p, _)| p == "work").count(), 1);
    }

    #[test]
    fn confirm_skip_does_not_persist() {
        let mut app = build_app("a +work\n");
        app.enter_review_view();
        app.review_enter_project();
        app.confirm_review_skip();
        assert_eq!(app.days_since_review("work"), None);
        assert!(app.reviewing_project().is_none());
    }

    #[test]
    fn merges_disk_and_memory_review_last() {
        let disk = vec![
            ("work".to_string(), "2026-01-01".to_string()),
            ("other".to_string(), "2026-02-02".to_string()),
        ];
        let mem = vec![("work".to_string(), "2026-07-29".to_string())];
        assert_eq!(
            merge_review_last(&disk, &mem),
            vec![
                ("work".to_string(), "2026-07-29".to_string()),
                ("other".to_string(), "2026-02-02".to_string()),
            ]
        );
    }
}
