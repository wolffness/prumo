use std::cell::Cell;
use std::path::PathBuf;

/// In-TUI note editor state. Holds the note file as a line buffer plus a
/// cursor. Opened by `N` (see `App::open_note_panel_for_current`), rendered
/// by `ui::note_panel`, driven by `handle_note` in `main.rs`.
///
/// The buffer is the source of truth while the panel is open; `save` writes
/// it back to `path`. `dirty` tracks unsaved edits so closing the panel can
/// persist them without prompting.
pub struct NotePanel {
    pub path: PathBuf,
    /// Title shown in the panel border (the task body).
    pub title: String,
    pub lines: Vec<String>,
    /// Cursor row, an index into `lines`.
    pub row: usize,
    /// Cursor column in characters (not bytes), clamped per-row on use.
    pub col: usize,
    /// True while typing (Insert); false in view/Normal mode.
    pub insert: bool,
    pub dirty: bool,
    /// Vertical scroll offset, updated at render time (same pattern as
    /// `App::view_scroll`).
    pub scroll: Cell<u16>,
    /// Display width the renderer wraps lines at, published at render time
    /// so vertical cursor motion can step through *visual* rows of a long
    /// wrapped line instead of jumping whole buffer lines. `usize::MAX`
    /// (the pre-first-render default) degrades to plain line movement.
    pub wrap_w: Cell<usize>,
    /// Selection anchor (`row`, `col` in chars), set when Shift+movement
    /// starts extending a selection. The selected region spans from the
    /// anchor to the cursor; `None` means no active selection.
    pub sel_anchor: Option<(usize, usize)>,
}

impl NotePanel {
    pub fn load(path: PathBuf, title: String) -> std::io::Result<Self> {
        let body = std::fs::read_to_string(&path)?;
        let mut lines: Vec<String> = body.lines().map(str::to_string).collect();
        if lines.is_empty() {
            lines.push(String::new());
        }
        Ok(Self {
            path,
            title,
            lines,
            row: 0,
            col: 0,
            insert: false,
            dirty: false,
            scroll: Cell::new(0),
            wrap_w: Cell::new(usize::MAX),
            sel_anchor: None,
        })
    }

    /// Selection bookkeeping for a movement key: Shift extends (anchoring at
    /// the current position on the first shifted move); unshifted movement
    /// drops the selection.
    pub fn track_selection(&mut self, shift: bool) {
        if shift {
            if self.sel_anchor.is_none() {
                self.clamp_col();
                self.sel_anchor = Some((self.row, self.col));
            }
        } else {
            self.sel_anchor = None;
        }
    }

    /// Ordered selection bounds `((row, col), (row, col))`, start < end, end
    /// exclusive. `None` when there is no selection or it is empty.
    pub fn selection_range(&self) -> Option<((usize, usize), (usize, usize))> {
        let anchor = self.sel_anchor?;
        let cursor = (self.row, self.col.min(self.lines[self.row].chars().count()));
        match anchor.cmp(&cursor) {
            std::cmp::Ordering::Less => Some((anchor, cursor)),
            std::cmp::Ordering::Greater => Some((cursor, anchor)),
            std::cmp::Ordering::Equal => None,
        }
    }

    /// Remove the selected region, leaving the cursor at its start. Returns
    /// false (no-op) when nothing is selected.
    pub fn delete_selection(&mut self) -> bool {
        let Some(((r1, c1), (r2, c2))) = self.selection_range() else {
            self.sel_anchor = None;
            return false;
        };
        let byte = |line: &str, col: usize| {
            line.char_indices()
                .nth(col)
                .map_or(line.len(), |(i, _)| i)
        };
        if r1 == r2 {
            let (s, e) = (byte(&self.lines[r1], c1), byte(&self.lines[r1], c2));
            self.lines[r1].replace_range(s..e, "");
        } else {
            let tail = {
                let end_line = &self.lines[r2];
                end_line[byte(end_line, c2)..].to_string()
            };
            let s = byte(&self.lines[r1], c1);
            self.lines[r1].truncate(s);
            self.lines[r1].push_str(&tail);
            self.lines.drain(r1 + 1..=r2);
        }
        self.row = r1;
        self.col = c1;
        self.sel_anchor = None;
        self.dirty = true;
        true
    }

    pub fn save(&mut self) -> std::io::Result<()> {
        let mut body = self.lines.join("\n");
        body.push('\n');
        std::fs::write(&self.path, body)?;
        self.dirty = false;
        Ok(())
    }

    fn cur_line_len(&self) -> usize {
        self.lines[self.row].chars().count()
    }

    /// Byte offset of character column `col` in the current line.
    fn byte_at(&self, col: usize) -> usize {
        let line = &self.lines[self.row];
        line.char_indices()
            .nth(col)
            .map_or(line.len(), |(i, _)| i)
    }

    pub fn clamp_col(&mut self) {
        // In Normal mode the cursor sits on a character; in Insert it may sit
        // one past the end (to append).
        let max = self.cur_line_len();
        let max = if self.insert { max } else { max.saturating_sub(1) };
        self.col = self.col.min(max);
    }

    /// Move up one *visual* row: within a wrapped line first, then into the
    /// last visual row of the previous buffer line.
    pub fn move_up(&mut self) {
        let w = self.wrap_w.get().max(1);
        if self.col >= w {
            self.col -= w;
        } else if self.row > 0 {
            self.row -= 1;
            let plen = self.cur_line_len();
            let last_start = if plen == 0 {
                0
            } else {
                ((plen - 1) / w) * w
            };
            self.col = (last_start + self.col).min(plen);
        }
        self.clamp_col();
    }

    /// Move down one *visual* row (see `move_up`).
    pub fn move_down(&mut self) {
        let w = self.wrap_w.get().max(1);
        let len = self.cur_line_len();
        let cur_chunk = self.col / w;
        let last_chunk = if len == 0 { 0 } else { (len - 1) / w };
        if cur_chunk < last_chunk {
            self.col = (self.col + w).min(len);
        } else if self.row + 1 < self.lines.len() {
            self.row += 1;
            self.col %= w;
        }
        self.clamp_col();
    }

    pub fn move_left(&mut self) {
        self.col = self.col.saturating_sub(1);
    }

    pub fn move_right(&mut self) {
        self.col += 1;
        self.clamp_col();
    }

    pub fn move_top(&mut self) {
        self.row = 0;
        self.clamp_col();
    }

    pub fn move_bottom(&mut self) {
        self.row = self.lines.len() - 1;
        self.clamp_col();
    }

    pub fn insert_char(&mut self, c: char) {
        self.clamp_col();
        let at = self.byte_at(self.col);
        self.lines[self.row].insert(at, c);
        self.col += 1;
        self.dirty = true;
    }

    pub fn newline(&mut self) {
        self.clamp_col();
        let at = self.byte_at(self.col);
        let rest = self.lines[self.row].split_off(at);
        self.lines.insert(self.row + 1, rest);
        self.row += 1;
        self.col = 0;
        self.dirty = true;
    }

    /// Backspace: delete the char before the cursor, joining lines at col 0.
    pub fn backspace(&mut self) {
        self.clamp_col();
        if self.col > 0 {
            let at = self.byte_at(self.col - 1);
            self.lines[self.row].remove(at);
            self.col -= 1;
            self.dirty = true;
        } else if self.row > 0 {
            let cur = self.lines.remove(self.row);
            self.row -= 1;
            self.col = self.cur_line_len();
            self.lines[self.row].push_str(&cur);
            self.dirty = true;
        }
    }

    /// Forward delete: remove the char under the cursor, joining the next
    /// line up when the cursor sits at end-of-line.
    pub fn delete_forward(&mut self) {
        self.clamp_col();
        if self.col < self.cur_line_len() {
            let at = self.byte_at(self.col);
            self.lines[self.row].remove(at);
            self.dirty = true;
        } else if self.row + 1 < self.lines.len() {
            let next = self.lines.remove(self.row + 1);
            self.lines[self.row].push_str(&next);
            self.dirty = true;
        }
    }

    pub fn line_start(&mut self) {
        self.col = 0;
    }

    pub fn line_end(&mut self) {
        self.col = self.cur_line_len();
        self.clamp_col();
    }

    /// Char classes for word motion: whitespace / alphanumeric / other.
    fn char_class(c: char) -> u8 {
        if c.is_whitespace() {
            0
        } else if c.is_alphanumeric() || c == '_' {
            1
        } else {
            2
        }
    }

    /// Move to the start of the previous word.
    pub fn word_left(&mut self) {
        self.clamp_col();
        if self.col == 0 {
            if self.row > 0 {
                self.row -= 1;
                self.col = self.cur_line_len();
                self.clamp_col();
            }
            return;
        }
        let chars: Vec<char> = self.lines[self.row].chars().collect();
        let mut i = self.col;
        while i > 0 && chars[i - 1].is_whitespace() {
            i -= 1;
        }
        if i > 0 {
            let class = Self::char_class(chars[i - 1]);
            while i > 0 && Self::char_class(chars[i - 1]) == class {
                i -= 1;
            }
        }
        self.col = i;
    }

    /// Move past the end of the current/next word.
    pub fn word_right(&mut self) {
        self.clamp_col();
        let chars: Vec<char> = self.lines[self.row].chars().collect();
        if self.col >= chars.len() {
            if self.row + 1 < self.lines.len() {
                self.row += 1;
                self.col = 0;
            }
            return;
        }
        let mut i = self.col;
        while i < chars.len() && chars[i].is_whitespace() {
            i += 1;
        }
        if i < chars.len() {
            let class = Self::char_class(chars[i]);
            while i < chars.len() && Self::char_class(chars[i]) == class {
                i += 1;
            }
        }
        self.col = i;
        self.clamp_col();
    }

    /// Delete from the previous word boundary to the cursor (Ctrl+W).
    pub fn delete_word_back(&mut self) {
        self.clamp_col();
        let (row, end) = (self.row, self.col);
        self.word_left();
        if self.row != row {
            // Cursor was at col 0: word_left crossed to the previous row.
            // Restore — Ctrl+W deletes within a line only.
            self.row = row;
            self.col = end;
            return;
        }
        let start = self.col;
        if end > start {
            let sb = self.byte_at(start);
            let eb = self.byte_at(end);
            self.lines[self.row].replace_range(sb..eb, "");
            self.dirty = true;
        }
    }

    /// Delete from the cursor to end of line (Ctrl+K). On an already-empty
    /// tail, joins the next line (Emacs behavior).
    pub fn kill_to_end(&mut self) {
        self.clamp_col();
        if self.col < self.cur_line_len() {
            let at = self.byte_at(self.col);
            self.lines[self.row].truncate(at);
            self.dirty = true;
        } else {
            self.delete_forward();
        }
    }

    /// Delete from start of line to the cursor (Ctrl+U).
    pub fn kill_to_start(&mut self) {
        self.clamp_col();
        if self.col > 0 {
            let at = self.byte_at(self.col);
            self.lines[self.row].replace_range(..at, "");
            self.col = 0;
            self.dirty = true;
        }
    }

    /// Delete the whole current line (`dd`), keeping at least one line.
    pub fn delete_line(&mut self) {
        if self.lines.len() == 1 {
            if !self.lines[0].is_empty() {
                self.lines[0].clear();
                self.dirty = true;
            }
        } else {
            self.lines.remove(self.row);
            self.row = self.row.min(self.lines.len() - 1);
            self.dirty = true;
        }
        self.col = 0;
    }

    /// Delete the char under the cursor (`x` in view mode).
    pub fn delete_char_at_cursor(&mut self) {
        self.delete_forward();
        self.clamp_col();
    }

    pub fn page_down(&mut self, rows: usize) {
        self.row = (self.row + rows).min(self.lines.len() - 1);
        self.clamp_col();
    }

    pub fn page_up(&mut self, rows: usize) {
        self.row = self.row.saturating_sub(rows);
        self.clamp_col();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn panel(body: &str) -> NotePanel {
        NotePanel {
            path: PathBuf::from("/dev/null"),
            title: "t".into(),
            lines: if body.is_empty() {
                vec![String::new()]
            } else {
                body.lines().map(str::to_string).collect()
            },
            row: 0,
            col: 0,
            insert: true,
            dirty: false,
            scroll: Cell::new(0),
            wrap_w: Cell::new(usize::MAX),
            sel_anchor: None,
        }
    }

    #[test]
    fn insert_char_advances_cursor_and_marks_dirty() {
        let mut p = panel("");
        p.insert_char('a');
        p.insert_char('é');
        p.insert_char('b');
        assert_eq!(p.lines[0], "aéb");
        assert_eq!(p.col, 3);
        assert!(p.dirty);
    }

    #[test]
    fn newline_splits_line_at_cursor() {
        let mut p = panel("hello");
        p.col = 2;
        p.newline();
        assert_eq!(p.lines, vec!["he", "llo"]);
        assert_eq!((p.row, p.col), (1, 0));
    }

    #[test]
    fn backspace_at_col_zero_joins_lines() {
        let mut p = panel("ab\ncd");
        p.row = 1;
        p.col = 0;
        p.backspace();
        assert_eq!(p.lines, vec!["abcd"]);
        assert_eq!((p.row, p.col), (0, 2));
    }

    #[test]
    fn backspace_removes_multibyte_char() {
        let mut p = panel("café");
        p.col = 4;
        p.backspace();
        assert_eq!(p.lines[0], "caf");
    }

    #[test]
    fn normal_mode_clamps_cursor_onto_last_char() {
        let mut p = panel("abc\nx");
        p.col = 3; // one past end, valid in insert
        p.insert = false;
        p.move_down();
        assert_eq!((p.row, p.col), (1, 0));
    }

    #[test]
    fn backspace_at_origin_is_noop() {
        let mut p = panel("ab");
        p.backspace();
        assert_eq!(p.lines, vec!["ab"]);
        assert!(!p.dirty);
    }

    #[test]
    fn vertical_motion_steps_through_visual_rows_of_wrapped_line() {
        // One 25-char line wrapped at 10 → visual rows at cols 0/10/20.
        let mut p = panel("abcdefghijklmnopqrstuvwxy");
        p.wrap_w.set(10);
        p.col = 3;
        p.move_down();
        assert_eq!(p.col, 13, "down moves one visual row within the line");
        p.move_down();
        assert_eq!(p.col, 23);
        p.move_down();
        assert_eq!(p.col, 23, "bottom visual row of last line: no-op");
        p.move_up();
        assert_eq!(p.col, 13);
        p.move_up();
        assert_eq!(p.col, 3);
    }

    #[test]
    fn vertical_motion_crosses_buffer_lines_via_visual_rows() {
        let mut p = panel("abcdefghijklmno\nshort");
        p.wrap_w.set(10);
        p.col = 12; // second visual row of line 0
        p.move_down();
        assert_eq!((p.row, p.col), (1, 2), "into the next buffer line");
        p.move_up();
        assert_eq!((p.row, p.col), (0, 12), "back to the last visual row");
    }

    #[test]
    fn shift_selection_deletes_within_line() {
        let mut p = panel("hello world");
        // Anchor at 0, extend to 6 ("hello ").
        p.track_selection(true);
        p.col = 6;
        assert!(p.delete_selection());
        assert_eq!(p.lines[0], "world");
        assert_eq!((p.row, p.col), (0, 0));
        assert!(p.sel_anchor.is_none());
    }

    #[test]
    fn shift_selection_deletes_across_lines() {
        let mut p = panel("first line\nsecond\nthird");
        p.col = 6; // after "first "
        p.track_selection(true);
        p.row = 2;
        p.col = 2; // into "third"
        assert!(p.delete_selection());
        assert_eq!(p.lines, vec!["first ird"]);
        assert_eq!((p.row, p.col), (0, 6));
    }

    #[test]
    fn unshifted_movement_clears_selection() {
        let mut p = panel("abc");
        p.track_selection(true);
        p.col = 2;
        assert!(p.selection_range().is_some());
        p.track_selection(false);
        assert!(p.selection_range().is_none());
        assert!(!p.delete_selection(), "nothing selected: no-op");
        assert_eq!(p.lines[0], "abc");
    }

    #[test]
    fn typing_replaces_selection() {
        let mut p = panel("abcdef");
        p.track_selection(true);
        p.col = 4;
        p.delete_selection();
        p.insert_char('X');
        assert_eq!(p.lines[0], "Xef");
    }

    #[test]
    fn delete_forward_removes_char_and_joins_lines() {
        let mut p = panel("ab\ncd");
        p.col = 1;
        p.delete_forward();
        assert_eq!(p.lines[0], "a");
        p.delete_forward(); // at end-of-line: joins next row up
        assert_eq!(p.lines, vec!["acd"]);
    }

    #[test]
    fn home_end_and_word_motion() {
        let mut p = panel("foo bar-baz qux");
        p.line_end();
        assert_eq!(p.col, 15);
        p.line_start();
        assert_eq!(p.col, 0);
        p.word_right();
        assert_eq!(p.col, 3, "past 'foo'");
        p.word_right();
        assert_eq!(p.col, 7, "past 'bar'");
        p.word_left();
        assert_eq!(p.col, 4, "back to start of 'bar'");
    }

    #[test]
    fn ctrl_w_deletes_word_within_line_only() {
        let mut p = panel("hello world");
        p.line_end();
        p.delete_word_back();
        assert_eq!(p.lines[0], "hello ");
        p.col = 0;
        let before = p.lines.clone();
        p.delete_word_back(); // at col 0: no cross-line deletion
        assert_eq!(p.lines, before);
        assert_eq!((p.row, p.col), (0, 0));
    }

    #[test]
    fn kill_to_end_and_start() {
        let mut p = panel("abcdef");
        p.col = 3;
        p.kill_to_end();
        assert_eq!(p.lines[0], "abc");
        p.kill_to_start();
        assert_eq!(p.lines[0], "");
        assert_eq!(p.col, 0);
    }

    #[test]
    fn delete_line_keeps_at_least_one_line() {
        let mut p = panel("one\ntwo");
        p.delete_line();
        assert_eq!(p.lines, vec!["two"]);
        p.delete_line();
        assert_eq!(p.lines, vec![""]);
        assert!(!p.lines.is_empty());
    }

    #[test]
    fn save_writes_buffer_with_trailing_newline() {
        let dir = std::env::temp_dir().join("tuxedo-note-panel-test");
        std::fs::create_dir_all(&dir).expect("mkdir");
        let path = dir.join("note.md");
        let mut p = panel("# Title\nbody");
        p.path = path.clone();
        p.dirty = true;
        p.save().expect("save");
        assert_eq!(
            std::fs::read_to_string(&path).expect("read"),
            "# Title\nbody\n"
        );
        assert!(!p.dirty);
    }
}
