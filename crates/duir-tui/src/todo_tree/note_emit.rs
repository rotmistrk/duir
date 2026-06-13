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

    #[allow(clippy::needless_pass_by_ref_mut)] // put_command needs &mut group
    pub(super) fn emit_note_now(&mut self) {
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
