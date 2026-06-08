//! `NoteView` — vi-modal note editor using `txv_edit::view::EditorView`.

use txv_core::prelude::*;
use txv_edit::view::EditorView;

use crate::todo_tree::model::TreePath;

/// Command: save note content. Payload: `(TreePath, String)`.
pub const CM_NOTE_SAVE: CommandId = txv_core::commands::CM_TXV_MAX + 12;

/// Note editor backed by txv-edit's `EditorView`.
/// Wraps `EditorView`, adds path tracking and save-on-leave.
pub struct NoteView {
    inner: EditorView,
    path: Option<TreePath>,
}

impl NoteView {
    pub fn new() -> Self {
        Self {
            inner: EditorView::new(),
            path: None,
        }
    }

    /// Load note content for a given tree path.
    pub fn load(&mut self, path: TreePath, content: &str) {
        self.save_current();
        self.path = Some(path);
        self.inner.set_content(content, "md");
    }

    /// Force save current content regardless of dirty state.
    fn save_current(&mut self) {
        let Some(path) = self.path.clone() else { return };
        let content = self.inner.content();
        self.inner
            .state_mut()
            .put_command(CM_NOTE_SAVE, Some(Box::new((path, content))));
    }
}

impl View for NoteView {
    delegate_view!(inner, override { unselect });

    fn unselect(&mut self) {
        self.save_current();
        self.inner.unselect();
    }

    fn as_any_mut(&mut self) -> Option<&mut dyn std::any::Any> {
        Some(self)
    }
}
