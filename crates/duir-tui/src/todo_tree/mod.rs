//! `TodoTreeView` — Group wrapping `TreeTableView` + `InputLine` for inline editing.

use std::path::Path;
use std::sync::Arc;

use txv_core::clipboard_ring::{ClipboardHandle, new_clipboard};
use txv_core::prelude::*;
use txv_widgets::TreeTableView;
use txv_widgets::input_line::InputLine;
use txv_widgets::tree_view::TreeData;

mod badges;
pub mod data;
mod draw;
mod edit_keys;
mod flat_node;
pub mod handle;
pub mod model;
mod source;
mod timestamps;

use data::TodoTreeData;
use handle::HandleAction;

/// Pending crypto operation.
pub(crate) struct CryptoPending {
    pub path: model::TreePath,
    pub mode: handle::CryptoMode,
    pub passphrase: String,
}

/// The duir todo tree view — Group hosting `TreeTableView` + `InputLine` overlay.
pub struct TodoTreeView {
    group: GroupState,
    pub(crate) inner: TreeTableView<TodoTreeData>,
    child_sink: EventSink,
    pub(crate) editing_row: Option<usize>,
    pub(crate) filter_active: bool,
    pub(crate) crypto_pending: Option<CryptoPending>,
    prev_cursor: usize,
    tick_count: u32,
    connectors_visible: bool,
    pub clipboard: ClipboardHandle,
}

impl TodoTreeView {
    pub fn new(root: &Path) -> Self {
        let file_path = root.join(".duir").join("todo.todo.json");
        let data = TodoTreeData::new(&file_path);
        Self {
            group: GroupState::default(),
            inner: TreeTableView::new(data, &[5]),
            child_sink: EventSink::new(),
            editing_row: None,
            filter_active: false,
            crypto_pending: None,
            prev_cursor: usize::MAX,
            tick_count: 0,
            connectors_visible: true,
            clipboard: new_clipboard(20),
        }
    }

    /// Access the underlying data mutably.
    pub fn data_mut(&mut self) -> &mut TodoTreeData {
        self.inner.data_mut()
    }

    pub fn cursor(&self) -> usize {
        self.inner.cursor()
    }

    pub fn set_cursor(&mut self, pos: usize) {
        self.inner.set_cursor(pos);
    }

    pub fn show_timestamps(&self) -> bool {
        self.inner.data().show_timestamps
    }

    pub fn toggle_timestamps_on(&mut self) {
        if !self.inner.data().show_timestamps {
            self.toggle_timestamps();
        }
    }

    pub const fn show_connectors(&self) -> bool {
        self.connectors_visible
    }

    pub fn set_show_connectors(&mut self, val: bool) {
        self.connectors_visible = val;
        self.inner.set_show_connectors(val);
    }

    fn toggle_timestamps(&mut self) {
        let new_val = !self.inner.data().show_timestamps;
        self.inner.data_mut().show_timestamps = new_val;
        let widths: &[u16] = if new_val { &[5, 5, 5, 5] } else { &[5] };
        self.inner.set_col_widths(widths);
        self.group.mark_dirty();
    }

    /// Start editing current item title via `InputLine`.
    fn start_edit(&mut self) {
        let row = self.inner.cursor();
        if row >= self.inner.data().visible_count() {
            return;
        }
        let id = self.inner.data().visible_id(row);
        let label = self.inner.data().label(id).to_owned();
        let mut input = InputLine::new()
            .with_command(CM_OK)
            .with_clipboard(self.clipboard.clone());
        input.set_text(&label);
        input.select_all();
        let sink = self.child_sink.clone();
        self.group.insert(Box::new(input));
        self.group.set_focused_index(0);
        if let Some(child) = self.group.child_mut(0) {
            child.set_sink(sink);
            child.select();
        }
        self.editing_row = Some(row);
        self.group.mark_dirty();
    }

    fn start_filter(&mut self) {
        let mut input = InputLine::new()
            .with_command(CM_OK)
            .with_clipboard(self.clipboard.clone());
        input.set_text(&self.inner.data_mut().filter_text.clone());
        let sink = self.child_sink.clone();
        self.group.insert(Box::new(input));
        self.group.set_focused_index(0);
        if let Some(child) = self.group.child_mut(0) {
            child.set_sink(sink);
            child.select();
        }
        self.filter_active = true;
        self.group.mark_dirty();
    }

    fn input_line_mut(&mut self) -> Option<&mut InputLine> {
        self.group
            .child_mut(0)
            .and_then(|c| c.as_any_mut()?.downcast_mut::<InputLine>())
    }

    fn remove_input_line(&mut self) {
        if self.group.child_count() > 0 {
            self.group.remove(0);
        }
    }

    fn apply_action(&mut self, action: &HandleAction) {
        match action {
            HandleAction::Stay => {}
            HandleAction::MoveTo(row) => self.inner.set_cursor(*row),
            HandleAction::EditNew(row) => {
                self.inner.set_cursor(*row);
                self.start_edit();
            }
            HandleAction::EnterFilter => self.start_filter(),
            HandleAction::CryptoPrompt(path, mode) => {
                self.crypto_pending = Some(CryptoPending {
                    path: path.clone(),
                    mode: match mode {
                        handle::CryptoMode::Encrypt => handle::CryptoMode::Encrypt,
                        handle::CryptoMode::Decrypt => handle::CryptoMode::Decrypt,
                    },
                    passphrase: String::new(),
                });
            }
        }
        self.group.mark_dirty();
    }
}

impl View for TodoTreeView {
    delegate_group_state!(group, override { title, handle, draw, select, unselect });

    fn title(&self) -> &'static str {
        "Todo"
    }

    fn can_close(&self) -> CloseResult {
        CloseResult::Denied("permanent tab".to_string())
    }

    fn as_any_mut(&mut self) -> Option<&mut dyn std::any::Any> {
        Some(self)
    }

    fn select(&mut self) {
        self.group.set_focused(true);
        self.group.mark_dirty();
        self.emit_note_if_cursor_changed();
    }

    fn unselect(&mut self) {
        self.group.set_focused(false);
        self.group.mark_dirty();
    }

    fn draw(&mut self) {
        self.draw_tree();
    }

    fn handle(&mut self, event: &Event) -> HandleResult {
        self.handle_event(event)
    }
}
