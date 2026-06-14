//! Event handling and edit/filter/crypto logic for `TodoTreeView`.

use duir_core::crypto;
use txv_core::prelude::*;
use txv_widgets::input_line::InputLine;
use txv_widgets::tree_view::TreeData;

use super::TodoTreeView;
use super::handle::CryptoMode;
use super::model;

impl TodoTreeView {
    pub(super) fn handle_event(&mut self, event: &Event) -> HandleResult {
        if matches!(event, Event::Tick) {
            if self.prev_cursor == usize::MAX && self.inner().data().visible_count() > 0 {
                self.prev_cursor = self.inner().cursor();
                self.emit_note_now();
            }
            if self.inner_mut().data_mut().reload_if_changed() {
                self.emit_note_now();
            }
            return HandleResult::Ignored;
        }

        let Event::Key(key) = event else {
            return HandleResult::Ignored;
        };

        if self.confirm_delete {
            return self.handle_confirm_delete(key);
        }

        if self.crypto_pending.is_some() {
            return self.handle_crypto_key(key);
        }

        if self.filter_active {
            return self.handle_filter_key(key, event);
        }

        if self.editing_row.is_some() {
            let _result = self.group.dispatch(event);
            self.drain_edit_commands();
            return HandleResult::Consumed;
        }

        self.handle_normal_key(key, event)
    }

    fn handle_normal_key(&mut self, key: &KeyEvent, event: &Event) -> HandleResult {
        if key.code() == KeyCode::Esc && !self.inner().data().filter_text.is_empty() {
            self.inner_mut().data_mut().filter_text.clear();
            self.inner_mut().data_mut().rebuild_flat();
            self.inner_mut().set_cursor(0);
            return HandleResult::Consumed;
        }

        if key.code() == KeyCode::Char('n') && self.inner().data().visible_count() == 0 {
            self.inner_mut().data_mut().add_first_item();
            self.inner_mut().set_cursor(0);
            self.start_edit();
            return HandleResult::Consumed;
        }

        if key.code() == KeyCode::Char('e') && self.inner().data().visible_count() > 0 {
            self.start_edit();
            return HandleResult::Consumed;
        }

        if key.code() == KeyCode::Char('D') {
            self.toggle_timestamps();
            return HandleResult::Consumed;
        }

        if key.code() == KeyCode::Char('T') {
            let new_val = !self.connectors_visible;
            self.connectors_visible = new_val;
            self.inner_mut().set_show_connectors(new_val);
            return HandleResult::Consumed;
        }

        let cursor = self.inner().cursor();
        if self.inner().data().visible_count() > 0
            && let Some(action) = super::handle::handle_todo_key(key, self.inner_mut().data_mut(), cursor)
        {
            self.apply_action(&action);
            self.emit_note_if_cursor_changed();
            return HandleResult::Consumed;
        }
        // 'd' on items with children requires confirmation
        if key.code() == KeyCode::Char('d') {
            let id = self.inner().data().visible_id(self.inner().cursor());
            if self.inner().data().child_count(id) > 0 {
                self.confirm_delete = true;
                self.group.mark_dirty();
                return HandleResult::Consumed;
            }
        }
        // Enter on leaf node → focus Note panel
        if key.code() == KeyCode::Enter || key.code() == KeyCode::Right {
            let id = self.inner().data().visible_id(self.inner().cursor());
            if !self.inner().data().is_expandable(id) {
                use crate::slots::SlotId;
                use txv_widgets::tiled_workspace::commands::CM_TW_FOCUS_PANEL;
                self.group
                    .put_command(CM_TW_FOCUS_PANEL, Some(Box::new(SlotId::Center as u16)));
                return HandleResult::Consumed;
            }
        }
        // Forward to TreeTableView for j/k/arrows/expand/collapse/d/J/K/H/L
        let result = self.group.dispatch(event);
        self.emit_note_if_cursor_changed();
        result
    }

    fn handle_filter_key(&mut self, key: &KeyEvent, event: &Event) -> HandleResult {
        let _result = self.group.dispatch(event);
        if let Some(input) = self
            .group
            .child_mut(1)
            .and_then(|c| c.as_any_mut())
            .and_then(|a| a.downcast_mut::<InputLine>())
        {
            self.inner_mut().data_mut().filter_text = input.text().to_string();
        }
        self.inner_mut().data_mut().rebuild_flat();
        self.inner_mut().set_cursor(0);
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
        if let Some(item) = model::get_item_mut(&mut self.inner_mut().data_mut().file, &pending.path) {
            let result = match pending.mode {
                CryptoMode::Encrypt => crypto::encrypt_item(item, &pending.passphrase),
                CryptoMode::Decrypt => crypto::decrypt_item(item, &pending.passphrase),
            };
            if let Err(e) = result {
                log::warn!("crypto: {e}");
            }
        }
        self.inner_mut().data_mut().save();
        self.inner_mut().data_mut().rebuild_flat();
    }

    // --- Edit ---

    pub(super) fn start_edit(&mut self) {
        let row = self.inner().cursor();
        if row >= self.inner().data().visible_count() {
            return;
        }
        let id = self.inner().data().visible_id(row);
        let label = self.inner().data().label(id).to_owned();
        let mut input = InputLine::new()
            .with_command(CM_OK)
            .with_clipboard(self.clipboard.clone());
        input.set_text(&label);
        input.select_all();
        let sink = self.child_sink.clone();
        self.group.insert(Box::new(input));
        self.group.set_focused_index(1);
        if let Some(child) = self.group.child_mut(1) {
            child.set_sink(sink);
            child.select();
        }
        self.editing_row = Some(row);
        self.layout_edit_child();
    }

    pub(super) fn start_filter(&mut self) {
        let mut input = InputLine::new()
            .with_command(CM_OK)
            .with_clipboard(self.clipboard.clone());
        input.set_text(&self.inner().data().filter_text.clone());
        let sink = self.child_sink.clone();
        self.group.insert(Box::new(input));
        self.group.set_focused_index(1);
        if let Some(child) = self.group.child_mut(1) {
            child.set_sink(sink);
            child.select();
        }
        self.filter_active = true;
        self.layout_edit_child();
    }

    fn commit_filter(&mut self) {
        self.remove_input_line();
        self.filter_active = false;
    }

    fn cancel_filter(&mut self) {
        self.remove_input_line();
        self.filter_active = false;
        self.inner_mut().data_mut().filter_text.clear();
        self.inner_mut().data_mut().rebuild_flat();
        self.inner_mut().set_cursor(0);
    }

    fn remove_input_line(&mut self) {
        if self.group.child_count() > 1 {
            self.group.remove(1);
        }
        self.group.set_focused_index(0);
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
                        self.inner_mut().data_mut().update_title(row, text);
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
}
