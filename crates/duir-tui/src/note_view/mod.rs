//! `NoteView` — vi-modal note editor using `txv_edit::view::EditorView`.

use txv_core::prelude::*;
use txv_edit::view::EditorView;

use crate::todo_tree::model::TreePath;

/// Command emitted when note content changes (payload: `(TreePath, String)`).
pub const CM_NOTE_SAVE: CommandId = txv_core::commands::CM_TXV_MAX + 12;

/// Note editor backed by txv-edit's `EditorView`.
pub struct NoteView {
    group: GroupState,
    path: Option<TreePath>,
}

impl NoteView {
    pub fn new() -> Self {
        let mut group = GroupState::default();
        group.insert(Box::new(EditorView::new()));
        Self { group, path: None }
    }

    /// Load note content for a given tree path.
    pub fn load(&mut self, path: TreePath, content: &str) {
        self.save_if_dirty();
        self.path = Some(path);
        if let Some(ev) = self.editor_mut() {
            ev.set_content(content, "md");
        }
        self.group.mark_dirty();
    }

    fn save_if_dirty(&mut self) {
        let Some(ev) = self.editor() else { return };
        if !ev.is_dirty() {
            return;
        }
        let Some(path) = self.path.clone() else { return };
        let content = ev.content();
        self.group.put_command(CM_NOTE_SAVE, Some(Box::new((path, content))));
    }

    fn editor(&self) -> Option<&EditorView> {
        self.group.child(0)?.as_any()?.downcast_ref::<EditorView>()
    }

    fn editor_mut(&mut self) -> Option<&mut EditorView> {
        self.group.child_mut(0)?.as_any_mut()?.downcast_mut::<EditorView>()
    }
}

impl View for NoteView {
    delegate_group_state!(group, override { unselect, draw, handle, set_bounds });

    fn set_bounds(&mut self, r: Rect) {
        self.group.set_bounds(r);
        // Set child bounds to fill entire area
        if self.group.child_count() > 0 {
            self.group.set_child_bounds(0, Rect::new(0, 0, r.w(), r.h()));
        }
    }

    fn draw(&mut self) {
        if let Some(child) = self.group.child_mut(0) {
            child.draw();
        }
    }

    fn handle(&mut self, event: &Event) -> HandleResult {
        self.group.dispatch(event)
    }

    fn unselect(&mut self) {
        self.save_if_dirty();
        self.group.set_focused(false);
        self.group.mark_dirty();
    }

    fn as_any_mut(&mut self) -> Option<&mut dyn std::any::Any> {
        Some(self)
    }
}
