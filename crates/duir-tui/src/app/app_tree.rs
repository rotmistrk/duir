use duir_core::stats::compute_stats;
use duir_core::{Completion, TodoItem};

use super::{App, FocusState, LoadedFile, TreeRow, TreeRowFlags};

impl App {
    pub(crate) fn rebuild_rows_raw(&mut self) {
        self.editor_cache.clear();

        self.rows.clear();
        for fi in 0..self.files.len() {
            let Some(file) = self.files.get(fi) else { continue };
            let mut flags = TreeRowFlags::default();
            flags.set_is_file_root(true);
            flags.set_expanded(!file.data.items.is_empty());
            flags.set_has_children(!file.data.items.is_empty());
            self.rows.push(TreeRow {
                path: vec![],
                depth: 0,
                title: file.name.clone(),
                completed: Completion::Open,
                flags,
                stats_text: String::new(),
                file_index: fi,
                file_id: file.id,
                file_source: Some(file.source),
            });
            let items: Vec<(usize, TodoItem)> = self
                .files
                .get(fi)
                .map(|f| {
                    f.data
                        .items
                        .iter()
                        .enumerate()
                        .map(|(i, item)| (i, item.clone()))
                        .collect()
                })
                .unwrap_or_default();
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

        let file_id = self.files.get(file_index).map_or(super::FileId(0), |f| f.id);
        let mut flags = TreeRowFlags::default();
        flags.set_important(item.important);
        flags.set_expanded(expanded);
        flags.set_has_children(!item.items.is_empty() || item.is_locked());
        flags.set_encrypted(item.is_encrypted());
        flags.set_locked(item.is_locked());
        flags.set_has_encrypted_children(has_enc_children);
        flags.set_is_kiron(item.is_kiron());
        flags.set_kiro_active(
            item.is_kiron() && {
                let node_id = item.id.clone();
                self.active_kirons.contains_key(&(file_id, node_id))
            },
        );

        self.rows.push(TreeRow {
            path: path.to_vec(),
            depth,
            title: item.title.clone(),
            completed: item.completed.clone(),
            flags,
            stats_text,
            file_index,
            file_id,
            file_source: None,
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
            .position(|r| r.file_index == file_index && !r.flags.is_file_root() && r.path == path)
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
