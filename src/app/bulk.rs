use super::App;
use super::types::Mode;
use crate::core::{BulkCompleteOutcome, BulkDeleteOutcome};

impl App {
    /// Bulk-complete every task in the selection that isn't already done.
    /// Clears the selection and exits Visual mode on success. Recurring tasks
    /// also spawn their next instance (handled by the store).
    pub fn complete_selected(&mut self) {
        if self.selection.is_empty() {
            return;
        }
        let indices: Vec<usize> = self.selection.iter().collect();
        let now = chrono::Local::now().format("%H:%M").to_string();
        match self.store.complete_many_at(&indices, Some(&now)) {
            BulkCompleteOutcome::Done { completed, spawned } => {
                self.selection.clear();
                self.mode = Mode::Normal;
                self.flash(if spawned > 0 {
                    format!("completed {completed} (+{spawned} recurring) → archived")
                } else {
                    format!("completed {completed} → archived")
                });
                // Same policy as single-task complete: finished tasks move
                // straight to done.txt.
                if let crate::core::ArchiveOutcome::Error(e) = self.store.archive_completed() {
                    self.flash(format!("archive failed: {e}"));
                }
                self.recompute_visible();
                self.clamp_cursor();
            }
            BulkCompleteOutcome::NothingToComplete => {
                self.flash("nothing to complete");
                self.selection.clear();
                self.mode = Mode::Normal;
            }
            BulkCompleteOutcome::Aborted(r) => self.handle_reconcile_abort(r),
            BulkCompleteOutcome::Error(e) => self.flash(format!("complete failed: {e}")),
        }
    }

    /// Bulk-delete every task in the selection.
    pub fn delete_selected(&mut self) {
        if self.selection.is_empty() {
            return;
        }
        let indices: Vec<usize> = self.selection.iter().collect();
        match self.store.delete_many(&indices) {
            BulkDeleteOutcome::Done { deleted } => {
                self.selection.clear();
                self.mode = Mode::Normal;
                self.flash(format!("deleted {deleted}"));
                self.recompute_visible();
                self.clamp_cursor();
            }
            BulkDeleteOutcome::Nothing => {
                self.selection.clear();
                self.mode = Mode::Normal;
            }
            BulkDeleteOutcome::Aborted(r) => self.handle_reconcile_abort(r),
            BulkDeleteOutcome::Error(e) => self.flash(format!("write failed: {e}")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::test_support::build_app;

    #[test]
    fn complete_selected_archives_stamped_tasks_and_flashes() {
        let mut app = build_app("a\nb\nc\n");
        app.selection.toggle(0);
        app.selection.toggle(2);
        app.mode = Mode::Visual;
        app.complete_selected();
        // Completed tasks move straight to the archive, stamped with done_at.
        assert_eq!(app.tasks().len(), 1);
        assert_eq!(app.tasks()[0].raw, "b");
        let archived = app.archive().tasks();
        assert_eq!(archived.len(), 2);
        assert!(archived.iter().all(|t| t.done && t.raw.contains("done_at:")));
        assert!(app.selection.is_empty());
        assert_eq!(app.mode, Mode::Normal);
        assert_eq!(app.flash_active(), Some("completed 2 → archived"));
    }

    #[test]
    fn complete_selected_reports_recurring_spawns() {
        let mut app = build_app("a\nPay rent due:2026-04-15 rec:+1m\nb\nWater plants rec:1w\n");
        app.refresh_today("2026-05-09".into());
        app.selection.toggle(1);
        app.selection.toggle(3);
        app.mode = Mode::Visual;
        app.complete_selected();
        // 2 completed instances archived; spawned successors stay in the list.
        assert_eq!(app.tasks().len(), 4);
        assert_eq!(app.archive().tasks().len(), 2);
        assert_eq!(
            app.flash_active(),
            Some("completed 2 (+2 recurring) → archived")
        );
    }

    #[test]
    fn delete_selected_removes_all_in_selection() {
        let mut app = build_app("a\nb\nc\nd\n");
        app.selection.toggle(1);
        app.selection.toggle(3);
        app.mode = Mode::Visual;
        app.delete_selected();
        assert_eq!(app.tasks().len(), 2);
        assert_eq!(app.tasks()[0].raw, "a");
        assert_eq!(app.tasks()[1].raw, "c");
        assert!(app.selection.is_empty());
        assert_eq!(app.mode, Mode::Normal);
    }
}
