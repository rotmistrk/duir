//! Edit, filter, crypto key handling + draw + event dispatch for `TodoTreeView`.

use duir_core::crypto;
use txv_core::prelude::*;
use txv_widgets::tree_view::TreeData;

use super::TodoTreeView;
use super::handle::CryptoMode;
use super::model;

use crate::handler::CM_NOTE_LOAD;

impl TodoTreeView {
    pub(super) fn handle_event(&mut self, event: &Event) -> HandleResult {
        if matches!(event, Event::Tick) {
            if self.inner.data_mut().reload_if_changed() {
                self.group.mark_dirty();
            }
            return HandleResult::Ignored;
        }

        let Event::Key(key) = event else {
            return HandleResult::Ignored;
        };

        if self.crypto_pending.is_some() {
            return self.handle_crypto_key(key);
        }

        if self.filter_active {
            return self.handle_filter_key(key, event);
        }

        if self.editing_row.is_some() {
            let _result = self.group.dispatch(event);
            self.drain_edit_commands();
            self.group.mark_dirty();
            return HandleResult::Consumed;
        }

        self.handle_normal_key(key, event)
    }

    fn handle_normal_key(&mut self, key: &KeyEvent, event: &Event) -> HandleResult {
        if key.code() == KeyCode::Esc && !self.inner.data().filter_text.is_empty() {
            self.inner.data_mut().filter_text.clear();
            self.inner.data_mut().rebuild_flat();
            self.inner.set_cursor(0);
            self.group.mark_dirty();
            return HandleResult::Consumed;
        }

        if key.code() == KeyCode::Char('n') && self.inner.data().visible_count() == 0 {
            self.inner.data_mut().add_first_item();
            self.group.mark_dirty();
            return HandleResult::Consumed;
        }

        if key.code() == KeyCode::Char('e') && self.inner.data().visible_count() > 0 {
            self.start_edit();
            return HandleResult::Consumed;
        }

        if key.code() == KeyCode::Char('D') {
            self.toggle_timestamps();
            return HandleResult::Consumed;
        }

        if key.code() == KeyCode::Char('T') {
            self.connectors_visible = !self.connectors_visible;
            self.inner.set_show_connectors(self.connectors_visible);
            self.group.mark_dirty();
            return HandleResult::Consumed;
        }

        let cursor = self.inner.cursor();
        if self.inner.data().visible_count() > 0
            && let Some(action) = super::handle::handle_todo_key(key, self.inner.data_mut(), cursor)
        {
            self.apply_action(&action);
            self.emit_note_if_cursor_changed();
            return HandleResult::Consumed;
        }

        let result = self.inner.handle(event);
        self.emit_note_if_cursor_changed();
        result
    }

    fn handle_filter_key(&mut self, key: &KeyEvent, event: &Event) -> HandleResult {
        let _result = self.group.dispatch(event);
        // Read filter text from InputLine
        if let Some(input) = self.input_line_mut() {
            self.inner.data_mut().filter_text = input.text().to_string();
        }
        self.inner.data_mut().rebuild_flat();
        self.inner.set_cursor(0);
        self.group.mark_dirty();
        if key.code() == KeyCode::Esc {
            self.cancel_filter();
        } else if key.code() == KeyCode::Enter {
            self.commit_filter();
        }
        HandleResult::Consumed
    }

    fn handle_crypto_key(&mut self, key: &KeyEvent) -> HandleResult {
        let Some(pending) = self.crypto_pending.as_mut() else {
            return HandleResult::Ignored;
        };
        match key.code() {
            KeyCode::Esc => {
                self.crypto_pending = None;
            }
            KeyCode::Enter => {
                self.commit_crypto();
            }
            KeyCode::Backspace => {
                pending.passphrase.pop();
            }
            KeyCode::Char(c) => pending.passphrase.push(c),
            _ => {}
        }
        self.group.mark_dirty();
        HandleResult::Consumed
    }

    fn commit_crypto(&mut self) {
        let Some(pending) = self.crypto_pending.take() else {
            return;
        };
        if let Some(item) = model::get_item_mut(&mut self.inner.data_mut().file, &pending.path) {
            let result = match pending.mode {
                CryptoMode::Encrypt => crypto::encrypt_item(item, &pending.passphrase),
                CryptoMode::Decrypt => crypto::decrypt_item(item, &pending.passphrase),
            };
            if let Err(e) = result {
                log::warn!("crypto: {e}");
            }
        }
        self.inner.data_mut().save();
        self.inner.data_mut().rebuild_flat();
    }

    fn commit_filter(&mut self) {
        self.remove_input_line();
        self.filter_active = false;
        self.group.mark_dirty();
    }

    fn cancel_filter(&mut self) {
        self.remove_input_line();
        self.filter_active = false;
        self.inner.data_mut().filter_text.clear();
        self.inner.data_mut().rebuild_flat();
        self.inner.set_cursor(0);
        self.group.mark_dirty();
    }

    fn drain_edit_commands(&mut self) {
        for ev in self.child_sink.drain() {
            if let Event::Command { id, data, .. } = ev {
                match id {
                    CM_OK => {
                        let text = data
                            .and_then(|d| d.downcast::<String>().ok())
                            .map_or_else(String::new, |s| *s);
                        let row = self.editing_row.take().unwrap_or(0);
                        self.remove_input_line();
                        self.inner.data_mut().update_title(row, text);
                        return;
                    }
                    CM_CANCEL => {
                        self.editing_row = None;
                        self.remove_input_line();
                        return;
                    }
                    _ => {}
                }
            }
        }
    }

    pub(super) fn emit_note_if_cursor_changed(&mut self) {
        let cursor = self.inner.cursor();
        if cursor == self.prev_cursor {
            return;
        }
        self.prev_cursor = cursor;
        if cursor >= self.inner.data().visible_count() {
            return;
        }
        let id = self.inner.data().visible_id(cursor);
        if let Some(path) = self.inner.data_mut().path_at(id).cloned() {
            let note =
                model::get_item(&self.inner.data_mut().file, &path).map_or_else(String::new, |item| item.note.clone());
            self.group.put_command(CM_NOTE_LOAD, Some(Box::new((path, note))));
        }
    }

    pub(super) const fn is_crypto_active(&self) -> bool {
        self.crypto_pending.is_some()
    }
}
