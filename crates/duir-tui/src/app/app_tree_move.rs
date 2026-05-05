use super::App;

// fi (file_index) is always set by rebuild_rows from 0..self.files.len(), so self.files[fi] is safe.
#[allow(clippy::indexing_slicing)]
impl App {
    pub fn swap_up(&mut self) {
        let Some(row) = self.rows.get(self.cursor).cloned() else {
            return;
        };
        if row.is_file_root {
            self.reorder_file_up();
            return;
        }
        let fi = row.file_index;
        if let Ok(new_path) = duir_core::tree_ops::swap_up(&mut self.files[fi].data, &row.path) {
            self.mark_modified(fi, &new_path);
            self.rebuild_rows();
            self.navigate_to(fi, &new_path);
        } else {
            // Item is first in file — try moving to previous file
            self.move_to_prev_file(&row.path, fi);
        }
    }

    pub fn swap_down(&mut self) {
        let Some(row) = self.rows.get(self.cursor).cloned() else {
            return;
        };
        if row.is_file_root {
            self.reorder_file_down();
            return;
        }
        let fi = row.file_index;
        if let Ok(new_path) = duir_core::tree_ops::swap_down(&mut self.files[fi].data, &row.path) {
            self.mark_modified(fi, &new_path);
            self.rebuild_rows();
            self.navigate_to(fi, &new_path);
        } else {
            // Item is last in file — try moving to next file
            self.move_to_next_file(&row.path, fi);
        }
    }

    pub fn promote(&mut self) {
        let Some(row) = self.rows.get(self.cursor).cloned() else {
            return;
        };
        if row.is_file_root {
            return;
        }
        let fi = row.file_index;
        if let Ok(new_path) = duir_core::tree_ops::promote(&mut self.files[fi].data, &row.path) {
            self.mark_modified(fi, &new_path);
            self.rebuild_rows();
            self.navigate_to(fi, &new_path);
        }
    }

    pub fn demote(&mut self) {
        let Some(row) = self.rows.get(self.cursor).cloned() else {
            return;
        };
        if row.is_file_root {
            return;
        }
        let fi = row.file_index;
        if let Ok(new_path) = duir_core::tree_ops::demote(&mut self.files[fi].data, &row.path) {
            self.mark_modified(fi, &new_path);
            self.rebuild_rows();
            self.navigate_to(fi, &new_path);
        }
    }

    pub fn sort_children(&mut self) {
        let Some(row) = self.rows.get(self.cursor).cloned() else {
            return;
        };
        let fi = row.file_index;
        if row.is_file_root {
            self.files[fi]
                .data
                .items
                .sort_by(|a, b| a.title.to_lowercase().cmp(&b.title.to_lowercase()));
            self.mark_modified(fi, &[]);
            self.set_status("Sorted", super::StatusLevel::Success);
        } else if duir_core::tree_ops::sort_children(&mut self.files[fi].data, &row.path).is_ok() {
            self.mark_modified(fi, &row.path);
            self.set_status("Sorted", super::StatusLevel::Success);
        }
        self.rebuild_rows();
    }

    pub fn clone_subtree(&mut self) {
        let Some(row) = self.rows.get(self.cursor).cloned() else {
            return;
        };
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

    fn move_to_prev_file(&mut self, path: &[usize], fi: usize) {
        if fi == 0 || path.len() != 1 {
            return; // only move top-level items across files
        }
        let target_fi = fi - 1;
        let Ok(item) = duir_core::tree_ops::remove_item(&mut self.files[fi].data, &path.to_vec()) else {
            return;
        };
        self.files[target_fi].data.items.push(item);
        let new_idx = self.files[target_fi].data.items.len() - 1;
        self.mark_modified(fi, &[]);
        self.mark_modified(target_fi, &[new_idx]);
        self.rebuild_rows();
        self.navigate_to(target_fi, &[new_idx]);
    }

    fn move_to_next_file(&mut self, path: &[usize], fi: usize) {
        if fi + 1 >= self.files.len() || path.len() != 1 {
            return; // only move top-level items across files
        }
        let target_fi = fi + 1;
        let Ok(item) = duir_core::tree_ops::remove_item(&mut self.files[fi].data, &path.to_vec()) else {
            return;
        };
        self.files[target_fi].data.items.insert(0, item);
        self.mark_modified(fi, &[]);
        self.mark_modified(target_fi, &[0]);
        self.rebuild_rows();
        self.navigate_to(target_fi, &[0]);
    }

    fn reorder_file_up(&mut self) {
        let Some(row) = self.rows.get(self.cursor).cloned() else {
            return;
        };
        let fi = row.file_index;
        if fi == 0 {
            return;
        }
        self.files.swap(fi, fi - 1);
        self.rebuild_rows();
        // Navigate to the file root at new position
        if let Some(pos) = self.rows.iter().position(|r| r.is_file_root && r.file_index == fi - 1) {
            self.cursor = pos;
        }
    }

    fn reorder_file_down(&mut self) {
        let Some(row) = self.rows.get(self.cursor).cloned() else {
            return;
        };
        let fi = row.file_index;
        if fi + 1 >= self.files.len() {
            return;
        }
        self.files.swap(fi, fi + 1);
        self.rebuild_rows();
        if let Some(pos) = self.rows.iter().position(|r| r.is_file_root && r.file_index == fi + 1) {
            self.cursor = pos;
        }
    }
}
