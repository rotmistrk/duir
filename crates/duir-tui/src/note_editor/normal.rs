use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use tui_textarea::CursorMove;

use super::NoteEditor;

impl NoteEditor<'_> {
    #[allow(clippy::too_many_lines)]
    pub(crate) fn handle_normal(&mut self, key: KeyEvent) -> bool {
        if key.modifiers.contains(KeyModifiers::CONTROL) {
            return match key.code {
                KeyCode::Char('u') => {
                    let n = self.page_half();
                    for _ in 0..n {
                        self.textarea.move_cursor(CursorMove::Up);
                    }
                    true
                }
                KeyCode::Char('d') => {
                    let n = self.page_half();
                    for _ in 0..n {
                        self.textarea.move_cursor(CursorMove::Down);
                    }
                    true
                }
                KeyCode::Char('r') => {
                    self.textarea.redo();
                    self.dirty = true;
                    true
                }
                _ => false,
            };
        }

        if let KeyCode::Char(c @ '1'..='9') = key.code
            && self.pending_op.is_none()
        {
            let digit = (c as usize) - ('0' as usize);
            self.pending_count = Some(self.pending_count.unwrap_or(0) * 10 + digit);
            return true;
        }
        if key.code == KeyCode::Char('0') && self.pending_count.is_some() && self.pending_op.is_none() {
            let val = self.pending_count.unwrap_or(0) * 10;
            self.pending_count = Some(val);
            return true;
        }

        if let Some(op) = self.pending_op {
            self.pending_op = None;
            return match (op, key.code) {
                ('d', KeyCode::Char('d')) => {
                    let n = self.count();
                    self.delete_lines(n);
                    true
                }
                ('y', KeyCode::Char('y')) => {
                    let n = self.count();
                    self.yank_lines(n);
                    true
                }
                _ => {
                    self.pending_count = None;
                    false
                }
            };
        }

        match key.code {
            KeyCode::Char('d') => {
                self.pending_op = Some('d');
                true
            }
            KeyCode::Char('y') => {
                self.pending_op = Some('y');
                true
            }
            KeyCode::Char('i') => {
                self.enter_insert();
                true
            }
            KeyCode::Char('a') => {
                self.textarea.move_cursor(CursorMove::Forward);
                self.enter_insert();
                true
            }
            KeyCode::Char('A') => {
                self.textarea.move_cursor(CursorMove::End);
                self.enter_insert();
                true
            }
            KeyCode::Char('I') => {
                self.textarea.move_cursor(CursorMove::Head);
                self.enter_insert();
                true
            }
            KeyCode::Char('o') => {
                self.textarea.move_cursor(CursorMove::End);
                self.textarea.insert_newline();
                self.auto_indent();
                self.enter_insert();
                self.dirty = true;
                true
            }
            KeyCode::Char('O') => {
                self.textarea.move_cursor(CursorMove::Head);
                self.textarea.insert_newline();
                self.textarea.move_cursor(CursorMove::Up);
                self.enter_insert();
                self.dirty = true;
                true
            }
            KeyCode::Char('v') => {
                self.mode = super::EditorMode::Visual;
                self.pending_count = None;
                self.textarea.start_selection();
                true
            }
            KeyCode::Char('V') => {
                self.mode = super::EditorMode::Visual;
                self.pending_count = None;
                self.textarea.move_cursor(CursorMove::Head);
                self.textarea.start_selection();
                self.textarea.move_cursor(CursorMove::End);
                true
            }
            KeyCode::Char(':') => {
                self.mode = super::EditorMode::Command;
                self.command_buf.clear();
                true
            }
            KeyCode::Char('/') => {
                self.mode = super::EditorMode::Search;
                self.command_buf.clear();
                true
            }
            KeyCode::Char('h') | KeyCode::Left => {
                let n = self.count();
                for _ in 0..n {
                    self.textarea.move_cursor(CursorMove::Back);
                }
                true
            }
            KeyCode::Char('j') | KeyCode::Down => {
                let n = self.count();
                for _ in 0..n {
                    self.textarea.move_cursor(CursorMove::Down);
                }
                true
            }
            KeyCode::Char('k') | KeyCode::Up => {
                let n = self.count();
                for _ in 0..n {
                    self.textarea.move_cursor(CursorMove::Up);
                }
                true
            }
            KeyCode::Char('l') | KeyCode::Right => {
                let n = self.count();
                for _ in 0..n {
                    self.textarea.move_cursor(CursorMove::Forward);
                }
                true
            }
            KeyCode::Char('0') => {
                self.textarea.move_cursor(CursorMove::Head);
                true
            }
            KeyCode::Char('$') => {
                self.textarea.move_cursor(CursorMove::End);
                true
            }
            KeyCode::Char('w') => {
                let n = self.count();
                for _ in 0..n {
                    self.textarea.move_cursor(CursorMove::WordForward);
                }
                true
            }
            KeyCode::Char('b') => {
                let n = self.count();
                for _ in 0..n {
                    self.textarea.move_cursor(CursorMove::WordBack);
                }
                true
            }
            KeyCode::Char('g') => {
                self.textarea.move_cursor(CursorMove::Top);
                true
            }
            KeyCode::Char('G') => {
                self.textarea.move_cursor(CursorMove::Bottom);
                true
            }
            KeyCode::PageUp => {
                let n = self.page_half();
                for _ in 0..n {
                    self.textarea.move_cursor(CursorMove::Up);
                }
                true
            }
            KeyCode::PageDown => {
                let n = self.page_half();
                for _ in 0..n {
                    self.textarea.move_cursor(CursorMove::Down);
                }
                true
            }
            KeyCode::Char('x') => {
                let n = self.count();
                for _ in 0..n {
                    self.textarea.delete_next_char();
                }
                self.dirty = true;
                true
            }
            KeyCode::Char('u') => {
                self.textarea.undo();
                self.dirty = true;
                true
            }
            KeyCode::Char('>') => {
                let n = self.count();
                self.indent_lines(n);
                true
            }
            KeyCode::Char('<') => {
                let n = self.count();
                self.unindent_lines(n);
                true
            }
            KeyCode::Char('p') => {
                self.textarea.paste();
                self.dirty = true;
                true
            }
            KeyCode::Char('P') => {
                self.textarea.move_cursor(CursorMove::Head);
                self.textarea.insert_newline();
                self.textarea.move_cursor(CursorMove::Up);
                self.textarea.paste();
                self.dirty = true;
                true
            }
            KeyCode::Char('n') => {
                self.textarea.search_forward(false);
                true
            }
            KeyCode::Char('N') => {
                self.textarea.search_back(false);
                true
            }
            KeyCode::Enter if key.modifiers.contains(KeyModifiers::SHIFT) => {
                self.open_url_at_cursor();
                true
            }
            _ => {
                self.pending_count = None;
                false
            }
        }
    }

    pub(crate) fn delete_lines(&mut self, n: usize) {
        for _ in 0..n {
            self.textarea.move_cursor(CursorMove::Head);
            self.textarea.start_selection();
            self.textarea.move_cursor(CursorMove::Down);
            self.textarea.cut();
        }
        self.dirty = true;
    }

    pub(crate) fn yank_lines(&mut self, n: usize) {
        self.textarea.move_cursor(CursorMove::Head);
        self.textarea.start_selection();
        for _ in 0..n {
            self.textarea.move_cursor(CursorMove::Down);
        }
        self.textarea.copy();
        self.textarea.cancel_selection();
        self.sync_system_clipboard();
        self.status = format!("{n} line(s) yanked");
    }
}
