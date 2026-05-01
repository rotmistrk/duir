use omela_core::stats::compute_stats;
use omela_core::tree_ops::TreePath;
use omela_core::{Completion, TodoFile, TodoItem};

/// A flattened row in the tree view, used for rendering and navigation.
#[derive(Debug, Clone)]
#[allow(clippy::struct_excessive_bools)]
pub struct TreeRow {
    pub path: TreePath,
    pub depth: usize,
    pub title: String,
    pub completed: Completion,
    pub important: bool,
    pub expanded: bool,
    pub has_children: bool,
    pub stats_text: String,
    pub is_file_root: bool,
    pub file_index: usize,
}

/// Loaded file with its data and metadata.
#[derive(Debug)]
pub struct LoadedFile {
    pub name: String,
    pub data: TodoFile,
    pub modified: bool,
}

/// Focus area in the UI.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Focus {
    Tree,
    Note,
}

/// Application state.
pub struct App {
    pub files: Vec<LoadedFile>,
    pub rows: Vec<TreeRow>,
    pub cursor: usize,
    pub scroll_offset: usize,
    pub focus: Focus,
    pub should_quit: bool,
    pub status_message: String,
    pub note_scroll: usize,
    pub editing_title: bool,
    pub edit_buffer: String,
    pub filter_active: bool,
    pub filter_text: String,
}

impl App {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            files: Vec::new(),
            rows: Vec::new(),
            cursor: 0,
            scroll_offset: 0,
            focus: Focus::Tree,
            should_quit: false,
            status_message: String::new(),
            note_scroll: 0,
            editing_title: false,
            edit_buffer: String::new(),
            filter_active: false,
            filter_text: String::new(),
        }
    }

    pub fn add_file(&mut self, name: String, data: TodoFile) {
        self.files.push(LoadedFile {
            name,
            data,
            modified: false,
        });
        self.rebuild_rows();
    }

    pub fn add_empty_file(&mut self, name: &str) {
        self.add_file(name.to_owned(), TodoFile::new(name));
    }

    /// Rebuild the flattened row list from all loaded files.
    pub fn rebuild_rows(&mut self) {
        self.rows.clear();
        for fi in 0..self.files.len() {
            let file = &self.files[fi];
            // File root row
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
            });
            // Items — collect data first to avoid borrow conflict
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

        let expanded = !item.folded && !item.items.is_empty();

        self.rows.push(TreeRow {
            path: path.to_vec(),
            depth,
            title: item.title.clone(),
            completed: item.completed.clone(),
            important: item.important,
            expanded,
            has_children: !item.items.is_empty(),
            stats_text,
            is_file_root: false,
            file_index,
        });

        if expanded {
            for (i, child) in item.items.iter().enumerate() {
                let mut child_path = path.to_vec();
                child_path.push(i);
                self.flatten_item(child, &child_path, depth + 1, file_index);
            }
        }
    }

    /// Get the current row, if any.
    #[must_use]
    pub fn current_row(&self) -> Option<&TreeRow> {
        self.rows.get(self.cursor)
    }

    /// Get the current item from the model.
    #[must_use]
    pub fn current_item(&self) -> Option<&TodoItem> {
        let row = self.current_row()?;
        if row.is_file_root {
            return None;
        }
        omela_core::tree_ops::get_item(&self.files[row.file_index].data, &row.path)
    }

    /// Get the current note text.
    #[must_use]
    pub fn current_note(&self) -> String {
        self.current_row().map_or_else(String::new, |row| {
            if row.is_file_root {
                self.files[row.file_index].data.note.clone()
            } else if let Some(item) = self.current_item() {
                item.note.clone()
            } else {
                String::new()
            }
        })
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

    pub fn collapse_current(&mut self) {
        if let Some(row) = self.rows.get(self.cursor) {
            if row.is_file_root {
                return;
            }
            let fi = row.file_index;
            let path = row.path.clone();
            if let Some(item) = omela_core::tree_ops::get_item_mut(&mut self.files[fi].data, &path)
                && !item.folded
                && !item.items.is_empty()
            {
                item.folded = true;
                self.rebuild_rows();
                return;
            }
            // If already collapsed or leaf, move to parent
            if path.len() > 1 {
                let parent_path: TreePath = path[..path.len() - 1].to_vec();
                if let Some(pos) = self
                    .rows
                    .iter()
                    .position(|r| r.file_index == fi && !r.is_file_root && r.path == parent_path)
                {
                    self.cursor = pos;
                    self.note_scroll = 0;
                }
            } else if let Some(pos) = self.rows.iter().position(|r| r.file_index == fi && r.is_file_root) {
                self.cursor = pos;
                self.note_scroll = 0;
            }
        }
    }

    pub fn expand_current(&mut self) {
        if let Some(row) = self.rows.get(self.cursor) {
            if row.is_file_root {
                return;
            }
            let fi = row.file_index;
            let path = row.path.clone();
            if let Some(item) = omela_core::tree_ops::get_item_mut(&mut self.files[fi].data, &path)
                && item.folded
                && !item.items.is_empty()
            {
                item.folded = false;
                self.rebuild_rows();
            }
        }
    }

    pub fn toggle_completed(&mut self) {
        if let Some(row) = self.rows.get(self.cursor) {
            if row.is_file_root {
                return;
            }
            let fi = row.file_index;
            let path = row.path.clone();
            if let Some(item) = omela_core::tree_ops::get_item_mut(&mut self.files[fi].data, &path) {
                item.completed = match item.completed {
                    Completion::Done => Completion::Open,
                    _ => Completion::Done,
                };
            }
            self.files[fi].modified = true;
            for item in &mut self.files[fi].data.items {
                omela_core::stats::update_completion(item);
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
            if let Some(item) = omela_core::tree_ops::get_item_mut(&mut self.files[fi].data, &path) {
                item.important = !item.important;
            }
            self.files[fi].modified = true;
            self.rebuild_rows();
        }
    }

    pub fn new_sibling(&mut self) {
        if let Some(row) = self.rows.get(self.cursor).cloned() {
            if row.is_file_root {
                return;
            }
            let fi = row.file_index;
            let new_item = TodoItem::new("<new task>");
            if omela_core::tree_ops::add_sibling(&mut self.files[fi].data, &row.path, new_item).is_ok() {
                self.files[fi].modified = true;
                self.rebuild_rows();
                self.move_down();
                self.start_editing();
            }
        }
    }

    pub fn new_child(&mut self) {
        if let Some(row) = self.rows.get(self.cursor).cloned() {
            let fi = row.file_index;
            if row.is_file_root {
                // Add top-level item to this file
                self.files[fi].data.items.push(TodoItem::new("<new task>"));
                self.files[fi].modified = true;
                self.rebuild_rows();
                self.move_down();
                self.start_editing();
                return;
            }
            let new_item = TodoItem::new("<new task>");
            if omela_core::tree_ops::add_child(&mut self.files[fi].data, &row.path, new_item).is_ok() {
                // Unfold parent
                if let Some(item) = omela_core::tree_ops::get_item_mut(&mut self.files[fi].data, &row.path) {
                    item.folded = false;
                }
                self.files[fi].modified = true;
                self.rebuild_rows();
                self.move_down();
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
            if omela_core::tree_ops::remove_item(&mut self.files[fi].data, &row.path).is_ok() {
                self.files[fi].modified = true;
                self.rebuild_rows();
            }
        }
    }

    pub fn swap_up(&mut self) {
        if let Some(row) = self.rows.get(self.cursor).cloned() {
            if row.is_file_root {
                return;
            }
            let fi = row.file_index;
            if let Ok(new_path) = omela_core::tree_ops::swap_up(&mut self.files[fi].data, &row.path) {
                self.files[fi].modified = true;
                self.rebuild_rows();
                // Find new position
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
            if let Ok(new_path) = omela_core::tree_ops::swap_down(&mut self.files[fi].data, &row.path) {
                self.files[fi].modified = true;
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
            if let Ok(new_path) = omela_core::tree_ops::promote(&mut self.files[fi].data, &row.path) {
                self.files[fi].modified = true;
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
            if let Ok(new_path) = omela_core::tree_ops::demote(&mut self.files[fi].data, &row.path) {
                self.files[fi].modified = true;
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
            if omela_core::tree_ops::sort_children(&mut self.files[fi].data, &row.path).is_ok() {
                self.files[fi].modified = true;
                self.rebuild_rows();
            }
        }
    }

    pub fn start_editing(&mut self) {
        if let Some(row) = self.current_row() {
            if row.is_file_root {
                return;
            }
            self.edit_buffer = row.title.clone();
            self.editing_title = true;
        }
    }

    pub fn finish_editing(&mut self) {
        if !self.editing_title {
            return;
        }
        self.editing_title = false;
        if let Some(row) = self.rows.get(self.cursor).cloned() {
            if row.is_file_root {
                return;
            }
            let fi = row.file_index;
            if let Some(item) = omela_core::tree_ops::get_item_mut(&mut self.files[fi].data, &row.path)
                && item.title != self.edit_buffer
            {
                item.title.clone_from(&self.edit_buffer);
                self.files[fi].modified = true;
            }
            self.rebuild_rows();
        }
    }

    pub fn cancel_editing(&mut self) {
        self.editing_title = false;
        self.edit_buffer.clear();
    }

    /// Save all modified files using the given storage.
    pub fn save_all(&mut self, storage: &dyn omela_core::TodoStorage) {
        for file in &mut self.files {
            if file.modified {
                match storage.save(&file.name, &file.data) {
                    Ok(()) => {
                        file.modified = false;
                    }
                    Err(e) => {
                        self.status_message = format!("Save error: {e}");
                        return;
                    }
                }
            }
        }
        "Saved".clone_into(&mut self.status_message);
    }

    /// Check if any file has unsaved modifications.
    #[must_use]
    pub fn has_unsaved(&self) -> bool {
        self.files.iter().any(|f| f.modified)
    }
}
