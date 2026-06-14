//! `NoteView` — vi-modal note editor using `txv_edit::view::EditorView`.
//!
//! Holds `EditorView` directly (typed). Uses `ViewState` for framework identity
//! and emits `CM_NOTE_SAVE` on unselect.

use txv_core::clipboard_ring::ClipboardHandle;
use txv_core::prelude::*;
use txv_edit::shared_register::new_register;
use txv_edit::view::EditorView;

use crate::handler::CM_NOTE_SAVE;
use crate::todo_tree::model::TreePath;

/// Note editor backed by txv-edit's `EditorView`.
pub struct NoteView {
    state: ViewState,
    editor: EditorView,
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
            state: ViewState::default(),
            editor: EditorView::new(),
            path: None,
        }
    }

    /// Wire the shared clipboard ring into the editor.
    pub fn set_clipboard(&mut self, clipboard: ClipboardHandle) {
        self.editor.editor_mut().set_shared_state(new_register(), clipboard);
    }

    pub fn load(&mut self, path: TreePath, content: &str) {
        self.path = Some(path);
        self.editor.set_content(content, "md");
    }

    pub fn content(&self) -> String {
        self.editor.content()
    }

    pub const fn path(&self) -> Option<&TreePath> {
        self.path.as_ref()
    }

    #[allow(dead_code)]
    pub fn is_dirty(&self) -> bool {
        self.editor.is_dirty()
    }
}

impl View for NoteView {
    fn view_id(&self) -> ViewId {
        self.state.id()
    }

    fn bounds(&self) -> Rect {
        self.state.bounds()
    }

    fn set_bounds(&mut self, r: Rect) {
        self.state.set_bounds(r);
        self.editor.set_bounds(r);
    }

    fn set_sink(&mut self, sink: EventSink) {
        self.state.set_sink(sink.clone());
        self.editor.set_sink(sink);
    }

    fn options(&self) -> ViewOptions {
        self.editor.options()
    }

    fn title(&self) -> &str {
        self.state.title()
    }

    fn needs_redraw(&self) -> bool {
        self.editor.needs_redraw()
    }

    fn mark_redrawn(&mut self) {
        self.editor.mark_redrawn();
    }

    fn select(&mut self) {
        self.editor.select();
    }

    fn unselect(&mut self) {
        self.editor.unselect();
        self.state.put_command(CM_NOTE_SAVE, None);
    }

    fn render(&mut self) -> bool {
        self.editor.render()
    }

    fn draw(&mut self) {}

    fn handle(&mut self, event: &Event) -> HandleResult {
        self.editor.handle(event)
    }

    fn cursor(&self) -> Option<CursorRequest> {
        self.editor.cursor()
    }

    fn buffer(&self) -> &Buffer {
        self.editor.buffer()
    }

    fn as_any_mut(&mut self) -> Option<&mut dyn std::any::Any> {
        Some(self)
    }
}
