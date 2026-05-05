use duir_core::{Completion, TodoItem};

use super::{App, FocusState, LoadedFile};

impl App {
    pub fn start_editing(&mut self) {
        if let Some(row) = self.current_row() {
            if row.flags.is_file_root() {
                return;
            }
            let buffer = row.title.clone();
            let cursor = buffer.len();
            self.state = FocusState::EditingTitle {
                buffer,
                cursor,
                select_all: true,
            };
        }
    }

    pub fn cancel_editing(&mut self) {
        if matches!(self.state, FocusState::EditingTitle { .. }) {
            self.state = FocusState::Tree;
        }
    }

    #[must_use]
    pub fn has_unsaved(&self) -> bool {
        self.files.iter().any(LoadedFile::is_modified)
    }

    pub fn toggle_completed(&mut self) {
        if let Some(row) = self.rows.get(self.cursor) {
            if row.flags.is_file_root() {
                return;
            }
            let fi = row.file_index;
            let path = row.path.clone();
            let Some(file) = self.files.get_mut(fi) else { return };
            if let Some(item) = duir_core::tree_ops::get_item_mut(&mut file.data, &path) {
                item.completed = match item.completed {
                    Completion::Done => Completion::Open,
                    _ => Completion::Done,
                };
            }
            self.mark_modified(fi, &path);
            if let Some(file) = self.files.get_mut(fi) {
                for item in &mut file.data.items {
                    duir_core::stats::update_completion(item);
                }
            }
            self.rebuild_rows();
        }
    }

    pub fn toggle_important(&mut self) {
        if let Some(row) = self.rows.get(self.cursor) {
            if row.flags.is_file_root() {
                return;
            }
            let fi = row.file_index;
            let path = row.path.clone();
            let Some(file) = self.files.get_mut(fi) else { return };
            if let Some(item) = duir_core::tree_ops::get_item_mut(&mut file.data, &path) {
                item.important = !item.important;
            }
            self.mark_modified(fi, &path);
            self.rebuild_rows();
        }
    }

    pub fn new_sibling(&mut self) {
        if let Some(row) = self.rows.get(self.cursor).cloned() {
            if row.flags.is_file_root() {
                return;
            }
            let fi = row.file_index;
            let mut new_path = row.path.clone();
            if let Some(last) = new_path.last_mut() {
                *last += 1;
            }
            let new_item = TodoItem::new("<new task>");
            let Some(file) = self.files.get_mut(fi) else { return };
            if duir_core::tree_ops::add_sibling(&mut file.data, &row.path, new_item).is_ok() {
                for item in &mut file.data.items {
                    duir_core::stats::update_completion(item);
                }
                self.mark_modified(fi, &new_path);
                self.rebuild_rows();
                self.navigate_to(fi, &new_path);
                self.start_editing();
            }
        }
    }

    pub fn new_child(&mut self) {
        if let Some(row) = self.rows.get(self.cursor).cloned() {
            let fi = row.file_index;
            let Some(file) = self.files.get_mut(fi) else { return };
            if row.flags.is_file_root() {
                let child_idx = file.data.items.len();
                file.data.items.push(TodoItem::new("<new task>"));
                self.mark_modified(fi, &[child_idx]);
                self.rebuild_rows();
                self.navigate_to(fi, &[child_idx]);
                self.start_editing();
                return;
            }
            let child_idx = duir_core::tree_ops::get_item(&file.data, &row.path).map_or(0, |item| item.items.len());
            let mut new_path = row.path.clone();
            new_path.push(child_idx);
            let new_item = TodoItem::new("<new task>");
            if duir_core::tree_ops::add_child(&mut file.data, &row.path, new_item).is_ok() {
                if let Some(item) = duir_core::tree_ops::get_item_mut(&mut file.data, &row.path) {
                    item.folded = false;
                }
                for item in &mut file.data.items {
                    duir_core::stats::update_completion(item);
                }
                self.mark_modified(fi, &new_path);
                self.rebuild_rows();
                self.navigate_to(fi, &new_path);
                self.start_editing();
            }
        }
    }

    pub fn delete_current(&mut self) {
        if let Some(row) = self.rows.get(self.cursor).cloned() {
            if row.flags.is_file_root() {
                "Cannot delete file root from tree".clone_into(&mut self.status_message);
                return;
            }
            let fi = row.file_index;
            let Some(file) = self.files.get(fi) else { return };
            if let Some(item) = duir_core::tree_ops::get_item(&file.data, &row.path) {
                let needs_confirm = !item.items.is_empty() || item.completed != Completion::Done;
                if needs_confirm {
                    self.flags.set_pending_delete(true);
                    let reason = if item.items.is_empty() {
                        "incomplete task"
                    } else {
                        "branch with children"
                    };
                    self.set_status(&format!("Delete {reason}? y/n"), super::StatusLevel::Warning);
                    return;
                }
            }
            self.force_delete_current();
        }
    }

    pub fn force_delete_current(&mut self) {
        if let Some(row) = self.rows.get(self.cursor).cloned() {
            if row.flags.is_file_root() {
                return;
            }
            let fi = row.file_index;
            let Some(file) = self.files.get_mut(fi) else { return };
            if duir_core::tree_ops::remove_item(&mut file.data, &row.path).is_ok() {
                for item in &mut file.data.items {
                    duir_core::stats::update_completion(item);
                }
                self.mark_modified(fi, &row.path);
                self.rebuild_rows();
                self.status_message.clear();
            }
        }
    }
}
