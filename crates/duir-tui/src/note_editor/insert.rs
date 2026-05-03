use crossterm::event::{KeyCode, KeyEvent};

use super::NoteEditor;

impl NoteEditor<'_> {
    pub(crate) fn handle_insert(&mut self, key: KeyEvent) -> bool {
        if key.code == KeyCode::Esc {
            self.enter_normal();
        } else if key.code == KeyCode::Enter {
            self.textarea.insert_newline();
            self.auto_indent();
            self.dirty = true;
        } else if key.code == KeyCode::Tab {
            let (_, col) = self.textarea.cursor();
            let tab_width = self.textarea.tab_length() as usize;
            let spaces = tab_width - (col % tab_width);
            self.textarea.insert_str(" ".repeat(spaces));
            self.dirty = true;
        } else if key.code == KeyCode::Backspace {
            let (row, col) = self.textarea.cursor();
            let line = self.textarea.lines().get(row).cloned().unwrap_or_default();
            let leading_ws = line.len() - line.trim_start().len();
            let tab_width = self.textarea.tab_length() as usize;
            if col > 0 && col <= leading_ws && col % tab_width == 0 {
                for _ in 0..tab_width {
                    self.textarea.delete_char();
                }
            } else {
                self.textarea.delete_char();
            }
            self.dirty = true;
        } else {
            self.textarea.input(key);
            self.dirty = true;
        }
        true
    }
}
