//! `NoteView` 2014 placeholder using `TextArea` until txv-edit `EditorView` API is finalized.

use txv_core::prelude::*;
use txv_widgets::TextArea;

use crate::todo_tree::model::TreePath;

/// Command emitted when note content changes (payload: `(TreePath, String)`).
pub const CM_NOTE_SAVE: CommandId = txv_core::commands::CM_TXV_MAX + 12;

/// Note view — currently read-only display. Will use txv-edit Editor once API stable.
pub struct NoteView {
    inner: TextArea,
    path: Option<TreePath>,
}

impl NoteView {
    pub fn new() -> Self {
        Self {
            inner: TextArea::new(),
            path: None,
        }
    }

    /// Load note content for a given tree path.
    pub fn load(&mut self, path: TreePath, content: &str) {
        self.path = Some(path);
        self.inner.set_content(content);
    }
}

impl View for NoteView {
    delegate_view!(inner, override {});
}
