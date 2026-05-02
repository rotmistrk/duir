use duir_core::stats::compute_stats;
use duir_core::{Completion, TodoItem};

use super::{App, FocusState, LoadedFile, TreeRow};

impl App {
    pub(crate) fn rebuild_rows_raw(&mut self) {
        self.editor_cache.clear();

        self.rows.clear();
        for fi in 0..self.files.len() {
            let file = &self.files[fi];
            self.rows.push(TreeRow {
                path: vec![],
                depth: 0,
                title: file.name.clone(),
                completed: Completion::Open,
                important: false,
                expanded: !file.data.items.is_empty(),
                has_children: !file.data.items.is_empty(),
                stats_text: String::new(),
                is_file_root: true,
                file_index: fi,
                file_id: file.id,
                encrypted: false,
                locked: false,
                has_encrypted_children: false,
                is_kiron: false,
            });
            let items: Vec<(usize, TodoItem)> = self.files[fi]
                .data
                .items
                .iter()
                .enumerate()
                .map(|(i, item)| (i, item.clone()))
                .collect();
            for (i, item) in &items {
                self.flatten_item(item, &[*i], 1, fi);
            }
        }
        if self.cursor >= self.rows.len() && !self.rows.is_empty() {
            self.cursor = self.rows.len() - 1;
        }
    }

    fn flatten_item(&mut self, item: &TodoItem, path: &[usize], depth: usize, file_index: usize) {
        let stats = compute_stats(item);
        let stats_text = if stats.total_leaves > 0 {
            format!("{}%", stats.percentage)
        } else {
            String::new()
        };

        let expanded = !item.folded && !item.items.is_empty() && !item.is_locked();
        let has_enc_children = item.items.iter().any(duir_core::crypto::has_encrypted_in_subtree);

        self.rows.push(TreeRow {
            path: path.to_vec(),
            depth,
            title: item.title.clone(),
            completed: item.completed.clone(),
            important: item.important,
            expanded,
            has_children: !item.items.is_empty() || item.is_locked(),
            stats_text,
            is_file_root: false,
            file_index,
            file_id: self.files[file_index].id,
            encrypted: item.is_encrypted(),
            locked: item.is_locked(),
            has_encrypted_children: has_enc_children,
            is_kiron: item.is_kiron(),
        });

        if expanded {
            for (i, child) in item.items.iter().enumerate() {
                let mut child_path = path.to_vec();
                child_path.push(i);
                self.flatten_item(child, &child_path, depth + 1, file_index);
            }
        }
    }

    pub(crate) fn navigate_to(&mut self, file_index: usize, path: &[usize]) {
        if let Some(pos) = self
            .rows
            .iter()
            .position(|r| r.file_index == file_index && !r.is_file_root && r.path == path)
        {
            self.cursor = pos;
            self.note_scroll = 0;
        }
    }

    pub const fn move_up(&mut self) {
        if self.cursor > 0 {
            self.cursor -= 1;
            self.note_scroll = 0;
        }
    }

    pub const fn move_down(&mut self) {
        if self.cursor + 1 < self.rows.len() {
            self.cursor += 1;
            self.note_scroll = 0;
        }
    }

    pub fn start_editing(&mut self) {
        if let Some(row) = self.current_row() {
            if row.is_file_root {
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
            if row.is_file_root {
                return;
            }
            let fi = row.file_index;
            let path = row.path.clone();
            if let Some(item) = duir_core::tree_ops::get_item_mut(&mut self.files[fi].data, &path) {
                item.completed = match item.completed {
                    Completion::Done => Completion::Open,
                    _ => Completion::Done,
                };
            }
            self.mark_modified(fi, &path);
            for item in &mut self.files[fi].data.items {
                duir_core::stats::update_completion(item);
            }
            self.rebuild_rows();
        }
    }

    pub fn toggle_important(&mut self) {
        if let Some(row) = self.rows.get(self.cursor) {
            if row.is_file_root {
                return;
            }
            let fi = row.file_index;
            let path = row.path.clone();
            if let Some(item) = duir_core::tree_ops::get_item_mut(&mut self.files[fi].data, &path) {
                item.important = !item.important;
            }
            self.mark_modified(fi, &path);
            self.rebuild_rows();
        }
    }

    pub fn new_sibling(&mut self) {
        if let Some(row) = self.rows.get(self.cursor).cloned() {
            if row.is_file_root {
                return;
            }
            let fi = row.file_index;
            let mut new_path = row.path.clone();
            if let Some(last) = new_path.last_mut() {
                *last += 1;
            }
            let new_item = TodoItem::new("<new task>");
            if duir_core::tree_ops::add_sibling(&mut self.files[fi].data, &row.path, new_item).is_ok() {
                for item in &mut self.files[fi].data.items {
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
            if row.is_file_root {
                let child_idx = self.files[fi].data.items.len();
                self.files[fi].data.items.push(TodoItem::new("<new task>"));
                self.mark_modified(fi, &[child_idx]);
                self.rebuild_rows();
                self.navigate_to(fi, &[child_idx]);
                self.start_editing();
                return;
            }
            let child_idx =
                duir_core::tree_ops::get_item(&self.files[fi].data, &row.path).map_or(0, |item| item.items.len());
            let mut new_path = row.path.clone();
            new_path.push(child_idx);
            let new_item = TodoItem::new("<new task>");
            if duir_core::tree_ops::add_child(&mut self.files[fi].data, &row.path, new_item).is_ok() {
                if let Some(item) = duir_core::tree_ops::get_item_mut(&mut self.files[fi].data, &row.path) {
                    item.folded = false;
                }
                for item in &mut self.files[fi].data.items {
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
            if row.is_file_root {
                "Cannot delete file root from tree".clone_into(&mut self.status_message);
                return;
            }
            let fi = row.file_index;
            if let Some(item) = duir_core::tree_ops::get_item(&self.files[fi].data, &row.path) {
                let needs_confirm = !item.items.is_empty() || item.completed != Completion::Done;
                if needs_confirm {
                    self.pending_delete = true;
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
            if row.is_file_root {
                return;
            }
            let fi = row.file_index;
            if duir_core::tree_ops::remove_item(&mut self.files[fi].data, &row.path).is_ok() {
                for item in &mut self.files[fi].data.items {
                    duir_core::stats::update_completion(item);
                }
                self.mark_modified(fi, &row.path);
                self.rebuild_rows();
                self.status_message.clear();
            }
        }
    }

    pub fn swap_up(&mut self) {
        if let Some(row) = self.rows.get(self.cursor).cloned() {
            if row.is_file_root {
                return;
            }
            let fi = row.file_index;
            if let Ok(new_path) = duir_core::tree_ops::swap_up(&mut self.files[fi].data, &row.path) {
                self.mark_modified(fi, &new_path);
                self.rebuild_rows();
                if let Some(pos) = self
                    .rows
                    .iter()
                    .position(|r| r.file_index == fi && !r.is_file_root && r.path == new_path)
                {
                    self.cursor = pos;
                }
            }
        }
    }

    pub fn swap_down(&mut self) {
        if let Some(row) = self.rows.get(self.cursor).cloned() {
            if row.is_file_root {
                return;
            }
            let fi = row.file_index;
            if let Ok(new_path) = duir_core::tree_ops::swap_down(&mut self.files[fi].data, &row.path) {
                self.mark_modified(fi, &new_path);
                self.rebuild_rows();
                if let Some(pos) = self
                    .rows
                    .iter()
                    .position(|r| r.file_index == fi && !r.is_file_root && r.path == new_path)
                {
                    self.cursor = pos;
                }
            }
        }
    }

    pub fn promote(&mut self) {
        if let Some(row) = self.rows.get(self.cursor).cloned() {
            if row.is_file_root {
                return;
            }
            let fi = row.file_index;
            if let Ok(new_path) = duir_core::tree_ops::promote(&mut self.files[fi].data, &row.path) {
                self.mark_modified(fi, &new_path);
                self.rebuild_rows();
                if let Some(pos) = self
                    .rows
                    .iter()
                    .position(|r| r.file_index == fi && !r.is_file_root && r.path == new_path)
                {
                    self.cursor = pos;
                }
            }
        }
    }

    pub fn demote(&mut self) {
        if let Some(row) = self.rows.get(self.cursor).cloned() {
            if row.is_file_root {
                return;
            }
            let fi = row.file_index;
            if let Ok(new_path) = duir_core::tree_ops::demote(&mut self.files[fi].data, &row.path) {
                self.mark_modified(fi, &new_path);
                self.rebuild_rows();
                if let Some(pos) = self
                    .rows
                    .iter()
                    .position(|r| r.file_index == fi && !r.is_file_root && r.path == new_path)
                {
                    self.cursor = pos;
                }
            }
        }
    }

    pub fn sort_children(&mut self) {
        if let Some(row) = self.rows.get(self.cursor).cloned() {
            if row.is_file_root {
                return;
            }
            let fi = row.file_index;
            if duir_core::tree_ops::sort_children(&mut self.files[fi].data, &row.path).is_ok() {
                self.mark_modified(fi, &row.path);
                self.rebuild_rows();
            }
        }
    }

    pub fn clone_subtree(&mut self) {
        if let Some(row) = self.rows.get(self.cursor).cloned() {
            if row.is_file_root {
                return;
            }
            let fi = row.file_index;
            let mut new_path = row.path.clone();
            if let Some(last) = new_path.last_mut() {
                *last += 1;
            }
            if duir_core::tree_ops::clone_subtree(&mut self.files[fi].data, &row.path).is_ok() {
                self.mark_modified(fi, &new_path);
                self.rebuild_rows();
                self.navigate_to(fi, &new_path);
            }
        }
    }
}
