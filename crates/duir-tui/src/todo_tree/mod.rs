//! `TodoTreeView` — Group hosting `TreeTableView` (child 0) + `InputLine` (child 1 when editing).

use std::path::Path;

use txv_core::clipboard_ring::{ClipboardHandle, new_clipboard};
use txv_core::prelude::*;
use txv_widgets::TreeTableView;
use txv_widgets::input_line::InputLine;

mod badges;
pub mod data;
mod draw;
mod edit;
mod flat_node;
pub mod handle;
pub mod model;
mod source;
mod timestamps;

use data::TodoTreeData;
use handle::HandleAction;

use crate::handler::CM_NOTE_LOAD;

/// Pending crypto operation.
pub(crate) struct CryptoPending {
    pub path: model::TreePath,
    pub mode: handle::CryptoMode,
    pub passphrase: String,
}

/// The duir todo tree view — Group with TreeTableView as child 0.
pub struct TodoTreeView {
    group: GroupState,
    child_sink: EventSink,
    pub(crate) editing_row: Option<usize>,
    pub(crate) filter_active: bool,
    pub(crate) crypto_pending: Option<CryptoPending>,
    prev_cursor: usize,
    connectors_visible: bool,
    pub clipboard: ClipboardHandle,
}

impl TodoTreeView {
    pub fn new(root: &Path) -> Self {
        let file_path = root.join(".duir").join("todo.todo.json");
        let data = TodoTreeData::new(&file_path);
        let mut group = GroupState::default();
        group.insert(Box::new(TreeTableView::new(data, &[5])));
        Self {
            group,
            child_sink: EventSink::new(),
            editing_row: None,
            filter_active: false,
            crypto_pending: None,
            prev_cursor: usize::MAX,
            connectors_visible: true,
            clipboard: new_clipboard(20),
        }
    }

    /// Typed access to TreeTableView (always child 0).
    pub(crate) fn inner(&self) -> &TreeTableView<TodoTreeData> {
        self.group
            .child(0)
            .and_then(|c| c.as_any())
            .and_then(|a| a.downcast_ref())
            .expect("child 0 is TreeTableView")
    }

    /// Typed mutable access to TreeTableView (always child 0).
    pub(crate) fn inner_mut(&mut self) -> &mut TreeTableView<TodoTreeData> {
        self.group
            .child_mut(0)
            .and_then(|c| c.as_any_mut())
            .and_then(|a| a.downcast_mut())
            .expect("child 0 is TreeTableView")
    }

    pub fn data_mut(&mut self) -> &mut TodoTreeData {
        self.inner_mut().data_mut()
    }

    pub fn cursor(&self) -> usize {
        self.inner().cursor()
    }

    pub fn set_cursor(&mut self, pos: usize) {
        self.inner_mut().set_cursor(pos);
    }

    pub fn show_timestamps(&self) -> bool {
        self.inner().data().show_timestamps
    }

    pub fn toggle_timestamps_on(&mut self) {
        if !self.inner().data().show_timestamps {
            self.toggle_timestamps();
        }
    }

    pub const fn show_connectors(&self) -> bool {
        self.connectors_visible
    }

    pub fn set_show_connectors(&mut self, val: bool) {
        self.connectors_visible = val;
        self.inner_mut().set_show_connectors(val);
    }

    fn toggle_timestamps(&mut self) {
        let new_val = !self.inner().data().show_timestamps;
        self.inner_mut().data_mut().show_timestamps = new_val;
        let widths: &[u16] = if new_val { &[5, 5, 5, 5] } else { &[5] };
        self.inner_mut().set_col_widths(widths);
    }
}

impl View for TodoTreeView {
    delegate_group_state!(group, override { title, handle, draw, select, unselect, set_bounds });

    fn title(&self) -> &'static str {
        "Todo"
    }

    fn set_bounds(&mut self, r: Rect) {
        self.group.set_bounds(r);
        let w = r.w();
        let h = r.h();
        let has_filter = self.filter_active || !self.inner().data().filter_text.is_empty();
        let draw_h = if has_filter { h.saturating_sub(1) } else { h };
        self.group.set_child_bounds(0, Rect::new(0, 0, w, draw_h));
    }

    fn select(&mut self) {
        self.group.set_focused(true);
        self.group.mark_dirty();
    }

    fn unselect(&mut self) {
        self.group.set_focused(false);
        self.group.mark_dirty();
    }

    fn can_close(&self) -> CloseResult {
        CloseResult::Denied("permanent tab".to_string())
    }

    fn as_any_mut(&mut self) -> Option<&mut dyn std::any::Any> {
        Some(self)
    }

    fn draw(&mut self) {
        self.draw_tree();
    }

    fn handle(&mut self, event: &Event) -> HandleResult {
        self.handle_event(event)
    }
}
