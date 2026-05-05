use crossterm::event::{KeyCode, KeyEvent};
use tui_textarea::CursorMove;

use super::NoteEditor;

impl NoteEditor<'_> {
    pub(crate) fn handle_normal_key(&mut self, key: KeyEvent) -> bool {
        if let Some(handled) = self.handle_operators(key) {
            return handled;
        }
        if let Some(handled) = self.handle_insert_entry(key) {
            return handled;
        }
        if let Some(handled) = self.handle_substitute_keys(key) {
            return handled;
        }
        if let Some(handled) = self.handle_visual_command(key) {
            return handled;
        }
        if let Some(handled) = self.handle_motions(key) {
            return handled;
        }
        if let Some(handled) = self.handle_editing_keys(key) {
            return handled;
        }
        if let Some(handled) = self.handle_search_keys(key) {
            return handled;
        }
        self.pending_count = None;
        false
    }

    const fn handle_operators(&mut self, key: KeyEvent) -> Option<bool> {
        match key.code {
            KeyCode::Char('d') => {
                self.pending_op = Some('d');
                Some(true)
            }
            KeyCode::Char('c') => {
                self.pending_op = Some('c');
                Some(true)
            }
            KeyCode::Char('y') => {
                self.pending_op = Some('y');
                Some(true)
            }
            KeyCode::Char('r') => {
                self.pending_char = Some('r');
                Some(true)
            }
            KeyCode::Char('f') => {
                self.pending_char = Some('f');
                Some(true)
            }
            KeyCode::Char('F') => {
                self.pending_char = Some('F');
                Some(true)
            }
            KeyCode::Char('t') => {
                self.pending_char = Some('t');
                Some(true)
            }
            KeyCode::Char('T') => {
                self.pending_char = Some('T');
                Some(true)
            }
            KeyCode::Char('g') => {
                self.pending_char = Some('g');
                Some(true)
            }
            _ => None,
        }
    }

    fn handle_insert_entry(&mut self, key: KeyEvent) -> Option<bool> {
        match key.code {
            KeyCode::Char('i') => {
                self.enter_insert();
                Some(true)
            }
            KeyCode::Char('a') => {
                self.textarea.move_cursor(CursorMove::Forward);
                self.enter_insert();
                Some(true)
            }
            KeyCode::Char('A') => {
                self.textarea.move_cursor(CursorMove::End);
                self.enter_insert();
                Some(true)
            }
            KeyCode::Char('I') => {
                self.move_to_first_nonblank();
                self.enter_insert();
                Some(true)
            }
            KeyCode::Char('o') => {
                self.textarea.move_cursor(CursorMove::End);
                self.textarea.insert_newline();
                self.auto_indent();
                self.enter_insert();
                self.dirty = true;
                Some(true)
            }
            KeyCode::Char('O') => {
                self.textarea.move_cursor(CursorMove::Head);
                self.textarea.insert_newline();
                self.textarea.move_cursor(CursorMove::Up);
                self.enter_insert();
                self.dirty = true;
                Some(true)
            }
            _ => None,
        }
    }

    fn handle_substitute_keys(&mut self, key: KeyEvent) -> Option<bool> {
        match key.code {
            KeyCode::Char('s') => {
                let n = self.count();
                for _ in 0..n {
                    self.textarea.delete_next_char();
                }
                self.dirty = true;
                self.enter_insert();
                Some(true)
            }
            KeyCode::Char('S') => {
                self.textarea.move_cursor(CursorMove::Head);
                self.textarea.start_selection();
                self.textarea.move_cursor(CursorMove::End);
                self.textarea.cut();
                self.dirty = true;
                self.enter_insert();
                Some(true)
            }
            KeyCode::Char('C') => {
                self.textarea.start_selection();
                self.textarea.move_cursor(CursorMove::End);
                self.textarea.cut();
                self.dirty = true;
                self.enter_insert();
                Some(true)
            }
            KeyCode::Char('D') => {
                self.textarea.start_selection();
                self.textarea.move_cursor(CursorMove::End);
                self.textarea.cut();
                self.dirty = true;
                Some(true)
            }
            KeyCode::Char('J') => {
                self.join_lines();
                Some(true)
            }
            KeyCode::Char('~') => {
                self.toggle_case();
                Some(true)
            }
            _ => None,
        }
    }

    fn handle_visual_command(&mut self, key: KeyEvent) -> Option<bool> {
        match key.code {
            KeyCode::Char('v') => {
                self.mode = super::EditorMode::Visual;
                self.pending_count = None;
                self.textarea.start_selection();
                Some(true)
            }
            KeyCode::Char('V') => {
                self.mode = super::EditorMode::Visual;
                self.pending_count = None;
                self.textarea.move_cursor(CursorMove::Head);
                self.textarea.start_selection();
                self.textarea.move_cursor(CursorMove::End);
                Some(true)
            }
            KeyCode::Char(':') => {
                self.mode = super::EditorMode::Command;
                self.command_buf.clear();
                Some(true)
            }
            KeyCode::Char('/') => {
                self.mode = super::EditorMode::Search;
                self.command_buf.clear();
                Some(true)
            }
            _ => None,
        }
    }
}
