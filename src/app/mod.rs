use core::fmt;
use std::cell::Cell;
use std::path::PathBuf;
use std::str::FromStr;
use std::sync::mpsc::{Receiver, TryRecvError};

use crate::config::Config;
use crate::core::Store;
use crate::core::outcome::{DrainReport, Reconcile};
use crate::note;
use crate::serve::{self, ShareInfo};
use crate::theme::{self, Theme};
use crate::todo::Task;

mod autocomplete;
mod bulk;
mod chord;
mod draft;
mod draft_overlay;
mod flash;
mod mutations;
pub mod note_panel;
pub mod palette;
mod picker;
mod prefs;
mod saved;
mod selection;
mod types;
mod visibility;

#[cfg(test)]
pub(crate) mod test_support;

pub use crate::core::Archive;
pub use crate::core::History;
pub use crate::core::filter::{ListDueBucket, ordered_unique};
pub use autocomplete::{ActiveToken, AutocompleteTarget, TokenKind, active_token};
pub use chord::Chord;
pub use draft::{DialogInputMode, DraftCursor, DraftState};
pub use draft_overlay::{
    BuilderField, CalendarState, CalendarTarget, DraftOverlay, OverlayKind, PriorityChooserState,
    REC_UNIT_ORDER, RecurrenceBuilderState, SLASH_ENTRIES, SlashEntry, SlashKind, SlashMenuState,
    format_rec_value, recurrence_next_preview,
};
pub use flash::Flash;
pub use note_panel::NotePanel;
pub use palette::CommandPaletteState;
pub use prefs::{Layout, Prefs};
pub use selection::Selection;
pub use types::{
    AUTOCOMPLETE_CAP, AddOutcome, Density, FLASH_TTL, Filter, LEADER_WINDOW, Mode, SavedFilter,
    Sort, UNDO_LIMIT, View,
};
pub use visibility::GroupKey;

/// What a registered mouse-click region does when hit (see
/// `App::click_targets`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ClickAction {
    /// Open a file with the system opener (attachment rows).
    Open(PathBuf),
    /// Toggle the checkbox on a note-panel buffer line.
    TogglePanelRow(usize),
    /// Toggle the checkbox on line `line` of the note file at `path`
    /// (DETAIL-pane clicks, where no panel buffer is open).
    ToggleNoteLine { path: PathBuf, line: usize },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WeekStart {
    Sunday,
    Monday,
}

impl WeekStart {
    pub fn as_str(self) -> &'static str {
        match self {
            WeekStart::Sunday => "sunday",
            WeekStart::Monday => "monday",
        }
    }
}

impl fmt::Display for WeekStart {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for WeekStart {
    type Err = ();
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "sunday" => Ok(WeekStart::Sunday),
            "monday" => Ok(WeekStart::Monday),
            _ => Err(()),
        }
    }
}

pub struct App {
    /// The headless durable store: tasks, archive, history, persistence, and
    /// `today`. Mutate via the methods on `App` (which map store outcomes to
    /// flash messages and refresh the visible cache); read via `tasks()`,
    /// `archive()`, `task_raw()`, etc.
    pub(crate) store: Store,
    /// Crate-private: writing here would not invalidate `visible_cache`.
    /// Read via `view()`; mutate via `set_view()`.
    pub(crate) view: View,
    pub mode: Mode,
    pub prefs: Prefs,
    pub cursor: usize,
    /// Per-view saved cursor, indexed by `View::idx()`. `set_view` snapshots
    /// the outgoing view's cursor here and restores the incoming view's, so
    /// each view remembers where the user last was.
    pub(crate) view_cursor: [usize; 3],
    /// Crate-private: same reason as `view` — `visible_cache` would drift.
    /// Read via `filter()`; mutate via `set_search`/`set_project`/etc.
    pub(crate) filter: Filter,
    pub draft: DraftState,
    pub selection: Selection,
    flash_state: Flash,
    pub chord: Chord,
    pub file_path: PathBuf,
    /// Resolved path of the on-disk config file. Set by the binary after
    /// construction so the settings overlay can render a stable, real path
    /// without the renderer having to reach into the environment itself.
    /// `None` in tests/examples that don't care about the value.
    pub config_path: Option<PathBuf>,
    pub should_quit: bool,
    visible_cache: Vec<usize>,
    /// Parallel to `visible_cache`: `visible_groups[i]` is the group key for
    /// the row at `visible_cache[i]`. `GroupKey::None` for List under
    /// `Sort::File`; priority/due bucket keys under other List sorts; date
    /// keys for Archive. Renderers read this to draw section headers.
    visible_groups: Vec<crate::app::visibility::GroupKey>,
    /// Latest known release tag, populated asynchronously by the update
    /// checker. `None` while we haven't heard back (or the check is disabled,
    /// e.g. in tests). The UI compares this against `CARGO_PKG_VERSION` to
    /// decide whether to surface an "update available" hint.
    pub(crate) latest_version: Option<String>,
    /// Receiver for the background update check. Drained each tick; cleared
    /// once a result has been received or the sender hung up.
    update_check: Option<Receiver<Option<String>>>,
    /// User-named saved searches, loaded from config at startup and
    /// upserted via `fs`. Recalled with the `ff` picker.
    pub saved_filters: Vec<SavedFilter>,
    /// Projetos com o advisor ligado, lido do config. Usado só para o
    /// indicador `✦` no painel PROJETOS; a alternância é via comando CLI.
    pub advisor_projects: Vec<String>,
    /// Vínculos projeto→repo, lido do config. Usado para descobrir o repo das
    /// issues do projeto em foco na visão Issues.
    pub advisor_links: Vec<(String, String)>,
    /// Objetivo salvo por projeto, lido do config (norte do ranking `p`).
    pub advisor_goals: Vec<(String, String)>,
    /// Cache da sessão das issues da visão Issues, e de qual repo/projeto vieram.
    pub(crate) issues: Vec<crate::advisor::github::IssueRow>,
    pub(crate) issues_repo: Option<String>,
    pub(crate) issues_project: Option<String>,
    /// Cursor próprio da visão Issues (as issues não são tarefas).
    pub(crate) issues_cursor: usize,
    /// The search string that was active when the `ff` picker opened, so
    /// cancelling (`Esc`) restores it instead of leaving the previewed
    /// filter applied. `None` outside `Mode::PickSavedFilter`.
    saved_pick_restore: Option<String>,
    /// Index into `saved_filters` of the row the `ff` picker currently
    /// previews. Tracked explicitly rather than re-derived from
    /// `filter.search` so duplicate queries don't strand j/k. Only
    /// meaningful while `Mode::PickSavedFilter`; re-seeded on each open.
    saved_pick_idx: usize,
    pub command_palette: CommandPaletteState,
    /// Vertical scroll offset (rows from the top of the line list) for each
    /// view, keyed by `View::idx()`. Updated at render time via `Cell` so the
    /// renderer can keep the cursor row visible without taking `&mut self`.
    pub(crate) view_scroll: [Cell<u16>; 3],
    /// Handle to the in-TUI capture server. `None` until the first time
    /// the user presses `s` (or invokes "show capture QR" from the
    /// palette). Once bound, the entry stays for the rest of the
    /// session and the overlay just re-displays the saved QR.
    share: Option<ShareInfo>,
    /// Base directory used by note actions. Relative `note:<path>` tokens are
    /// resolved under this directory, and generated notes are created below it.
    pub(crate) notes_dir: PathBuf,
    /// Path queued for opening in the user's editor after the TUI temporarily
    /// restores the terminal. Set by OpenNote and drained by the run loop.
    pending_editor_path: Option<PathBuf>,
    /// Shell command queued from the search field (`! <cmd>`), run after the
    /// TUI temporarily restores the terminal. Set by `handle_search` and
    /// drained by the run loop.
    pending_shell: Option<String>,
    /// In-TUI note editor. `Some` only while `Mode::Note` is active.
    pub note_panel: Option<NotePanel>,
    /// Display-text → link-target registry for the OSC 8 overlay. The
    /// hyperlink pass (`ui::hyperlinks`) can only see the rendered text of an
    /// underlined run, so renderers that want a link target different from
    /// the visible text (e.g. an attachment name pointing at a `file://`
    /// URI) register the mapping here during draw. Cleared at the start of
    /// every frame; `RefCell` because renderers only hold `&App`.
    pub(crate) link_targets: std::cell::RefCell<std::collections::HashMap<String, String>>,
    /// Clickable screen regions registered during draw. The run loop
    /// hit-tests mouse clicks against these and dispatches the action
    /// (open an attachment, toggle a subtask checkbox). Cleared alongside
    /// `link_targets` at the start of every frame.
    pub(crate) click_targets: std::cell::RefCell<Vec<(ratatui::layout::Rect, ClickAction)>>,
    /// Subtask-progress cache keyed by note path: `(mtime, (done, total))`.
    /// Renderers query per visible row every frame; the mtime check keeps
    /// this to one cheap metadata stat per row instead of a full read.
    #[allow(clippy::type_complexity)]
    pub(crate) subtask_cache: std::cell::RefCell<
        std::collections::HashMap<PathBuf, (std::time::SystemTime, Option<(usize, usize)>)>,
    >,
    /// Theme index captured when the theme picker opened, so cancel
    /// can restore it.
    theme_pick_orig: usize,
    pub week_start: WeekStart,
}

impl App {
    /// Construct an App whose archive is the sibling `done.txt` of `file_path`.
    pub fn new(file_path: PathBuf, body: String, today: String, cfg: Config) -> Self {
        let store = Store::new(file_path.clone(), body, today);
        Self::from_store(store, file_path, cfg)
    }

    /// Like [`App::new`] but with an explicit `done.txt` path (e.g. `DONE_FILE`).
    pub fn new_with_done(
        file_path: PathBuf,
        done_path: PathBuf,
        body: String,
        today: String,
        cfg: Config,
    ) -> Self {
        let store = Store::new_with_done(file_path.clone(), done_path, body, today);
        Self::from_store(store, file_path, cfg)
    }

    fn from_store(store: Store, file_path: PathBuf, cfg: Config) -> Self {
        // Read saved filters before `cfg` is moved into `Prefs::from_config`.
        let note_dir = note::notes_dir_from_config(cfg.notes_dir.as_deref());
        let saved_filters = cfg
            .filters
            .iter()
            .map(|(name, query)| SavedFilter {
                name: name.clone(),
                query: query.clone(),
            })
            .collect();
        let advisor_projects = cfg.advisor_projects.clone();
        let advisor_links = cfg.advisor_links.clone();
        let advisor_goals = cfg.advisor_goals.clone();
        let mut app = Self {
            store,
            view: View::List,
            mode: Mode::Normal,
            prefs: Prefs::from_config(cfg),
            cursor: 0,
            view_cursor: [0; 3],
            filter: Filter::default(),
            draft: DraftState::default(),
            selection: Selection::default(),
            flash_state: Flash::default(),
            chord: Chord::default(),
            file_path,
            config_path: None,
            should_quit: false,
            visible_cache: Vec::new(),
            visible_groups: Vec::new(),
            latest_version: None,
            update_check: None,
            saved_filters,
            advisor_projects,
            advisor_links,
            advisor_goals,
            issues: Vec::new(),
            issues_repo: None,
            issues_project: None,
            issues_cursor: 0,
            saved_pick_restore: None,
            saved_pick_idx: 0,
            command_palette: CommandPaletteState::default(),
            view_scroll: [Cell::new(0), Cell::new(0), Cell::new(0)],
            share: None,
            notes_dir: note_dir,
            pending_editor_path: None,
            pending_shell: None,
            note_panel: None,
            link_targets: std::cell::RefCell::new(std::collections::HashMap::new()),
            click_targets: std::cell::RefCell::new(Vec::new()),
            subtask_cache: std::cell::RefCell::new(std::collections::HashMap::new()),
            theme_pick_orig: 0,
            week_start: WeekStart::Sunday,
        };
        app.recompute_visible();
        app
    }

    /// Rebind the App to a different on-disk file at runtime, replacing the
    /// store (tasks, archive, history, external-change baseline) with a fresh
    /// one for `file_path`/`done_path` loaded from `body`. Prefs, saved
    /// filters, theme, and config live on `App` and are left intact. Used by
    /// the first-run welcome prompt to swap from the placeholder file to the
    /// chosen one. Resets the cursor and recomputes the visible cache.
    pub fn open_file(&mut self, file_path: PathBuf, done_path: PathBuf, body: String) {
        let today = self.store.today().to_string();
        self.store = Store::new_with_done(file_path.clone(), done_path, body, today);
        self.file_path = file_path;
        self.cursor = 0;
        self.recompute_visible();
    }

    /// Idempotent: bind the capture server on first call, then store
    /// the [`ShareInfo`] so subsequent calls just re-show the overlay.
    ///
    /// First-call behavior: if the user has a persisted token + port in
    /// config, reuse them so phone bookmarks survive across sessions.
    /// Otherwise, generate a fresh token, let the OS pick a port, and
    /// write both back to the config. If the persisted port is taken
    /// (another tuxedo instance on the same machine, say), fall back to
    /// an OS-assigned port and rewrite the config so the next session
    /// starts fresh.
    pub fn ensure_share_started(&mut self) -> Result<&ShareInfo, String> {
        // Two-step to dodge stable's lack of Polonius: do the bind work
        // first (which needs `&mut self`), then the unconditional final
        // borrow returns the now-present `ShareInfo`.
        if self.share.is_none() {
            let info = self.bind_share()?;
            self.share = Some(info);
        }
        Ok(self
            .share
            .as_ref()
            .expect("share is Some after the bind branch"))
    }

    fn bind_share(&mut self) -> Result<ShareInfo, String> {
        let cfg = Config::load();
        let token = match cfg.share_token {
            Some(t) => t,
            None => serve::net::generate_token().map_err(|e| format!("token: {e}"))?,
        };
        let requested_port = cfg.share_port.unwrap_or(0);
        let info = match serve::start(self.file_path.clone(), token.clone(), requested_port) {
            Ok(info) => info,
            Err(_) if requested_port != 0 => {
                // Configured port is taken — fall back to OS-assigned.
                serve::start(self.file_path.clone(), token.clone(), 0)
                    .map_err(|e| format!("bind: {e}"))?
            }
            Err(e) => return Err(format!("bind: {e}")),
        };
        // Persist token + port back to config so phone bookmarks survive.
        // Load fresh first so we don't clobber any prefs the user has
        // toggled since this App was constructed.
        let mut to_save = Config::load();
        to_save.share_token = Some(info.token.clone());
        to_save.share_port = Some(info.port);
        if let Err(e) = to_save.save() {
            self.flash(format!("share config save failed: {e}"));
        }
        Ok(info)
    }

    pub fn share_info(&self) -> Option<&ShareInfo> {
        self.share.as_ref()
    }

    /// Install the receiver from [`update::spawn_check`](crate::update::spawn_check).
    /// `main` calls this; tests leave it unset so the App stays inert and
    /// doesn't spawn network work as a side effect of construction.
    pub fn set_update_check(&mut self, rx: Receiver<Option<String>>) {
        self.update_check = Some(rx);
    }

    /// Drain the update-check receiver. Returns `true` if a new latest
    /// version was just received — callers use this to trigger a redraw so
    /// the status bar can paint the indicator on the next frame.
    pub fn poll_update_check(&mut self) -> bool {
        let Some(rx) = &self.update_check else {
            return false;
        };
        match rx.try_recv() {
            Ok(maybe_tag) => {
                self.latest_version = maybe_tag;
                self.update_check = None;
                self.latest_version.is_some()
            }
            Err(TryRecvError::Empty) => false,
            Err(TryRecvError::Disconnected) => {
                self.update_check = None;
                false
            }
        }
    }

    /// Returns the latest known release tag *if* it is strictly newer than
    /// the running binary. The status bar uses this to decide whether to
    /// draw an "update available" hint.
    pub fn update_available(&self) -> Option<&str> {
        let latest = self.latest_version.as_deref()?;
        if crate::update::is_newer_fork(latest, env!("CARGO_PKG_VERSION")) {
            Some(latest)
        } else {
            None
        }
    }

    pub fn theme(&self) -> &'static Theme {
        self.prefs.theme()
    }

    /// Mode the rest of the UI should react to. While the command palette is
    /// open, the underlying list/sidebars should keep rendering as if the
    /// user were still in the mode they came from — otherwise opening the
    /// palette mid-Visual hides the multi-select checkboxes and similar
    /// mode-driven affordances.
    pub fn effective_mode(&self) -> Mode {
        match self.mode {
            Mode::CommandPalette => self.command_palette.prior().unwrap_or(self.mode),
            m => m,
        }
    }

    pub fn sort_label(&self) -> &'static str {
        self.prefs.sort_label()
    }

    /// Persist preferences. On failure, flashes a short error so the user
    /// sees the problem inside the TUI (writing to stderr would smash the
    /// alt-screen).
    pub fn save_prefs(&mut self) {
        if let Err(e) = self.prefs.save() {
            self.flash(format!("config save failed: {e}"));
        }
    }

    pub fn cycle_theme(&mut self) {
        let msg = self.prefs.cycle_theme();
        self.flash(msg);
        self.save_prefs();
    }

    /// Enter theme picker mode. Snapshot the current theme index so
    /// cancel can restore it. j/k live-previews; Enter accepts; Esc
    /// reverts.
    pub fn enter_pick_theme(&mut self) {
        self.theme_pick_orig = self.prefs.theme_idx();
        self.mode = Mode::PickTheme;
    }

    /// Step through themes in `forward` (true = next) direction with
    /// wrap-around. The theme is applied immediately for live preview.
    pub fn pick_theme_step(&mut self, forward: bool) {
        let all = theme::all();
        let len = all.len();
        if len <= 1 {
            return;
        }
        let cur = self.prefs.theme_idx();
        let next = if forward {
            (cur + 1) % len
        } else {
            (cur + len - 1) % len
        };
        self.prefs.set_theme_idx(next);
    }

    /// Accept the previewed theme and persist to config.
    pub fn pick_theme_accept(&mut self) {
        self.mode = Mode::Normal;
        self.save_prefs();
        self.flash(format!("theme: {}", self.theme().name));
    }

    /// Cancel the picker and restore the theme that was active when
    /// the picker opened.
    pub fn pick_theme_cancel(&mut self) {
        self.prefs.set_theme_idx(self.theme_pick_orig);
        self.mode = Mode::Normal;
    }

    pub fn cycle_density(&mut self) {
        let msg = self.prefs.cycle_density();
        self.flash(msg);
        self.save_prefs();
    }

    pub fn cycle_sort(&mut self) {
        let msg = self.prefs.cycle_sort();
        self.flash(msg);
        self.recompute_visible();
        self.save_prefs();
    }

    /// Read-only view of the parsed task list. Mutations go through
    /// dedicated methods so history/persist/visible-cache stay coherent.
    pub fn tasks(&self) -> &[Task] {
        self.store.tasks()
    }

    /// Read-only view of the archived (`done.txt`) tasks.
    pub fn archive(&self) -> &Archive {
        self.store.archive()
    }

    /// The cached "today" (ISO `YYYY-MM-DD`) the store resolves dates against.
    pub fn today(&self) -> &str {
        self.store.today()
    }

    pub fn queue_editor_path(&mut self, path: PathBuf) {
        self.pending_editor_path = Some(path);
    }

    pub fn notes_dir(&self) -> &PathBuf {
        &self.notes_dir
    }

    pub fn take_pending_editor_path(&mut self) -> Option<PathBuf> {
        self.pending_editor_path.take()
    }

    /// Enfileira um comando de shell (do campo `! <cmd>`) para o loop principal
    /// rodar com o terminal restaurado.
    pub fn queue_shell(&mut self, cmd: String) {
        self.pending_shell = Some(cmd);
    }

    pub fn take_pending_shell(&mut self) -> Option<String> {
        self.pending_shell.take()
    }

    /// Reset the display-text → link-target registry. Called at the top of
    /// every `ui::draw` so stale mappings from previous frames can't leak.
    pub fn clear_link_targets(&self) {
        self.link_targets.borrow_mut().clear();
        self.click_targets.borrow_mut().clear();
    }

    /// Register a clickable screen region (see `click_targets`).
    pub fn register_click_target(&self, rect: ratatui::layout::Rect, action: ClickAction) {
        self.click_targets.borrow_mut().push((rect, action));
    }

    /// Subtask progress `(done, total)` for a task, from the checkboxes in
    /// its linked note. `None` when the task has no note, the note has no
    /// checkboxes, or the file is unreadable. Cached by note mtime.
    pub fn subtask_progress(&self, task: &Task) -> Option<(usize, usize)> {
        note::note_rel_from_raw(&task.raw)?;
        let target = note::target_for_task(task, &self.notes_dir);
        let mtime = std::fs::metadata(&target.path).ok()?.modified().ok()?;
        let mut cache = self.subtask_cache.borrow_mut();
        if let Some((cached_mtime, progress)) = cache.get(&target.path)
            && *cached_mtime == mtime
        {
            return *progress;
        }
        let progress = std::fs::read_to_string(&target.path)
            .ok()
            .and_then(|body| crate::subtasks::progress(&body));
        cache.insert(target.path, (mtime, progress));
        progress
    }

    /// Action registered under the screen cell `(x, y)` this frame, if any.
    pub fn click_target_at(&self, x: u16, y: u16) -> Option<ClickAction> {
        self.click_targets
            .borrow()
            .iter()
            .find(|(r, _)| x >= r.x && x < r.x + r.width && y >= r.y && y < r.y + r.height)
            .map(|(_, a)| a.clone())
    }

    /// Dispatch a mouse click. Returns true when something was hit (the
    /// caller redraws). `Open` targets spawn the system opener; toggle
    /// targets flip the checkbox in the panel buffer or the note file.
    pub fn handle_click(&mut self, x: u16, y: u16) -> bool {
        match self.click_target_at(x, y) {
            Some(ClickAction::Open(path)) => {
                match crate::attach::open_with_system(&path) {
                    Ok(()) => self.flash(crate::brand::tr("opened attachment", "anexo aberto")),
                    Err(e) => self.flash(format!("open failed: {e}")),
                }
                true
            }
            Some(ClickAction::TogglePanelRow(row)) => {
                if let Some(panel) = self.note_panel.as_mut() {
                    panel.toggle_checkbox_at(row);
                }
                true
            }
            Some(ClickAction::ToggleNoteLine { path, line }) => {
                self.toggle_note_file_line(&path, line);
                true
            }
            None => false,
        }
    }

    /// Flip the checkbox on 0-based `line` of the note at `path`, writing
    /// the file back. Used by DETAIL-pane clicks where no buffer is open.
    fn toggle_note_file_line(&mut self, path: &std::path::Path, line: usize) {
        let Ok(body) = std::fs::read_to_string(path) else {
            self.flash("note read failed");
            return;
        };
        let mut lines: Vec<String> = body.lines().map(str::to_string).collect();
        let Some(flipped) = lines
            .get(line)
            .and_then(|l| crate::subtasks::toggle_line(l))
        else {
            return;
        };
        lines[line] = flipped;
        let mut out = lines.join("\n");
        out.push('\n');
        if let Err(e) = std::fs::write(path, out) {
            self.flash(format!("note save failed: {e}"));
        }
    }

    /// Register a link target for a piece of underlined display text (see
    /// `link_targets`).
    pub fn register_link_target(&self, text: impl Into<String>, href: impl Into<String>) {
        self.link_targets
            .borrow_mut()
            .insert(text.into(), href.into());
    }

    /// Link target registered for `text` this frame, if any.
    pub fn link_target(&self, text: &str) -> Option<String> {
        self.link_targets.borrow().get(text).cloned()
    }

    /// True when at least one task is marked done. Used by the binary to
    /// decide whether `A` archives or just toggles the archive view.
    pub fn has_completed_tasks(&self) -> bool {
        self.store.has_completed()
    }

    /// Cloned `raw` for the task at `abs`, or `None` if out of range.
    /// Returning an owned `String` so the caller can hold it across `&mut self`
    /// calls (the common shape for "load draft from current task").
    pub fn task_raw(&self, abs: usize) -> Option<String> {
        self.store.task_raw(abs)
    }

    /// Task under the cursor, resolved against the active view's source:
    /// `archive.tasks()` in Archive view, `tasks` otherwise.
    pub fn cur_task(&self) -> Option<&Task> {
        let i = self.cur_abs()?;
        match self.view {
            View::Archive => self.store.archive().tasks().get(i),
            _ => self.store.tasks().get(i),
        }
    }

    /// Pump archive state (startup loader + external `done.txt` edits). Returns
    /// true when the visible archive changed, so the caller redraws. Refreshes
    /// the visible cache when the Archive view is active.
    pub fn poll_archive(&mut self) -> bool {
        let changed = self.store.poll_archive();
        if changed && matches!(self.view, View::Archive) {
            self.recompute_visible();
            self.clamp_cursor();
        }
        changed
    }

    /// Index of the task under the cursor *into `self.tasks`*. Returns `None`
    /// in Archive view because the cursor there points into `archive.tasks()`.
    /// Use this — not `cur_abs()` — for any write that mutates `self.tasks`.
    pub fn cur_task_index_in_tasks(&self) -> Option<usize> {
        if matches!(self.view, View::Archive) {
            return None;
        }
        self.cur_abs()
    }

    /// Read-only view of the active filter.
    pub fn filter(&self) -> &Filter {
        &self.filter
    }

    /// Active top-level view (List/Archive).
    pub fn view(&self) -> View {
        self.view
    }

    /// Switch top-level view. Recomputes the cache so the next frame reflects
    /// the change, snapshots the outgoing view's cursor, and restores the
    /// incoming view's saved cursor (clamped to the new visible length).
    pub fn set_view(&mut self, view: View) {
        if self.view == view {
            return;
        }
        self.view_cursor[self.view.idx()] = self.cursor;
        self.view = view;
        self.recompute_visible();
        self.cursor = self.view_cursor[view.idx()];
        self.clamp_cursor();
    }

    /// Set the search-filter text. Cursor resets and the cache is recomputed.
    /// Typing into the search prompt calls this on every keystroke.
    pub fn set_search(&mut self, search: String) {
        self.filter.search = search;
        self.cursor = 0;
        self.recompute_visible();
    }

    /// `true` se o texto do campo de busca é um comando shell (`! ...`), não
    /// uma busca. Usado para suprimir o filtro ao vivo e trocar o rótulo do
    /// status para `SHELL`.
    pub fn search_is_shell(&self) -> bool {
        self.draft.text().trim_start().starts_with('!')
    }

    /// Trata o Enter no modo Search. Se o draft é `! <cmd>`, enfileira o comando
    /// de shell e volta ao Normal; senão confirma a busca. Retorna `true` se um
    /// comando foi enfileirado.
    pub fn commit_search(&mut self) -> bool {
        if let Some(rest) = self.draft.text().trim_start().strip_prefix('!') {
            let cmd = rest.trim().to_string();
            self.mode = Mode::Normal;
            self.draft_clear();
            self.clear_search();
            if cmd.is_empty() {
                return false;
            }
            self.queue_shell(cmd);
            return true;
        }
        // Confirmação normal de busca: mantém o filtro, sai do modo.
        self.mode = Mode::Normal;
        self.cursor = 0;
        false
    }

    /// Clear just the search component of the filter.
    pub fn clear_search(&mut self) {
        if self.filter.search.is_empty() {
            return;
        }
        self.filter.search.clear();
        self.cursor = 0;
        self.recompute_visible();
    }

    /// Set or clear the active project filter. `None` removes it.
    pub fn set_project_filter(&mut self, project: Option<String>) {
        self.filter.project = project;
        self.cursor = 0;
        self.recompute_visible();
    }

    /// Set or clear the active context filter. `None` removes it.
    pub fn set_context_filter(&mut self, context: Option<String>) {
        self.filter.context = context;
        self.cursor = 0;
        self.recompute_visible();
    }

    /// Update the cached "today" string. When it changes, the visible cache
    /// is recomputed so threshold-hidden tasks become visible the moment the
    /// wall clock crosses midnight (without requiring an app restart).
    /// Returns `true` iff the date actually advanced — callers use this to
    /// trigger a redraw.
    pub fn refresh_today(&mut self, now: String) -> bool {
        if self.store.set_today(now) {
            self.recompute_visible();
            true
        } else {
            false
        }
    }

    /// Drop every filter component (project + context + search).
    pub fn clear_filter(&mut self) {
        if !self.filter.has_any() {
            return;
        }
        self.filter.clear();
        self.cursor = 0;
        self.recompute_visible();
    }

    // ---- shared helpers for the mutation wrappers -----------------------

    /// After a successful mutation that returned an absolute task index,
    /// rebuild the visible cache and move the cursor to follow that task.
    pub(crate) fn after_mutation(&mut self, follow_abs: usize) {
        self.recompute_visible();
        self.follow_cursor(follow_abs);
    }

    /// Handle a store reconcile that reloaded the file from disk: reset
    /// transient input state and refresh the view, matching the old
    /// `apply_external_state` behavior.
    pub(crate) fn on_reload(&mut self) {
        self.selection.clear();
        self.selection.exit_edit();
        self.recompute_visible();
        self.clamp_cursor();
        self.flash(crate::brand::tr(
            "file changed on disk — reloaded",
            "arquivo mudou no disco — recarregado",
        ));
    }

    /// Map a store reconcile result to the matching flash + view refresh for a
    /// mutation that produced no change of its own (used on the abort paths).
    pub(crate) fn handle_reconcile_abort(&mut self, r: Reconcile) {
        match r {
            Reconcile::Reloaded => self.on_reload(),
            Reconcile::ReadError => self.flash("read failed"),
            Reconcile::Unchanged => {}
        }
    }

    /// Surface a [`DrainReport`] from `Store::drain_inbox` as a flash, matching
    /// the wording the inline drain used to emit, and refresh the view when
    /// tasks were merged.
    pub(crate) fn apply_drain(&mut self, report: DrainReport) {
        if report.merged > 0 {
            self.recompute_visible();
            self.clamp_cursor();
        }
        if let Some(err) = report.error {
            self.flash(err);
        } else if report.merged > 0 {
            if report.skipped > 0 {
                self.flash(format!(
                    "merged {} from inbox ({} skipped)",
                    report.merged, report.skipped
                ));
            } else {
                self.flash(format!("merged {} from inbox", report.merged));
            }
        } else if report.skipped > 0 {
            self.flash(format!(
                "inbox: {} unparseable, nothing merged",
                report.skipped
            ));
        }
    }

    /// Apply a freshly loaded [`Config`] at runtime — used by the hot-reload
    /// watcher. Rebuilds `prefs` and `saved_filters` from the new config
    /// values, then refreshes the visible task cache so theme/density/sort/
    /// layout changes take effect immediately.
    pub fn reload_config(&mut self, new_cfg: Config) {
        self.prefs = Prefs::from_config(new_cfg.clone());
        self.saved_filters = new_cfg
            .filters
            .iter()
            .map(|(name, query)| SavedFilter {
                name: name.clone(),
                query: query.clone(),
            })
            .collect();
        self.advisor_projects = new_cfg.advisor_projects.clone();
        self.advisor_links = new_cfg.advisor_links.clone();
        self.advisor_goals = new_cfg.advisor_goals.clone();
        self.week_start = new_cfg.week_start.unwrap_or(WeekStart::Sunday);
        self.recompute_visible();
    }

    /// `true` se o projeto tem o advisor ligado (para o indicador `✦`).
    pub fn advisor_project_enabled(&self, name: &str) -> bool {
        self.advisor_projects.iter().any(|p| p == name)
    }

    /// O repo GitHub vinculado a um projeto, se houver.
    pub fn linked_repo(&self, project: &str) -> Option<&str> {
        self.advisor_links
            .iter()
            .find(|(p, _)| p == project)
            .map(|(_, r)| r.as_str())
    }

    /// Issues atualmente em cache (visão Issues).
    pub fn issues(&self) -> &[crate::advisor::github::IssueRow] {
        &self.issues
    }

    pub fn issues_cursor(&self) -> usize {
        self.issues_cursor
    }

    /// Move o cursor da visão Issues (`down=true` desce).
    pub fn issues_step(&mut self, down: bool) {
        if self.issues.is_empty() {
            self.issues_cursor = 0;
            return;
        }
        let last = self.issues.len() - 1;
        self.issues_cursor = if down {
            (self.issues_cursor + 1).min(last)
        } else {
            self.issues_cursor.saturating_sub(1)
        };
    }

    /// Entra na visão Issues. O repo-alvo é o do `+projeto` em foco (se
    /// vinculado); se nenhum estiver em foco mas houver **exatamente um** projeto
    /// vinculado, usa esse (conveniência). Sem alvo → flash orientando.
    pub fn enter_issues_view(&mut self) {
        // Projeto em foco, se vinculado.
        let focused = self
            .filter
            .project
            .clone()
            .and_then(|p| self.linked_repo(&p).map(|r| (p.clone(), r.to_string())));
        // Fallback: único vínculo existente.
        let target = focused.or_else(|| {
            if self.advisor_links.len() == 1 {
                let (p, r) = &self.advisor_links[0];
                Some((p.clone(), r.clone()))
            } else {
                None
            }
        });
        let Some((project, repo)) = target else {
            let msg = if self.advisor_links.is_empty() {
                crate::brand::tr(
                    "no linked repo — run `advisor link`",
                    "nenhum repo vinculado — rode `advisor link`",
                )
            } else {
                crate::brand::tr(
                    "focus a linked +project (fp), then I",
                    "foque um +projeto vinculado (fp), depois I",
                )
            };
            self.flash(msg);
            return;
        };
        self.set_view(View::Issues);
        self.mode = Mode::Issues;
        self.issues_cursor = 0;
        // Guarda de qual projeto é a visão, para o import marcar o +tag certo
        // mesmo quando o projeto não estava em foco.
        self.issues_project = Some(project);
        if self.issues_repo.as_deref() != Some(repo.as_str()) {
            self.refresh_issues(&repo);
        }
    }

    /// Re-busca as issues do repo atual da visão (tecla `r`).
    pub fn refresh_current_issues(&mut self) {
        if let Some(repo) = self.issues_repo.clone() {
            self.refresh_issues(&repo);
        }
    }

    /// Objetivo salvo de um projeto, se houver.
    pub fn goal_for(&self, project: &str) -> Option<&str> {
        self.advisor_goals
            .iter()
            .find(|(p, _)| p == project)
            .map(|(_, g)| g.as_str())
    }

    /// Ranqueia as issues em cache rumo ao objetivo (o `override_goal`, senão o
    /// salvo do projeto), preenchendo tier/porquê e reordenando por tier.
    /// Chamada de IA **síncrona** — bloqueia brevemente o TUI.
    pub fn rank_current_issues(&mut self, override_goal: Option<String>) {
        if self.issues.is_empty() {
            return;
        }
        let project = self.issues_project.clone().unwrap_or_default();
        let goal = override_goal.or_else(|| self.goal_for(&project).map(str::to_string));
        let Some(goal) = goal.filter(|g| !g.trim().is_empty()) else {
            self.flash(crate::brand::tr(
                "no goal set — run `advisor goal +proj \"...\"` first",
                "sem objetivo — rode `advisor goal +proj \"...\"` antes",
            ));
            return;
        };
        let cfg = Config::load();
        let advisor_cfg = crate::advisor::AdvisorConfig::resolve(
            cfg.advisor_backend.as_deref(),
            cfg.advisor_model.as_deref(),
        );
        let list: Vec<(u64, String)> = self
            .issues
            .iter()
            .map(|r| (r.number, r.title.clone()))
            .collect();
        match crate::advisor::rank_issues(&advisor_cfg, &goal, &list) {
            Ok(ranking) => {
                let n = ranking.len();
                apply_ranking(&mut self.issues, &ranking);
                self.issues_cursor = 0;
                self.flash(format!("{} {n}", crate::brand::tr("ranked", "ranqueadas")));
            }
            Err(e) => self.flash(format!(
                "{}: {e}",
                crate::brand::tr("rank failed", "falha no ranking")
            )),
        }
    }

    /// Sai da visão Issues de volta para a Lista (Normal).
    pub fn exit_issues_view(&mut self) {
        self.set_view(View::List);
        self.mode = Mode::Normal;
    }

    /// (Re)busca as issues abertas de um repo para a visão Issues.
    pub fn refresh_issues(&mut self, repo: &str) {
        match crate::advisor::github::fetch_issues(repo) {
            Ok(rows) => {
                let n = rows.len();
                self.issues = rows;
                self.issues_repo = Some(repo.to_string());
                self.issues_cursor = 0;
                self.flash(format!("{n} {}", crate::brand::tr("issues", "issues")));
            }
            Err(e) => self.flash(format!(
                "{}: {e}",
                crate::brand::tr("issues fetch failed", "falha ao buscar issues")
            )),
        }
    }

    /// Abre a issue selecionada no navegador do sistema.
    pub fn open_selected_issue(&mut self) {
        let Some(row) = self.issues.get(self.issues_cursor) else {
            return;
        };
        if row.url.is_empty() {
            self.flash(crate::brand::tr("no URL for issue", "issue sem URL"));
            return;
        }
        if let Err(e) = crate::attach::open_with_system(std::path::Path::new(&row.url)) {
            self.flash(format!(
                "{}: {e}",
                crate::brand::tr("open failed", "falha ao abrir")
            ));
        }
    }

    /// Importa a issue selecionada para o todo.txt como tarefa local, marcada
    /// com o token `gh:owner/repo#N` para rastrear a origem.
    pub fn import_selected_issue(&mut self) {
        let repo = match self.issues_repo.clone() {
            Some(r) => r,
            None => return,
        };
        let project = self.issues_project.clone();
        let Some(row) = self.issues.get(self.issues_cursor).cloned() else {
            return;
        };
        let line = issue_import_line(&row, project.as_deref(), &repo);
        match self.store.add_line(&line) {
            crate::core::AddOutcome::Added { .. } => self.flash(format!(
                "{} #{}",
                crate::brand::tr("imported issue", "issue importada"),
                row.number
            )),
            _ => self.flash(crate::brand::tr("import failed", "falha ao importar")),
        }
    }

    /// Reconcile against disk and drain the inbox. Returns `true` when it is
    /// safe to proceed (disk unchanged); `false` when the file was reloaded or
    /// unreadable. The TUI run loop and `handle_key` call this each tick.
    pub fn check_external_changes(&mut self) -> bool {
        let reconcile = self.store.reconcile();
        if matches!(reconcile, Reconcile::Reloaded) {
            self.on_reload();
        }
        let report = self.store.drain_inbox();
        self.apply_drain(report);
        matches!(reconcile, Reconcile::Unchanged)
    }
}

/// Monta a linha todo.txt de import de uma issue: `<título> +<projeto>
/// gh:<owner/repo>#<nº>`. O `+projeto` é omitido quando não há projeto em foco.
/// Isolado para teste puro.
pub(crate) fn issue_import_line(
    row: &crate::advisor::github::IssueRow,
    project: Option<&str>,
    repo: &str,
) -> String {
    match project {
        Some(p) => format!("{} +{p} gh:{repo}#{}", row.title, row.number),
        None => format!("{} gh:{repo}#{}", row.title, row.number),
    }
}

/// Aplica `(número, tier, porquê)` às issues por número e reordena por tier
/// (desc; sem tier por último), de forma estável. Isolado para teste puro.
pub(crate) fn apply_ranking(
    issues: &mut [crate::advisor::github::IssueRow],
    ranking: &[(u64, u8, String)],
) {
    for (number, tier, why) in ranking {
        if let Some(row) = issues.iter_mut().find(|r| r.number == *number) {
            row.tier = Some(*tier);
            row.why = Some(why.clone());
        }
    }
    issues.sort_by_key(|r| std::cmp::Reverse(r.tier.unwrap_or(0)));
}
