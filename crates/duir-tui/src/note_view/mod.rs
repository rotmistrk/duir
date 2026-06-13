//! `NoteView` — vi-modal note editor using `txv_edit::view::EditorView`.

use txv_core::prelude::*;
use txv_edit::view::EditorView;

use crate::todo_tree::model::TreePath;

/// Note editor backed by txv-edit's `EditorView`.
pub struct NoteView {
    inner: EditorView,
    path: Option<TreePath>,
}

impl Default for NoteView {
    fn default() -> Self {
        Self::new()
    }
}

impl NoteView {
    #[must_use]
    pub fn new() -> Self {
        Self {
            inner: EditorView::new(),
            path: None,
        }
    }

    pub fn load(&mut self, path: TreePath, content: &str) {
        self.path = Some(path);
        self.inner.set_content(content, "md");
    }

    pub fn content(&self) -> String {
        self.inner.content()
    }

    pub const fn path(&self) -> Option<&TreePath> {
        self.path.as_ref()
    }

    pub fn is_dirty(&self) -> bool {
        self.inner.is_dirty()
    }
}

impl View for NoteView {
    delegate_view!(inner, override { unselect });

    fn unselect(&mut self) {
        self.inner.unselect();
    }

    fn as_any_mut(&mut self) -> Option<&mut dyn std::any::Any> {
        Some(self)
    }
}
