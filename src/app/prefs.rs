use std::io;

use super::types::{Density, Sort};
use crate::app::WeekStart;
use crate::config::Config;
use crate::theme::{self, Theme};

#[derive(Debug, Clone)]
pub struct Layout {
    pub left: bool,
    pub right: bool,
    pub line_num: bool,
    pub status_bar: bool,
}

impl Default for Layout {
    fn default() -> Self {
        Self {
            left: true,
            right: true,
            line_num: true,
            status_bar: true,
        }
    }
}

/// User-tunable preferences persisted to `Config`. Cycle/toggle methods return
/// the flash message for the caller to display, sidestepping any `&mut prefs`
/// + `&mut flash_state` borrow tangle on `App`.
#[derive(Debug, Clone)]
pub struct Prefs {
    theme_idx: usize,
    /// Theme name from the config that didn't resolve against the registry
    /// (e.g. a user theme file that this instance didn't load). We fall back
    /// visually but keep the configured name on save, so a stale instance
    /// can't silently downgrade `theme =` for everyone else. Cleared as soon
    /// as the user picks a theme in-app.
    unresolved_theme: Option<String>,
    pub density: Density,
    pub sort: Sort,
    pub layout: Layout,
    pub show_done: bool,
    pub show_future: bool,
    /// Metadata keys whose `key:value` tokens are hidden from task rows.
    /// Config-only (no in-app toggle); see `Config::hidden_keys`.
    pub hidden_keys: Vec<String>,
    pub week_start: WeekStart,
}

impl Prefs {
    pub fn from_config(cfg: Config) -> Self {
        let resolved = cfg
            .theme
            .as_deref()
            .map(|name| theme::all().iter().position(|t| t.name == name));
        let theme_idx = resolved.flatten().unwrap_or(0);
        let unresolved_theme = match resolved {
            Some(None) => cfg.theme.clone(),
            _ => None,
        };
        Self {
            theme_idx,
            unresolved_theme,
            density: cfg.density.unwrap_or(Density::Comfortable),
            sort: cfg.sort.unwrap_or(Sort::Priority),
            layout: Layout {
                left: cfg.show_left.unwrap_or(true),
                right: cfg.show_right.unwrap_or(true),
                line_num: cfg.show_line_num.unwrap_or(true),
                status_bar: cfg.show_status_bar.unwrap_or(true),
            },
            show_done: cfg.show_done.unwrap_or(false),
            show_future: cfg.show_future.unwrap_or(false),
            hidden_keys: cfg.hidden_keys,
            week_start: cfg.week_start.unwrap_or(WeekStart::Sunday),
        }
    }

    pub fn theme(&self) -> &'static Theme {
        let all = theme::all();
        all[self.theme_idx % all.len()]
    }

    pub fn theme_idx(&self) -> usize {
        self.theme_idx
    }

    /// Jump directly to a specific theme by index. Used by the screenshot
    /// example to render every theme; production code should call
    /// `cycle_theme` instead so the change persists with a flash message.
    pub fn set_theme_idx(&mut self, idx: usize) {
        self.theme_idx = idx % theme::all().len();
        self.unresolved_theme = None;
    }

    pub fn sort_label(&self) -> &'static str {
        self.sort.as_str()
    }

    pub fn cycle_theme(&mut self) -> String {
        self.theme_idx = (self.theme_idx + 1) % theme::all().len();
        self.unresolved_theme = None;
        format!("theme: {}", self.theme().name)
    }

    pub fn cycle_density(&mut self) -> String {
        self.density = match self.density {
            Density::Compact => Density::Comfortable,
            Density::Comfortable => Density::Cozy,
            Density::Cozy => Density::Compact,
        };
        format!("density: {}", self.density)
    }

    pub fn cycle_sort(&mut self) -> String {
        self.sort = match self.sort {
            Sort::Priority => Sort::Due,
            Sort::Due => Sort::File,
            Sort::File => Sort::Priority,
        };
        format!("sort: {}", self.sort)
    }

    pub fn toggle_left(&mut self) {
        self.layout.left = !self.layout.left;
    }

    pub fn toggle_right(&mut self) {
        self.layout.right = !self.layout.right;
    }

    pub fn toggle_line_num(&mut self) {
        self.layout.line_num = !self.layout.line_num;
    }

    pub fn toggle_show_done(&mut self) {
        self.show_done = !self.show_done;
    }

    pub fn toggle_show_future(&mut self) {
        self.show_future = !self.show_future;
    }

    pub fn cycle_week_start(&mut self) -> String {
        self.week_start = match self.week_start {
            WeekStart::Sunday => WeekStart::Monday,
            WeekStart::Monday => WeekStart::Sunday,
        };
        format!("week_start: {}", self.week_start)
    }

    /// Persist to the XDG config path. Returns the IO error so the caller
    /// can flash it (writing to stderr from inside the alt-screen would
    /// corrupt the TUI). Saving is best-effort — callers that don't care
    /// about reporting can `let _ = prefs.save();`.
    ///
    /// Loads the on-disk config first so non-pref fields (like
    /// `share_token` / `share_port`, owned by the capture server) are
    /// preserved across pref toggles.
    pub fn save(&self) -> io::Result<()> {
        let mut cfg = Config::load();
        // An unresolved configured theme round-trips by name; only a theme
        // this instance actually knows overwrites it.
        cfg.theme = Some(
            self.unresolved_theme
                .clone()
                .unwrap_or_else(|| self.theme().name.to_string()),
        );
        cfg.density = Some(self.density);
        cfg.sort = Some(self.sort);
        cfg.show_left = Some(self.layout.left);
        cfg.show_right = Some(self.layout.right);
        cfg.show_line_num = Some(self.layout.line_num);
        cfg.show_status_bar = Some(self.layout.status_bar);
        cfg.show_done = Some(self.show_done);
        cfg.show_future = Some(self.show_future);
        // `hidden_keys` has no in-app toggle — the config file is its only
        // editor. Keep the freshly-loaded disk value instead of echoing our
        // in-memory copy, which may predate an external edit and would
        // silently erase it on the next pref save.
        cfg.save()
    }
}
