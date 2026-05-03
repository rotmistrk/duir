use crossterm::event::{KeyCode, KeyEvent};
use tui_textarea::CursorMove;

use super::NoteEditor;

impl NoteEditor<'_> {
    pub(crate) fn handle_visual(&mut self, key: KeyEvent) -> bool {
        match key.code {
            KeyCode::Esc => {
                self.enter_normal();
                true
            }
            KeyCode::Char('h') | KeyCode::Left => {
                self.textarea.move_cursor(CursorMove::Back);
                true
            }
            KeyCode::Char('j') | KeyCode::Down => {
                self.textarea.move_cursor(CursorMove::Down);
                true
            }
            KeyCode::Char('k') | KeyCode::Up => {
                self.textarea.move_cursor(CursorMove::Up);
                true
            }
            KeyCode::Char('l') | KeyCode::Right => {
                self.textarea.move_cursor(CursorMove::Forward);
                true
            }
            KeyCode::Char('w') => {
                self.textarea.move_cursor(CursorMove::WordForward);
                true
            }
            KeyCode::Char('b') => {
                self.textarea.move_cursor(CursorMove::WordBack);
                true
            }
            KeyCode::Char('$') => {
                self.textarea.move_cursor(CursorMove::End);
                true
            }
            KeyCode::Char('0') => {
                self.textarea.move_cursor(CursorMove::Head);
                true
            }
            KeyCode::Char('G') => {
                self.textarea.move_cursor(CursorMove::Bottom);
                true
            }
            KeyCode::Char('g') => {
                self.textarea.move_cursor(CursorMove::Top);
                true
            }
            KeyCode::Char('y') => {
                self.textarea.copy();
                self.sync_system_clipboard();
                self.enter_normal();
                "yanked".clone_into(&mut self.status);
                true
            }
            KeyCode::Char('d' | 'x') => {
                self.textarea.cut();
                self.sync_system_clipboard();
                self.enter_normal();
                self.dirty = true;
                true
            }
            KeyCode::Char('>') => {
                self.indent_selection();
                self.enter_normal();
                true
            }
            KeyCode::Char('<') => {
                self.unindent_selection();
                self.enter_normal();
                true
            }
            _ => false,
        }
    }
}
