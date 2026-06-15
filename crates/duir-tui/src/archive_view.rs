//! Archive/Trash view — read-only tree with restore (`r`) binding.

use std::path::Path;

use txv_core::prelude::*;
use txv_widgets::TreeTableView;
use txv_widgets::tree_view::TreeData;

use crate::todo_tree::data::TodoTreeData;
use crate::todo_tree::model;

/// Read-only tree view for Archive or Trash. Only supports navigation + restore.
pub struct ArchiveView {
    group: GroupState,
    label: &'static str,
}

impl ArchiveView {
    #[must_use]
    pub fn new(file_path: &Path, label: &'static str) -> Self {
        let data = TodoTreeData::new(file_path);
        let mut group = GroupState::default();
        group.insert(Box::new(TreeTableView::new(data, &[5])));
        Self { group, label }
    }

    fn inner_mut(&mut self) -> &mut TreeTableView<TodoTreeData> {
        self.group
            .child_mut(0)
            .and_then(|c| c.as_any_mut())
            .and_then(|a| a.downcast_mut())
            .unwrap_or_else(|| std::process::abort())
    }

    fn inner(&self) -> &TreeTableView<TodoTreeData> {
        self.group
            .child(0)
            .and_then(|c| c.as_any())
            .and_then(|a| a.downcast_ref())
            .unwrap_or_else(|| std::process::abort())
    }

    /// Remove item at cursor and return it (for restore).
    pub fn take_at_cursor(&mut self) -> Option<(Vec<usize>, duir_core::TodoItem)> {
        let cursor = self.inner().cursor();
        if cursor >= self.inner().data().visible_count() {
            return None;
        }
        let id = self.inner().data().visible_id(cursor);
        let path = self.inner().data().path_at(id)?.clone();
        let item = model::remove_item(&mut self.inner_mut().data_mut().file, &path)?;
        self.inner_mut().data_mut().save();
        self.inner_mut().data_mut().rebuild_flat();
        Some((path, item))
    }

    /// Reload data from disk if changed.
    pub fn reload_if_changed(&mut self) -> bool {
        self.inner_mut().data_mut().reload_if_changed()
    }
}

impl View for ArchiveView {
    delegate_group_state!(group, override { title, handle, draw, set_bounds, select, unselect });

    fn title(&self) -> &'static str {
        self.label
    }

    fn set_bounds(&mut self, r: Rect) {
        self.group.set_bounds(r);
        self.group.set_child_bounds(0, Rect::new(0, 0, r.w(), r.h()));
    }

    fn select(&mut self) {
        self.group.set_focused(true);
        self.group.mark_dirty();
        self.reload_if_changed();
        if let Some(child) = self.group.focused_child_mut() {
            child.select();
        }
    }

    fn unselect(&mut self) {
        self.group.set_focused(false);
        self.group.mark_dirty();
        if let Some(child) = self.group.focused_child_mut() {
            child.unselect();
        }
    }

    fn draw(&mut self) {
        let b = self.group.bounds();
        if b.w() == 0 || b.h() == 0 {
            return;
        }
        if self.inner().data().visible_count() == 0 {
            let dim = palette().style(StyleId::Dim);
            self.group.buffer_mut().fill(' ', Style::default());
            self.group.buffer_mut().print(0, 0, "  (empty)", dim);
        }
    }

    fn handle(&mut self, event: &Event) -> HandleResult {
        if matches!(event, Event::Tick) {
            self.reload_if_changed();
            return HandleResult::Ignored;
        }
        if let Event::Key(key) = event
            && key.code() == KeyCode::Char('r')
        {
            if let Some((_path, item)) = self.take_at_cursor() {
                self.group
                    .put_command(crate::handler::CM_RESTORE_ITEM, Some(Box::new(item)));
            }
            return HandleResult::Consumed;
        }
        // Forward nav keys to inner tree
        self.group.dispatch(event)
    }

    fn as_any_mut(&mut self) -> Option<&mut dyn std::any::Any> {
        Some(self)
    }
}
