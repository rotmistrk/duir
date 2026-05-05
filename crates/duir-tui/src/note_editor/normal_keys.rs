use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use tui_textarea::CursorMove;

use super::NoteEditor;

impl NoteEditor<'_> {
    #[allow(clippy::too_many_lines)]
    pub(crate) fn handle_normal_key(&mut self, key: KeyEvent) -> bool {
        match key.code {
            // Operators
            KeyCode::Char('d') => {
                self.pending_op = Some('d');
                true
            }
            KeyCode::Char('c') => {
                self.pending_op = Some('c');
                true
            }
            KeyCode::Char('y') => {
                self.pending_op = Some('y');
                true
            }

            // Pending char commands
            KeyCode::Char('r') => {
                self.pending_char = Some('r');
                true
            }
            KeyCode::Char('f') => {
                self.pending_char = Some('f');
                true
            }
            KeyCode::Char('F') => {
                self.pending_char = Some('F');
                true
            }
            KeyCode::Char('t') => {
                self.pending_char = Some('t');
                true
            }
            KeyCode::Char('T') => {
                self.pending_char = Some('T');
                true
            }
            KeyCode::Char('g') => {
                self.pending_char = Some('g');
                true
            }

            // Insert mode entry
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
                self.move_to_first_nonblank();
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

            // Substitute / change / delete shortcuts
            KeyCode::Char('s') => {
                let n = self.count();
                for _ in 0..n {
                    self.textarea.delete_next_char();
                }
                self.dirty = true;
                self.enter_insert();
                true
            }
            KeyCode::Char('S') => {
                self.textarea.move_cursor(CursorMove::Head);
                self.textarea.start_selection();
                self.textarea.move_cursor(CursorMove::End);
                self.textarea.cut();
                self.dirty = true;
                self.enter_insert();
                true
            }
            KeyCode::Char('C') => {
                self.textarea.start_selection();
                self.textarea.move_cursor(CursorMove::End);
                self.textarea.cut();
                self.dirty = true;
                self.enter_insert();
                true
            }
            KeyCode::Char('D') => {
                self.textarea.start_selection();
                self.textarea.move_cursor(CursorMove::End);
                self.textarea.cut();
                self.dirty = true;
                true
            }
            KeyCode::Char('J') => {
                self.join_lines();
                true
            }
            KeyCode::Char('~') => {
                self.toggle_case();
                true
            }

            // Visual mode
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

            // Command/search
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

            // Motions
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
            KeyCode::Char('^') => {
                self.move_to_first_nonblank();
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
            KeyCode::Char('e') => {
                let n = self.count();
                for _ in 0..n {
                    self.move_to_word_end();
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
            KeyCode::Char('G') => {
                if let Some(n) = self.pending_count.take() {
                    self.goto_line(n);
                } else {
                    self.textarea.move_cursor(CursorMove::Bottom);
                }
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

            // Editing
            KeyCode::Char('x') => {
                let n = self.count();
                for _ in 0..n {
                    self.textarea.delete_next_char();
                }
                self.dirty = true;
                true
            }
            KeyCode::Char('X') => {
                let n = self.count();
                for _ in 0..n {
                    self.textarea.delete_char();
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

            // Search
            KeyCode::Char('n') => {
                self.textarea.search_forward(false);
                true
            }
            KeyCode::Char('N') => {
                self.textarea.search_back(false);
                true
            }
            KeyCode::Char('*') => {
                self.search_word_under_cursor(true);
                true
            }
            KeyCode::Char('#') => {
                self.search_word_under_cursor(false);
                true
            }

            // Repeat find
            KeyCode::Char(';') => {
                if let Some((cmd, ch)) = self.last_find {
                    self.execute_find(cmd, ch);
                }
                true
            }
            KeyCode::Char(',') => {
                if let Some((cmd, ch)) = self.last_find {
                    let rev = match cmd {
                        'f' => 'F',
                        'F' => 'f',
                        't' => 'T',
                        'T' => 't',
                        _ => cmd,
                    };
                    self.execute_find(rev, ch);
                }
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
}
