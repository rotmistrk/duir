//! `TreeTableSource` implementation for `TodoTreeData` — badge + timestamp columns.

use txv_core::cell::{Attrs, Style};
use txv_widgets::tree_table_source::TreeTableSource;
use txv_widgets::tree_view::TreeData;

use super::data::TodoTreeData;
use super::model::Completion;

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
}
