use super::App;

// fi (file_index) is always set by rebuild_rows from 0..self.files.len(), so self.files[fi] is safe.
#[allow(clippy::indexing_slicing)]
impl App {
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
