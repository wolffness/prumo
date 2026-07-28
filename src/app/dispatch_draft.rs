//! Draft de despacho (`D`): transforma as tarefas selecionadas (ou a tarefa
//! sob o cursor) num buffer markdown editável — reusando o note panel — e
//! despacha o conteúdo para um agente (claude/codex) via herdr, no diretório
//! do projeto das tarefas.

use std::path::PathBuf;

use crate::advisor::dispatch;
use crate::brand::tr;
use crate::core::outcome::EditOutcome;

use super::{App, Mode, NotePanel, View};

/// Agentes disponíveis no ciclo do `Tab`: (chave p/ `agent_argv`, rótulo).
/// `◇` = família claude, `◆` = codex — símbolo distingue, nunca só cor.
pub const AGENTS: &[(&str, &str)] = &[
    ("sonnet", "◇ claude (sonnet)"),
    ("opus", "◇ claude (opus)"),
    ("fable", "◇ claude (fable)"),
    ("codex", "◆ codex"),
];

/// Token `dispatch:<slug>` de uma tarefa despachada, se houver.
pub fn slug_from_raw(raw: &str) -> Option<&str> {
    raw.split_whitespace()
        .find_map(|t| t.strip_prefix("dispatch:"))
        .filter(|s| !s.is_empty())
}

/// Estado visível do agente de uma tarefa despachada. Símbolo + palavra na
/// UI — nunca só cor (daltonismo).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DispatchBadge {
    Working,
    Blocked,
    Idle,
    /// O agente não existe mais no herdr: terminou (ou foi encerrado).
    Done,
}

/// Estado do draft de despacho enquanto o note panel o edita. `Some` muda o
/// footer do panel e habilita `Tab` (agente) e `Ctrl-D` (despachar).
pub struct DispatchCtx {
    pub agent: usize,
    pub slug: String,
    /// Diretório resolvido do projeto, ou a mensagem do porquê não resolveu.
    pub dir: Result<PathBuf, String>,
    /// Índices absolutos (em `tasks`) das tarefas do draft.
    pub tasks: Vec<usize>,
    /// Mais de um `+projeto` entre as tarefas: bloqueia o despacho (o draft
    /// exige um diretório só).
    pub mixed: bool,
}

impl DispatchCtx {
    /// Linha de status do footer: agente + dir (ou o erro de resolução).
    pub fn status_line(&self) -> String {
        let agent = AGENTS[self.agent].1;
        if self.mixed {
            return format!(
                "{agent} · ⚠ {}",
                tr(
                    "tasks span multiple +projects — dispatch needs one dir",
                    "tarefas de vários +projetos — despacho exige 1 diretório"
                )
            );
        }
        match &self.dir {
            Ok(dir) => {
                let home = std::env::var("HOME").unwrap_or_default();
                let shown = dir.to_string_lossy().replacen(&home, "~", 1);
                format!("{agent} · dir: {shown}")
            }
            Err(e) => format!("{agent} · ⚠ {e}"),
        }
    }
}

impl App {
    /// Abre o draft de despacho para a seleção (Visual) ou a tarefa do cursor.
    pub fn open_dispatch_draft(&mut self) {
        if self.view() != View::List {
            self.flash(tr("dispatch: list view only", "despacho: só na lista"));
            return;
        }
        let mut abs: Vec<usize> = if self.selection.is_empty() {
            self.cur_task_index_in_tasks().into_iter().collect()
        } else {
            self.selection.iter().collect()
        };
        abs.sort_unstable();
        let tasks: Vec<crate::todo::Task> = abs
            .iter()
            .filter_map(|&i| self.tasks().get(i).cloned())
            .collect();
        if tasks.is_empty() {
            self.flash(tr("no task to dispatch", "nenhuma tarefa para despachar"));
            return;
        }

        let mut projects: Vec<&str> = tasks
            .iter()
            .filter_map(|t| t.projects.first().map(String::as_str))
            .collect();
        projects.dedup();
        projects.sort_unstable();
        projects.dedup();
        let mixed = projects.len() > 1;
        let dir = match projects.first() {
            Some(project) => {
                let repo = self
                    .advisor_links
                    .iter()
                    .find(|(p, _)| p.eq_ignore_ascii_case(project))
                    .map(|(_, r)| r.clone())
                    .unwrap_or_else(|| (*project).to_string());
                dispatch::repo_dir(&repo).map_err(|e| e.to_string())
            }
            None => Err(tr("task has no +project", "tarefa sem +projeto").to_string()),
        };

        let first_title = crate::todo::body_only(&tasks[0].raw);
        let slug = format!(
            "{}-{}",
            self.today(),
            crate::note::slugify(&first_title)
        );
        let path = self
            .notes_dir()
            .join(format!("projects/prumo-dispatch/{slug}.md"));
        if !path.exists() {
            if let Some(parent) = path.parent()
                && let Err(e) = std::fs::create_dir_all(parent)
            {
                self.flash(format!("draft mkdir failed: {e}"));
                return;
            }
            let mut body = String::from(tr("## Tasks\n\n", "## Tarefas\n\n"));
            for t in &tasks {
                body.push_str(&format!("- {}\n", t.raw));
            }
            body.push_str(tr("\n## Instructions\n\n", "\n## Instruções\n\n"));
            if let Err(e) = std::fs::write(&path, body) {
                self.flash(format!("draft write failed: {e}"));
                return;
            }
        }

        let title = format!(
            "{} · {} {}",
            tr("dispatch draft", "draft de despacho"),
            tasks.len(),
            tr("task(s)", "tarefa(s)")
        );
        match NotePanel::load(path, title) {
            Ok(mut panel) => {
                panel.insert = true;
                panel.move_bottom();
                panel.line_end();
                self.note_panel = Some(panel);
                self.dispatch_ctx = Some(DispatchCtx {
                    agent: 0,
                    slug,
                    dir,
                    tasks: abs,
                    mixed,
                });
                self.mode = Mode::Note;
            }
            Err(e) => self.flash(format!("draft read failed: {e}")),
        }
    }

    /// `Tab` no draft: próximo agente do ciclo.
    pub fn dispatch_cycle_agent(&mut self) {
        if let Some(ctx) = self.dispatch_ctx.as_mut() {
            ctx.agent = (ctx.agent + 1) % AGENTS.len();
        }
    }

    /// `Ctrl-D` no draft: salva o buffer e dispara o agente via herdr no
    /// diretório do projeto. Em erro o draft fica aberto (nada se perde).
    pub fn dispatch_send(&mut self) {
        let Some(ctx) = self.dispatch_ctx.as_ref() else {
            return;
        };
        if ctx.mixed {
            self.flash(tr(
                "⚠ multiple +projects — split the dispatch",
                "⚠ vários +projetos — separe o despacho",
            ));
            return;
        }
        let dir = match &ctx.dir {
            Ok(d) => d.clone(),
            Err(e) => {
                self.flash(format!("⚠ {e}"));
                return;
            }
        };
        let Some(panel) = self.note_panel.as_mut() else {
            return;
        };
        if let Err(e) = panel.save() {
            self.flash(format!("draft save failed: {e}"));
            return;
        }
        let prompt = panel.lines.join("\n");
        let slug = ctx.slug.clone();
        let agent = AGENTS[ctx.agent].0;
        if dispatch::is_dispatched(&slug) {
            self.flash(format!(
                "dispatch-{slug} {}",
                tr("already running", "já em execução")
            ));
            return;
        }
        match dispatch::dispatch(&slug, agent, &dir, &prompt) {
            Ok(()) => {
                let tasks = ctx.tasks.clone();
                self.tag_dispatched(&tasks, &slug);
                self.note_panel = None;
                self.dispatch_ctx = None;
                self.selection.clear();
                self.mode = Mode::Normal;
                self.flash(format!(
                    "▶ dispatch-{slug} {}",
                    tr("dispatched", "despachado")
                ));
            }
            Err(e) => self.flash(format!(
                "{}: {e}",
                tr("dispatch failed", "falha no despacho")
            )),
        }
    }

    /// Badge do agente da tarefa, se ela carrega um token `dispatch:`.
    pub fn dispatch_badge_for(&self, task: &crate::todo::Task) -> Option<DispatchBadge> {
        self.dispatch_status
            .get(slug_from_raw(&task.raw)?)
            .copied()
    }

    /// Poll do estado dos agentes despachados (badges da lista). Retorna true
    /// quando algo mudou (o chamador redesenha). Throttle interno de 10s;
    /// sem tokens `dispatch:` nas tarefas o custo é zero.
    // ponytail: `herdr agent get` síncrono por slug no loop de UI — trocar
    // por um poll em thread se a lista de despachos crescer.
    pub fn poll_dispatch_status(&mut self) -> bool {
        let slugs: Vec<String> = {
            let mut s: Vec<String> = self
                .tasks()
                .iter()
                .filter_map(|t| slug_from_raw(&t.raw).map(str::to_string))
                .collect();
            s.sort_unstable();
            s.dedup();
            s
        };
        if slugs.is_empty() {
            let had = !self.dispatch_status.is_empty();
            self.dispatch_status.clear();
            return had;
        }
        let now = std::time::Instant::now();
        if self
            .dispatch_poll_at
            .is_some_and(|t| now.duration_since(t).as_secs() < 10)
        {
            return false;
        }
        self.dispatch_poll_at = Some(now);
        // herdr fora do ar não vira "✔ done" falso: sem `agent list`, os
        // badges anteriores ficam como estão.
        if dispatch::herdr(&["agent", "list"]).is_err() {
            return false;
        }
        let mut new = std::collections::HashMap::new();
        for slug in slugs {
            let badge = match dispatch::agent_status(&slug).as_deref() {
                Some("working") => DispatchBadge::Working,
                Some("blocked") => DispatchBadge::Blocked,
                Some(_) => DispatchBadge::Idle,
                None => DispatchBadge::Done,
            };
            new.insert(slug, badge);
        }
        let changed = new != self.dispatch_status;
        self.dispatch_status = new;
        changed
    }

    /// Grava o token `dispatch:<slug>` nas tarefas do draft (persistente no
    /// todo.txt — os badges sobrevivem a restart).
    fn tag_dispatched(&mut self, tasks: &[usize], slug: &str) {
        for &abs in tasks {
            match self.store.append_at(abs, &format!("dispatch:{slug}")) {
                EditOutcome::Saved { abs } => self.after_mutation(abs),
                EditOutcome::Aborted(r) => {
                    self.handle_reconcile_abort(r);
                    return;
                }
                _ => {}
            }
        }
    }
}
