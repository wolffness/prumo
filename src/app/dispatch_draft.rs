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
    /// `claude -p` do brief-builder rodando em background (Ctrl-P).
    pub busy: bool,
}

impl DispatchCtx {
    /// Linha de status do footer: agente + dir (ou o erro de resolução).
    pub fn status_line(&self) -> String {
        let agent = AGENTS[self.agent].1;
        if self.busy {
            return format!(
                "{agent} · ⧗ {}",
                tr("improving prompt…", "melhorando prompt…")
            );
        }
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

/// Troca o conteúdo da seção `## Instruções`/`## Instructions` pelo brief.
/// Sem a seção, o brief é anexado ao fim sob a seção nova.
fn replace_instructions(lines: &mut Vec<String>, brief: &str) {
    let header = |l: &str| {
        let t = l.trim();
        t == "## Instruções" || t == "## Instructions"
    };
    match lines.iter().position(|l| header(l)) {
        Some(i) => lines.truncate(i + 1),
        None => {
            if lines.last().is_some_and(|l| !l.trim().is_empty()) {
                lines.push(String::new());
            }
            lines.push(tr("## Instructions", "## Instruções").to_string());
        }
    }
    lines.push(String::new());
    lines.extend(brief.lines().map(str::to_string));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slug_from_raw_finds_dispatch_token() {
        assert_eq!(
            slug_from_raw("(A) fix parser +prumo dispatch:2026-07-28-fix"),
            Some("2026-07-28-fix")
        );
        assert_eq!(slug_from_raw("(A) fix parser +prumo"), None);
        assert_eq!(slug_from_raw("dispatch:"), None);
    }

    #[test]
    fn replace_instructions_swaps_section_content() {
        let mut lines: Vec<String> = ["## Tarefas", "- x", "", "## Instruções", "", "ideia crua"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        replace_instructions(&mut lines, "brief\nfinal");
        assert_eq!(
            lines,
            vec!["## Tarefas", "- x", "", "## Instruções", "", "brief", "final"]
        );
    }

    #[test]
    fn open_dispatch_draft_creates_template_and_ctx() {
        let mut app = crate::app::test_support::build_app("(A) Corrigir parser +prumo\n");
        // notes_dir aponta para o tmp do app para o teste não escrever em ~.
        let dir = app.file_path.parent().unwrap().to_path_buf();
        app.notes_dir = dir.clone();
        app.open_dispatch_draft();
        let ctx = app.dispatch_ctx.as_ref().expect("ctx aberto");
        assert_eq!(ctx.tasks, vec![0]);
        assert!(!ctx.mixed);
        assert_eq!(ctx.slug, "2026-05-06-corrigir-parser");
        let panel = app.note_panel.as_ref().expect("panel aberto");
        let body = panel.lines.join("\n");
        assert!(body.contains("(A) Corrigir parser +prumo"), "{body}");
        assert!(body.contains("## Instru"), "{body}");
        assert!(panel.path.starts_with(dir.join("projects/prumo-dispatch")));
    }

    #[test]
    fn replace_instructions_appends_section_when_missing() {
        let mut lines: Vec<String> = vec!["só texto".to_string()];
        replace_instructions(&mut lines, "brief");
        assert_eq!(lines.last().unwrap(), "brief");
        assert!(lines.iter().any(|l| l.starts_with("## Instru")));
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
                    busy: false,
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
            // Conclusão assistida (4b): transição para Done (o agente sumiu
            // do herdr) oferece completar as tarefas do despacho. Só em
            // transição observada — na primeira população pós-restart nada
            // dispara, e um "não" fica registrado em `dispatch_done_seen`.
            let was = self.dispatch_status.get(&slug);
            if badge == DispatchBadge::Done
                && was.is_some_and(|&b| b != DispatchBadge::Done)
                && !self.dispatch_done_seen.contains(&slug)
                && self.mode == Mode::Normal
                && self.dispatch_done_pending.is_none()
            {
                self.dispatch_done_pending = Some(slug.clone());
                self.mode = Mode::ConfirmDispatch;
            }
            new.insert(slug, badge);
        }
        let changed = new != self.dispatch_status;
        self.dispatch_status = new;
        changed
    }

    /// Índices (em `tasks`) das tarefas com o token `dispatch:<slug>`.
    fn tasks_with_slug(&self, slug: &str) -> Vec<usize> {
        self.tasks()
            .iter()
            .enumerate()
            .filter(|(_, t)| slug_from_raw(&t.raw) == Some(slug))
            .map(|(i, _)| i)
            .collect()
    }

    /// Quantas tarefas o confirm de conclusão cobriria (render do overlay).
    pub fn dispatch_done_task_count(&self) -> usize {
        self.dispatch_done_pending
            .as_deref()
            .map(|s| self.tasks_with_slug(s).len())
            .unwrap_or(0)
    }

    /// Confirmação aceita: completa as tarefas abertas do despacho concluído.
    pub fn confirm_dispatch_done(&mut self) {
        let Some(slug) = self.dispatch_done_pending.take() else {
            self.mode = Mode::Normal;
            return;
        };
        for abs in self.tasks_with_slug(&slug) {
            if self.tasks().get(abs).is_some_and(|t| !t.done) {
                self.toggle_complete(abs);
            }
        }
        self.dispatch_done_seen.insert(slug);
        self.mode = Mode::Normal;
    }

    /// Confirmação recusada: não pergunta de novo para este slug.
    pub fn dismiss_dispatch_done(&mut self) {
        if let Some(slug) = self.dispatch_done_pending.take() {
            self.dispatch_done_seen.insert(slug);
        }
        self.mode = Mode::Normal;
    }

    /// `Ctrl-P` no draft: roda o brief-builder (`claude -p`) em background no
    /// diretório do projeto e troca `## Instruções` pelo brief resultante.
    pub fn dispatch_improve_prompt(&mut self) {
        let (busy, dir) = match self.dispatch_ctx.as_ref() {
            Some(c) => (c.busy, c.dir.clone()),
            None => return,
        };
        if busy {
            return;
        }
        let dir = match dir {
            Ok(d) => d,
            Err(e) => {
                self.flash(format!("⚠ {e}"));
                return;
            }
        };
        let brief_path = std::path::PathBuf::from(std::env::var("HOME").unwrap_or_default())
            .join("Documents/Projetos/brain/prompts/brief-builder.md");
        let brief = match std::fs::read_to_string(&brief_path) {
            Ok(b) => b,
            Err(e) => {
                let msg = format!("brief-builder: {e} ({})", brief_path.display());
                self.flash(msg);
                return;
            }
        };
        let Some(panel) = self.note_panel.as_ref() else {
            return;
        };
        let draft = panel.lines.join("\n");
        let prompt = format!(
            "{brief}\n\n---\n\n{}\n\n{draft}",
            tr(
                "Apply the process above to the request below (a dispatch draft: tasks + instructions). Reply with ONLY the final brief in markdown, no interview — put open questions in a final section.",
                "Aplique o processo acima ao pedido abaixo (um draft de despacho: tarefas + instruções). Responda SOMENTE com o brief final em markdown, sem entrevista — perguntas em aberto vão numa seção final."
            )
        );
        let (tx, rx) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            // Sem a env var o CLI usa o login OAuth da assinatura, não a API.
            let out = std::process::Command::new("claude")
                .args(["-p", &prompt])
                .current_dir(&dir)
                .env_remove("ANTHROPIC_API_KEY")
                .output();
            let result = match out {
                Ok(o) if o.status.success() => {
                    Ok(String::from_utf8_lossy(&o.stdout).trim().to_string())
                }
                Ok(o) => Err(String::from_utf8_lossy(&o.stderr).trim().to_string()),
                Err(e) => Err(format!("claude: {e}")),
            };
            let _ = tx.send(result);
        });
        self.prompt_improver = Some(rx);
        if let Some(ctx) = self.dispatch_ctx.as_mut() {
            ctx.busy = true;
        }
    }

    /// Drena o brief-builder em background. `true` quando algo mudou (redraw).
    pub fn poll_prompt_improver(&mut self) -> bool {
        use std::sync::mpsc::TryRecvError;
        let Some(rx) = &self.prompt_improver else {
            return false;
        };
        let result = match rx.try_recv() {
            Ok(r) => r,
            Err(TryRecvError::Empty) => return false,
            Err(TryRecvError::Disconnected) => {
                self.prompt_improver = None;
                if let Some(ctx) = self.dispatch_ctx.as_mut() {
                    ctx.busy = false;
                }
                return true;
            }
        };
        self.prompt_improver = None;
        if let Some(ctx) = self.dispatch_ctx.as_mut() {
            ctx.busy = false;
        }
        match result {
            Ok(brief) => {
                if let Some(panel) = self.note_panel.as_mut() {
                    replace_instructions(&mut panel.lines, &brief);
                    panel.row = panel.lines.len() - 1;
                    panel.col = 0;
                    panel.dirty = true;
                    self.flash(tr("prompt improved", "prompt melhorado"));
                }
            }
            Err(e) => self.flash(format!("brief-builder: {e}")),
        }
        true
    }

    /// Grava o token `dispatch:<slug>` nas tarefas do draft (persistente no
    /// todo.txt — os badges sobrevivem a restart).
    fn tag_dispatched(&mut self, tasks: &[usize], slug: &str) {
        // Um redespacho do mesmo slug volta a poder oferecer conclusão.
        self.dispatch_done_seen.remove(slug);
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
