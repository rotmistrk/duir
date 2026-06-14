//! `TreeTableSource` implementation for `TodoTreeData` — columns + structural ops.

use txv_core::cell::{Attrs, Style};
use txv_widgets::tree_table_source::{AcceptAll, CellValidator, TreeTableSource};
use txv_widgets::tree_view::TreeData;

use super::data::TodoTreeData;
use super::model::{self, Completion, TodoItem};

impl TreeTableSource for TodoTreeData {
    fn visible_count(&self) -> usize {
        TreeData::visible_count(self)
    }

    fn label(&self, row: usize) -> &str {
        let id = self.visible_id(row);
        TreeData::label(self, id)
    }

    fn depth(&self, row: usize) -> usize {
        let id = self.visible_id(row);
        TreeData::depth(self, id)
    }

    fn is_expandable(&self, row: usize) -> bool {
        let id = self.visible_id(row);
        TreeData::is_expandable(self, id)
    }

    fn is_expanded(&self, row: usize) -> bool {
        let id = self.visible_id(row);
        TreeData::is_expanded(self, id)
    }

    fn toggle(&mut self, row: usize) {
        let id = self.visible_id(row);
        TreeData::toggle(self, id);
    }

    fn style(&self, row: usize) -> Style {
        let id = self.visible_id(row);
        TreeData::style(self, id)
    }

    fn column_count(&self) -> usize {
        if self.show_timestamps { 4 } else { 1 }
    }

    fn cell(&self, row: usize, col: usize) -> &str {
        let id = self.visible_id(row);
        match col {
            0 => self.badge_at(id),
            1..=3 if self.show_timestamps => self.timestamp_cell(id, col - 1),
            _ => "",
        }
    }

    fn cell_style(&self, row: usize, _col: usize) -> Style {
        let id = self.visible_id(row);
        let Some(item) = self.item_at(id) else {
            return Style::default();
        };
        if item.completed == Completion::Done {
            Style::default().with_attrs(Attrs::default().dim())
        } else {
            Style::default()
        }
    }

    fn column_validator(&self, col: usize) -> Option<&dyn CellValidator> {
        if col == 0 { Some(&AcceptAll) } else { None }
    }

    fn commit_edit(&mut self, row: usize, _col: usize, text: &str) {
        self.update_title(row, text.to_owned());
    }

    // --- Structural operations ---

    fn can_add_sibling(&self, _row: usize) -> bool {
        true
    }
    fn can_add_child(&self, _row: usize) -> bool {
        true
    }
    fn can_delete(&self, row: usize) -> bool {
        row < TreeData::visible_count(self)
    }
    fn can_swap_up(&self, row: usize) -> bool {
        row > 0
    }
    fn can_swap_down(&self, row: usize) -> bool {
        row + 1 < TreeData::visible_count(self)
    }
    fn can_promote(&self, _row: usize) -> bool {
        true
    }
    fn can_demote(&self, _row: usize) -> bool {
        true
    }

    fn add_sibling(&mut self, row: usize) -> Option<usize> {
        let id = self.visible_id(row);
        let path = self.path_at(id)?.clone();
        let new_item = TodoItem::new("");
        if !model::add_sibling(&mut self.file, &path, new_item) {
            return Some(row);
        }
        let mut new_path = path;
        if let Some(last) = new_path.last_mut() {
            *last += 1;
        }
        model::propagate_completion(&mut self.file, &new_path);
        self.save();
        self.rebuild_flat();
        self.row_for_path(&new_path).or(Some(row + 1))
    }

    fn add_child(&mut self, row: usize) -> Option<usize> {
        let id = self.visible_id(row);
        let path = self.path_at(id)?.clone();
        let child_idx = model::get_item(&self.file, &path).map_or(0, |item| item.items.len());
        if !model::add_child(&mut self.file, &path, TodoItem::new("")) {
            return Some(row);
        }
        if let Some(item) = model::get_item_mut(&mut self.file, &path) {
            item.folded = false;
        }
        let mut new_path = path;
        new_path.push(child_idx);
        model::propagate_completion(&mut self.file, &new_path);
        self.save();
        self.rebuild_flat();
        self.row_for_path(&new_path).or(Some(row + 1))
    }

    fn delete(&mut self, row: usize) -> Option<usize> {
        let id = self.visible_id(row);
        let path = self.path_at(id)?.clone();
        model::remove_item(&mut self.file, &path)?;
        model::propagate_completion(&mut self.file, &path);
        self.save();
        self.rebuild_flat();
        Some(row.min(TreeData::visible_count(self).saturating_sub(1)))
    }

    fn swap_up(&mut self, row: usize) -> Option<usize> {
        let id = self.visible_id(row);
        let path = self.path_at(id)?.clone();
        let new_path = model::swap_up(&mut self.file, &path)?;
        self.save();
        self.rebuild_flat();
        self.row_for_path(&new_path)
    }

    fn swap_down(&mut self, row: usize) -> Option<usize> {
        let id = self.visible_id(row);
        let path = self.path_at(id)?.clone();
        let new_path = model::swap_down(&mut self.file, &path)?;
        self.save();
        self.rebuild_flat();
        self.row_for_path(&new_path)
    }

    fn promote(&mut self, row: usize) -> Option<usize> {
        let id = self.visible_id(row);
        let path = self.path_at(id)?.clone();
        let new_path = model::promote(&mut self.file, &path)?;
        self.save();
        self.rebuild_flat();
        self.row_for_path(&new_path)
    }

    fn demote(&mut self, row: usize) -> Option<usize> {
        let id = self.visible_id(row);
        let path = self.path_at(id)?.clone();
        let new_path = model::demote(&mut self.file, &path)?;
        self.save();
        self.rebuild_flat();
        self.row_for_path(&new_path)
    }
}
