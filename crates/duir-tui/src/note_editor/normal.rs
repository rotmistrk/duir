use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use tui_textarea::CursorMove;

use super::NoteEditor;

impl NoteEditor<'_> {
    #[allow(clippy::too_many_lines)]
    pub(crate) fn handle_normal(&mut self, key: KeyEvent) -> bool {
        if key.modifiers.contains(KeyModifiers::CONTROL) {
            return self.handle_normal_ctrl(key);
        }

        // Pending char: r, f, F, t, T wait for next character
        if let Some(pending) = self.pending_char {
            self.pending_char = None;
            if let KeyCode::Char(ch) = key.code {
                return self.execute_pending_char(pending, ch);
            }
            self.pending_count = None;
            return false;
        }

        // Numeric prefix (1-9 starts, 0 extends)
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

        // Pending operator: d, c, y + motion
        if let Some(op) = self.pending_op {
            return self.handle_operator_motion(op, key);
        }

        self.handle_normal_key(key)
    }

    fn handle_normal_ctrl(&mut self, key: KeyEvent) -> bool {
        match key.code {
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
            KeyCode::Char('f') => {
                let n = self.page_full();
                for _ in 0..n {
                    self.textarea.move_cursor(CursorMove::Down);
                }
                true
            }
            KeyCode::Char('b') => {
                let n = self.page_full();
                for _ in 0..n {
                    self.textarea.move_cursor(CursorMove::Up);
                }
                true
            }
            KeyCode::Char('r') => {
                self.textarea.redo();
                self.dirty = true;
                true
            }
            _ => false,
        }
    }

    fn handle_operator_motion(&mut self, op: char, key: KeyEvent) -> bool {
        self.pending_op = None;
        let motion = match key.code {
            KeyCode::Char('d') if op == 'd' => {
                let n = self.count();
                self.delete_lines(n);
                return true;
            }
            KeyCode::Char('c') if op == 'c' => {
                let n = self.count();
                self.delete_lines(n);
                self.enter_insert();
                return true;
            }
            KeyCode::Char('y') if op == 'y' => {
                let n = self.count();
                self.yank_lines(n);
                return true;
            }
            KeyCode::Char('w' | 'e') => CursorMove::WordForward,
            KeyCode::Char('b') => CursorMove::WordBack,
            KeyCode::Char('$') => CursorMove::End,
            KeyCode::Char('0' | '^') => CursorMove::Head,
            _ => {
                self.pending_count = None;
                return false;
            }
        };

        let n = self.count();
        // Select from cursor to motion target, then cut/yank
        self.textarea.start_selection();
        for _ in 0..n {
            self.textarea.move_cursor(motion);
        }
        // For ^ motion, move to first non-blank after Head
        if key.code == KeyCode::Char('^') {
            self.move_to_first_nonblank();
        }

        match op {
            'd' => {
                self.textarea.cut();
                self.dirty = true;
            }
            'c' => {
                self.textarea.cut();
                self.dirty = true;
                self.enter_insert();
            }
            'y' => {
                self.textarea.copy();
                self.textarea.cancel_selection();
                self.sync_system_clipboard();
            }
            _ => {
                self.textarea.cancel_selection();
            }
        }
        true
    }
}
