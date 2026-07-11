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
        })
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

    pub fn move_up(&mut self) {
        self.row = self.row.saturating_sub(1);
        self.clamp_col();
    }

    pub fn move_down(&mut self) {
        self.row = (self.row + 1).min(self.lines.len() - 1);
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
