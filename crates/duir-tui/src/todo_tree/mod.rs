//! `TodoTreeView` — the main tree view for duir, wrapping `TreeView<TodoTreeData>`.

use std::path::Path;

use txv_core::prelude::*;
use txv_widgets::tree_view::TreeData;

mod badges;
pub mod data;
mod edit_keys;
mod flat_node;
pub mod handle;
pub mod model;
mod source;
mod timestamps;

use data::TodoTreeData;
use handle::HandleAction;

/// Pending crypto operation.
struct CryptoPending {
    path: model::TreePath,
    mode: handle::CryptoMode,
    passphrase: String,
}

/// The duir todo tree view.
///
/// Uses `TreeTableView` with a 5-char badge column for rendering.
/// Inline editing overlays text on the cursor row.
pub struct TodoTreeView {
    inner: txv_widgets::TreeTableView<TodoTreeData>,
    editing_row: Option<usize>,
    editing_text: String,
    filter_active: bool,
    crypto_pending: Option<CryptoPending>,
    prev_cursor: usize,
}

impl TodoTreeView {
    /// Access the underlying data mutably (for handler use).
    #[allow(clippy::missing_const_for_fn)]
    pub fn data_mut(&mut self) -> &mut TodoTreeData {
        &mut self.inner.data
    }

    /// Get the current cursor position.
    pub const fn cursor(&self) -> usize {
        self.inner.cursor
    }

    /// Set the cursor position.
    pub fn set_cursor(&mut self, pos: usize) {
        self.inner.set_cursor(pos);
    }

    /// Whether timestamps are showing.
    pub const fn show_timestamps(&self) -> bool {
        self.inner.data.show_timestamps
    }

    /// Enable timestamps (for session restore).
    pub fn toggle_timestamps_on(&mut self) {
        if !self.inner.data.show_timestamps {
            self.toggle_timestamps();
        }
    }
    pub fn new(root: &Path) -> Self {
        let file_path = root.join(".duir").join("todo.todo.json");
        let data = TodoTreeData::new(&file_path);
        Self {
            inner: txv_widgets::TreeTableView::new(data, &[5]),
            editing_row: None,
            editing_text: String::new(),
            filter_active: false,
            crypto_pending: None,
            prev_cursor: usize::MAX,
        }
    }

    fn start_edit(&mut self) {
        let row = self.inner.cursor;
        if row >= self.inner.data.visible_count() {
            return;
        }
        let id = self.inner.data.visible_id(row);
        self.editing_text = self.inner.data.label(id).to_owned();
        self.editing_row = Some(row);
        self.inner.state.mark_dirty();
    }

    fn commit_edit(&mut self) {
        if let Some(row) = self.editing_row.take() {
            self.inner.data.update_title(row, self.editing_text.clone());
        }
        self.editing_text.clear();
        self.inner.state.mark_dirty();
    }

    fn cancel_edit(&mut self) {
        self.editing_row = None;
        self.editing_text.clear();
        self.inner.state.mark_dirty();
    }

    fn start_filter(&mut self) {
        self.filter_active = true;
        self.inner.state.mark_dirty();
    }

    fn commit_filter(&mut self) {
        self.filter_active = false;
        self.inner.state.mark_dirty();
    }

    fn cancel_filter(&mut self) {
        self.filter_active = false;
        self.inner.data.filter_text.clear();
        self.inner.data.rebuild_flat();
        self.inner.cursor = 0;
        self.inner.state.mark_dirty();
    }

    fn toggle_timestamps(&mut self) {
        self.inner.data.show_timestamps = !self.inner.data.show_timestamps;
        let widths: &[u16] = if self.inner.data.show_timestamps {
            &[5, 5, 5, 5]
        } else {
            &[5]
        };
        self.inner.set_col_widths(widths);
        self.inner.state.mark_dirty();
    }

    fn apply_action(&mut self, action: &HandleAction) {
        match action {
            HandleAction::Stay => {}
            HandleAction::MoveTo(row) => self.inner.cursor = *row,
            HandleAction::EditNew(row) => {
                self.inner.cursor = *row;
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
        self.inner.state.mark_dirty();
    }
}

impl View for TodoTreeView {
    delegate_view!(inner, override { title, handle, draw });

    fn title(&self) -> &'static str {
        "Todo"
    }

    fn can_close(&self) -> CloseResult {
        CloseResult::Denied("permanent tab".to_string())
    }

    fn as_any_mut(&mut self) -> Option<&mut dyn std::any::Any> {
        Some(self)
    }

    fn draw(&mut self) {
        let b = self.inner.state.bounds();
        if b.w == 0 || b.h == 0 {
            return;
        }

        if self.inner.data.visible_count() == 0 {
            let dim = Style {
                attrs: Attrs {
                    dim: true,
                    ..Attrs::default()
                },
                ..Style::default()
            };
            self.inner.state.buffer_mut().fill(' ', Style::default());
            self.inner
                .state
                .buffer_mut()
                .print(0, 0, "  (empty — press 'n' to add)", dim);
            return;
        }

        self.inner.draw();

        // Overlay edit text on cursor row if editing
        if let Some(row) = self.editing_row {
            let scroll_offset = self.inner.scroll.offset;
            if row >= scroll_offset && (row - scroll_offset) < b.h as usize {
                let y = u16::try_from(row - scroll_offset).unwrap_or(0);
                let id = self.inner.data.visible_id(row);
                let depth = self.inner.data.depth(id);
                let x = u16::try_from(depth * 2 + 2).unwrap_or(0);
                let style = Style {
                    attrs: Attrs {
                        underline: true,
                        ..Attrs::default()
                    },
                    ..Style::default()
                };
                let w = b.w.saturating_sub(x);
                self.inner.state.buffer_mut().hline(x, y, w, ' ', style);
                self.inner.state.buffer_mut().print(x, y, &self.editing_text, style);
            }
        }

        // Crypto passphrase prompt on last line
        if self.is_crypto_active() {
            let y = b.h.saturating_sub(1);
            let style = Style {
                attrs: Attrs {
                    bold: true,
                    ..Attrs::default()
                },
                ..Style::default()
            };
            self.inner.state.buffer_mut().hline(0, y, b.w, ' ', style);
            let prompt = self.crypto_prompt_display();
            let mask = "*".repeat(self.crypto_mask_len());
            let display = format!("{prompt}{mask}");
            self.inner.state.buffer_mut().print(0, y, &display, style);
        }
    }

    fn handle(&mut self, event: &Event) -> HandleResult {
        if matches!(event, Event::Tick) {
            if self.inner.data.reload_if_changed() {
                self.inner.state.mark_dirty();
            }
            return HandleResult::Ignored;
        }

        let Event::Key(key) = event else {
            return HandleResult::Ignored;
        };

        if self.is_crypto_active() {
            return self.handle_crypto_key(key);
        }

        if self.filter_active {
            return self.handle_filter_key(key);
        }

        if self.editing_row.is_some() {
            return self.handle_edit_key(key);
        }

        // Clear filter on Esc
        if key.code == KeyCode::Esc && !self.inner.data.filter_text.is_empty() {
            self.inner.data.filter_text.clear();
            self.inner.data.rebuild_flat();
            self.inner.cursor = 0;
            self.inner.state.mark_dirty();
            return HandleResult::Consumed;
        }

        if key.code == KeyCode::Char('n') && self.inner.data.visible_count() == 0 {
            self.inner.data.add_first_item();
            self.inner.state.mark_dirty();
            return HandleResult::Consumed;
        }

        if key.code == KeyCode::Char('e') && self.inner.data.visible_count() > 0 {
            self.start_edit();
            return HandleResult::Consumed;
        }

        if key.code == KeyCode::Char('D') {
            self.toggle_timestamps();
            return HandleResult::Consumed;
        }

        let cursor = self.inner.cursor;
        if self.inner.data.visible_count() > 0
            && let Some(action) = handle::handle_todo_key(key, &mut self.inner.data, cursor)
        {
            self.apply_action(&action);
            self.emit_note_if_cursor_changed();
            return HandleResult::Consumed;
        }

        let result = self.inner.handle(event);
        self.emit_note_if_cursor_changed();
        result
    }
}
