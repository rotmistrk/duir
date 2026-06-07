//! Edit, filter, and crypto key handling for `TodoTreeView`.

use duir_core::crypto;
use txv_core::prelude::*;
use txv_widgets::tree_view::TreeData;

use super::TodoTreeView;
use super::handle::CryptoMode;
use super::model;

impl TodoTreeView {
    pub(super) fn handle_edit_key(&mut self, key: &KeyEvent) -> HandleResult {
        match key.code {
            KeyCode::Enter => self.commit_edit(),
            KeyCode::Esc => self.cancel_edit(),
            KeyCode::Backspace => {
                self.editing_text.pop();
            }
            KeyCode::Char(c) => self.editing_text.push(c),
            _ => {}
        }
        self.inner.state.mark_dirty();
        HandleResult::Consumed
    }

    pub(super) fn handle_filter_key(&mut self, key: &KeyEvent) -> HandleResult {
        match key.code {
            KeyCode::Enter => self.commit_filter(),
            KeyCode::Esc => self.cancel_filter(),
            KeyCode::Backspace => {
                self.inner.data.filter_text.pop();
            }
            KeyCode::Char(c) => self.inner.data.filter_text.push(c),
            _ => {}
        }
        self.inner.data.rebuild_flat();
        self.inner.cursor = 0;
        self.inner.state.mark_dirty();
        HandleResult::Consumed
    }

    pub(super) fn handle_crypto_key(&mut self, key: &KeyEvent) -> HandleResult {
        let Some(pending) = self.crypto_pending.as_mut() else {
            return HandleResult::Ignored;
        };
        match key.code {
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
        self.inner.state.mark_dirty();
        HandleResult::Consumed
    }

    fn commit_crypto(&mut self) {
        let Some(pending) = self.crypto_pending.take() else {
            return;
        };
        if let Some(item) = model::get_item_mut(&mut self.inner.data.file, &pending.path) {
            let result = match pending.mode {
                CryptoMode::Encrypt => crypto::encrypt_item(item, &pending.passphrase),
                CryptoMode::Decrypt => crypto::decrypt_item(item, &pending.passphrase),
            };
            if let Err(e) = result {
                log::warn!("crypto: {e}");
            }
        }
        self.inner.data.save();
        self.inner.data.rebuild_flat();
    }

    /// Whether crypto prompt is active.
    pub(super) const fn is_crypto_active(&self) -> bool {
        self.crypto_pending.is_some()
    }

    /// Get masked passphrase display for draw.
    pub(super) const fn crypto_prompt_display(&self) -> &'static str {
        if self.crypto_pending.is_some() {
            "Passphrase: "
        } else {
            ""
        }
    }

    pub(super) fn crypto_mask_len(&self) -> usize {
        self.crypto_pending.as_ref().map_or(0, |p| p.passphrase.len())
    }
}

impl TodoTreeView {
    pub(super) fn emit_note_if_cursor_changed(&mut self) {
        let cursor = self.inner.cursor;
        if cursor == self.prev_cursor {
            return;
        }
        self.prev_cursor = cursor;
        if cursor >= self.inner.data.visible_count() {
            return;
        }
        let id = self.inner.data.visible_id(cursor);
        if let Some(path) = self.inner.data.path_at(id).cloned() {
            let note = model::get_item(&self.inner.data.file, &path).map_or_else(String::new, |item| item.note.clone());
            self.inner
                .state
                .put_command(crate::handler::CM_NOTE_LOAD, Some(Box::new((path, note))));
        }
    }
}
