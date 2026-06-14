//! Note emission and action dispatch for `TodoTreeView`.

use txv_widgets::tree_view::TreeData;

use super::TodoTreeView;
use super::handle::{CryptoMode, HandleAction};
use super::model;

use crate::handler::CM_NOTE_LOAD;

impl TodoTreeView {
    pub(super) fn apply_action(&mut self, action: &HandleAction) {
        match action {
            HandleAction::Stay => {}
            HandleAction::MoveTo(row) => self.inner_mut().set_cursor(*row),
            HandleAction::EditNew(row) => {
                self.inner_mut().set_cursor(*row);
                self.start_edit();
            }
            HandleAction::EnterFilter => self.start_filter(),
            HandleAction::CryptoPrompt(path, mode) => {
                self.crypto_pending = Some(super::CryptoPending {
                    path: path.clone(),
                    mode: match mode {
                        CryptoMode::Encrypt => CryptoMode::Encrypt,
                        CryptoMode::Decrypt => CryptoMode::Decrypt,
                    },
                    passphrase: String::new(),
                });
                self.group.mark_dirty();
            }
        }
    }

    pub(crate) fn emit_note_if_cursor_changed(&mut self) {
        let cursor = self.inner().cursor();
        if cursor == self.prev_cursor {
            return;
        }
        self.prev_cursor = cursor;
        self.emit_note_now();
    }

    pub(super) fn emit_note_now(&self) {
        let cursor = self.inner().cursor();
        if cursor >= self.inner().data().visible_count() {
            return;
        }
        let id = self.inner().data().visible_id(cursor);
        if let Some(path) = self.inner().data().path_at(id).cloned() {
            let note =
                model::get_item(&self.inner().data().file, &path).map_or_else(String::new, |item| item.note.clone());
            self.group.put_command(CM_NOTE_LOAD, Some(Box::new((path, note))));
        }
    }
}

impl TodoTreeView {
    pub(super) fn handle_confirm_delete(&mut self, key: &txv_core::event::KeyEvent) -> txv_core::view::HandleResult {
        use txv_core::event::KeyCode;
        use txv_core::view::HandleResult;
        self.confirm_delete = false;
        if let KeyCode::Char('y' | 'Y') = key.code() {
            let cursor = self.inner().cursor();
            let data = self.inner_mut().data_mut();
            if let Some(new_pos) = txv_widgets::tree_table_source::TreeTableSource::delete(data, cursor) {
                self.inner_mut().set_cursor(new_pos);
            }
        }
        self.group.mark_dirty();
        HandleResult::Consumed
    }
}

impl TodoTreeView {
    /// Position `InputLine` (child 1) at the correct screen location.
    pub(super) fn layout_edit_child(&mut self) {
        use txv_core::geometry::Rect;
        if self.group.child_count() <= 1 {
            return;
        }
        let b = self.group.bounds();
        let w = b.w();
        let h = b.h();
        if self.filter_active {
            let filter_row = h.saturating_sub(1);
            self.group
                .set_child_bounds(1, Rect::new(1, filter_row, w.saturating_sub(1), 1));
        } else if let Some(row) = self.editing_row {
            let scroll_offset = self.inner().scroll_offset();
            let draw_h = h as usize;
            if row < scroll_offset || (row - scroll_offset) >= draw_h {
                return;
            }
            let screen_y = u16::try_from(row - scroll_offset).unwrap_or(u16::MAX);
            let id = self.inner().data().visible_id(row);
            let depth = self.inner().data().depth(id);
            let indent = u16::try_from(depth * 2 + 2).unwrap_or(0);
            self.group
                .set_child_bounds(1, Rect::new(indent, screen_y, w.saturating_sub(indent), 1));
        }
    }
}
