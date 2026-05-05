use crossterm::event::{KeyCode, KeyEvent};
use tui_textarea::CursorMove;

use super::NoteEditor;

impl NoteEditor<'_> {
    pub(crate) fn handle_motions(&mut self, key: KeyEvent) -> Option<bool> {
        match key.code {
            KeyCode::Char('h') | KeyCode::Left => {
                let n = self.count();
                for _ in 0..n {
                    self.textarea.move_cursor(CursorMove::Back);
                }
                Some(true)
            }
            KeyCode::Char('j') | KeyCode::Down => {
                let n = self.count();
                for _ in 0..n {
                    self.textarea.move_cursor(CursorMove::Down);
                }
                Some(true)
            }
            KeyCode::Char('k') | KeyCode::Up => {
                let n = self.count();
                for _ in 0..n {
                    self.textarea.move_cursor(CursorMove::Up);
                }
                Some(true)
            }
            KeyCode::Char('l') | KeyCode::Right => {
                let n = self.count();
                for _ in 0..n {
                    self.textarea.move_cursor(CursorMove::Forward);
                }
                Some(true)
            }
            KeyCode::Char('0') => {
                self.textarea.move_cursor(CursorMove::Head);
                Some(true)
            }
            KeyCode::Char('^') => {
                self.move_to_first_nonblank();
                Some(true)
            }
            KeyCode::Char('$') => {
                self.textarea.move_cursor(CursorMove::End);
                Some(true)
            }
            KeyCode::Char('w') => {
                let n = self.count();
                for _ in 0..n {
                    self.textarea.move_cursor(CursorMove::WordForward);
                }
                Some(true)
            }
            KeyCode::Char('e') => {
                let n = self.count();
                for _ in 0..n {
                    self.move_to_word_end();
                }
                Some(true)
            }
            KeyCode::Char('b') => {
                let n = self.count();
                for _ in 0..n {
                    self.textarea.move_cursor(CursorMove::WordBack);
                }
                Some(true)
            }
            KeyCode::Char('G') => {
                if let Some(n) = self.pending_count.take() {
                    self.goto_line(n);
                } else {
                    self.textarea.move_cursor(CursorMove::Bottom);
                }
                Some(true)
            }
            KeyCode::PageUp => {
                let n = self.page_half();
                for _ in 0..n {
                    self.textarea.move_cursor(CursorMove::Up);
                }
                Some(true)
            }
            KeyCode::PageDown => {
                let n = self.page_half();
                for _ in 0..n {
                    self.textarea.move_cursor(CursorMove::Down);
                }
                Some(true)
            }
            _ => None,
        }
    }

    pub(crate) fn handle_editing_keys(&mut self, key: KeyEvent) -> Option<bool> {
        match key.code {
            KeyCode::Char('x') => {
                let n = self.count();
                for _ in 0..n {
                    self.textarea.delete_next_char();
                }
                self.dirty = true;
                Some(true)
            }
            KeyCode::Char('X') => {
                let n = self.count();
                for _ in 0..n {
                    self.textarea.delete_char();
                }
                self.dirty = true;
                Some(true)
            }
            KeyCode::Char('u') => {
                self.textarea.undo();
                self.dirty = true;
                Some(true)
            }
            KeyCode::Char('>') => {
                let n = self.count();
                self.indent_lines(n);
                Some(true)
            }
            KeyCode::Char('<') => {
                let n = self.count();
                self.unindent_lines(n);
                Some(true)
            }
            KeyCode::Char('p') => {
                self.textarea.paste();
                self.dirty = true;
                Some(true)
            }
            KeyCode::Char('P') => {
                self.textarea.move_cursor(CursorMove::Head);
                self.textarea.insert_newline();
                self.textarea.move_cursor(CursorMove::Up);
                self.textarea.paste();
                self.dirty = true;
                Some(true)
            }
            _ => None,
        }
    }
}
