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
    pub autosave: bool,
}

/// Focus area in the UI.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Focus {
    Tree,
    Note,
}

/// Application state.
#[allow(clippy::struct_excessive_bools)]
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
    pub edit_select_all: bool,
    pub filter_active: bool,
    pub filter_text: String,
    pub command_active: bool,
    pub command_buffer: String,
    pub autosave_global: bool,
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
            edit_select_all: false,
            filter_active: false,
            filter_text: String::new(),
            command_active: false,
            command_buffer: String::new(),
            autosave_global: false,
        }
    }

    pub fn add_file(&mut self, name: String, data: TodoFile) {
        let autosave = self.autosave_global;
        self.files.push(LoadedFile {
            name,
            data,
            modified: false,
            autosave,
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

    fn navigate_to(&mut self, file_index: usize, path: &[usize]) {
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
            // New sibling path: same parent, index + 1
            let mut new_path = row.path.clone();
            if let Some(last) = new_path.last_mut() {
                *last += 1;
            }
            let new_item = TodoItem::new("<new task>");
            if omela_core::tree_ops::add_sibling(&mut self.files[fi].data, &row.path, new_item).is_ok() {
                self.files[fi].modified = true;
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
                self.files[fi].modified = true;
                self.rebuild_rows();
                self.navigate_to(fi, &[child_idx]);
                self.start_editing();
                return;
            }
            // New child path: current path + last child index
            let child_idx =
                omela_core::tree_ops::get_item(&self.files[fi].data, &row.path).map_or(0, |item| item.items.len());
            let mut new_path = row.path.clone();
            new_path.push(child_idx);
            let new_item = TodoItem::new("<new task>");
            if omela_core::tree_ops::add_child(&mut self.files[fi].data, &row.path, new_item).is_ok() {
                if let Some(item) = omela_core::tree_ops::get_item_mut(&mut self.files[fi].data, &row.path) {
                    item.folded = false;
                }
                self.files[fi].modified = true;
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
            self.edit_select_all = true;
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

    /// Execute a `:` command. Returns an optional path for file operations.
    pub fn execute_command(&mut self, storage: &dyn omela_core::TodoStorage) {
        let cmd = self.command_buffer.trim().to_owned();
        self.command_active = false;
        self.command_buffer.clear();

        let parts: Vec<&str> = cmd.splitn(3, ' ').collect();
        match parts.first().copied().unwrap_or("") {
            "w" => self.save_current(storage),
            "wa" => self.save_all(storage),
            "q" => self.close_current_file(),
            "qa" | "q!" => {
                self.should_quit = true;
            }
            "e" => {
                if let Some(&name) = parts.get(1) {
                    self.add_empty_file(name);
                    self.status_message = format!("New file: {name}");
                } else {
                    "Usage: :e <name>".clone_into(&mut self.status_message);
                }
            }
            "o" => {
                if let Some(&path_str) = parts.get(1) {
                    self.open_file_path(path_str, storage);
                } else {
                    "Usage: :o <path>".clone_into(&mut self.status_message);
                }
            }
            "export" => self.cmd_export(&parts),
            "collapse" => self.cmd_collapse(),
            "expand" => self.cmd_expand(),
            "autosave" => self.cmd_autosave(&parts),
            _ => {
                self.status_message = format!("Unknown command: {cmd}");
            }
        }
    }

    fn save_current(&mut self, storage: &dyn omela_core::TodoStorage) {
        if let Some(row) = self.current_row().cloned() {
            let fi = row.file_index;
            let file = &mut self.files[fi];
            match storage.save(&file.name, &file.data) {
                Ok(()) => {
                    file.modified = false;
                    self.status_message = format!("Saved {}", file.name);
                }
                Err(e) => self.status_message = format!("Save error: {e}"),
            }
        }
    }

    fn close_current_file(&mut self) {
        if let Some(row) = self.current_row().cloned() {
            let fi = row.file_index;
            if self.files[fi].modified {
                "File has unsaved changes. Use :q! to force".clone_into(&mut self.status_message);
                return;
            }
            self.files.remove(fi);
            if self.files.is_empty() {
                self.should_quit = true;
            } else {
                self.rebuild_rows();
            }
        }
    }

    fn open_file_path(&mut self, path_str: &str, storage: &dyn omela_core::TodoStorage) {
        let path = std::path::Path::new(path_str);
        if path.exists() {
            match omela_core::file_storage::load_path(path) {
                Ok(data) => {
                    let name = path
                        .file_stem()
                        .and_then(|s| s.to_str())
                        .unwrap_or("untitled")
                        .to_owned();
                    self.add_file(name.clone(), data);
                    self.status_message = format!("Opened {name}");
                }
                Err(e) => self.status_message = format!("Error: {e}"),
            }
        } else {
            // Try as a name in the storage directory
            match storage.load(path_str) {
                Ok(data) => {
                    self.add_file(path_str.to_owned(), data);
                    self.status_message = format!("Opened {path_str}");
                }
                Err(e) => self.status_message = format!("Error: {e}"),
            }
        }
    }

    fn cmd_export(&mut self, parts: &[&str]) {
        if parts.get(1).copied() != Some("md") {
            "Usage: :export md".clone_into(&mut self.status_message);
            return;
        }
        if let Some(item) = self.current_item() {
            let md = omela_core::markdown_export::export_subtree(item, 3);
            // Store in clipboard-like status for now; in future write to file
            self.status_message = format!("Exported {} lines", md.lines().count());
            // Write to a temp location
            let path = std::env::temp_dir().join("omela-export.md");
            if std::fs::write(&path, &md).is_ok() {
                self.status_message = format!("Exported to {}", path.display());
            }
        } else {
            "No item selected".clone_into(&mut self.status_message);
        }
    }

    fn cmd_collapse(&mut self) {
        // Collapse subtree children into markdown note
        if let Some(row) = self.rows.get(self.cursor).cloned() {
            if row.is_file_root {
                return;
            }
            let fi = row.file_index;
            if let Some(item) = omela_core::tree_ops::get_item_mut(&mut self.files[fi].data, &row.path) {
                if item.items.is_empty() {
                    "No children to collapse".clone_into(&mut self.status_message);
                    return;
                }
                // Export children as markdown, append to note
                let mut md = String::new();
                for child in &item.items {
                    md.push_str(&omela_core::markdown_export::export_subtree(child, 3));
                }
                if !item.note.is_empty() {
                    item.note.push_str("\n\n");
                }
                item.note.push_str(&md);
                item.items.clear();
                self.files[fi].modified = true;
                self.rebuild_rows();
                "Children collapsed to note".clone_into(&mut self.status_message);
            }
        }
    }

    fn cmd_expand(&mut self) {
        // Expand markdown note into subtree children
        if let Some(row) = self.rows.get(self.cursor).cloned() {
            if row.is_file_root {
                return;
            }
            let fi = row.file_index;
            if let Some(item) = omela_core::tree_ops::get_item_mut(&mut self.files[fi].data, &row.path) {
                if item.note.trim().is_empty() {
                    "No note to expand".clone_into(&mut self.status_message);
                    return;
                }
                let parsed = omela_core::markdown_import::import_markdown(&item.note);
                item.items.extend(parsed.items);
                item.note.clear();
                item.folded = false;
                self.files[fi].modified = true;
                self.rebuild_rows();
                "Note expanded to children".clone_into(&mut self.status_message);
            }
        }
    }

    fn cmd_autosave(&mut self, parts: &[&str]) {
        if parts.get(1).copied() == Some("all") {
            self.autosave_global = !self.autosave_global;
            for file in &mut self.files {
                file.autosave = self.autosave_global;
            }
            let state = if self.autosave_global { "ON" } else { "OFF" };
            self.status_message = format!("Autosave (all): {state}");
        } else if let Some(row) = self.current_row().cloned() {
            let fi = row.file_index;
            self.files[fi].autosave = !self.files[fi].autosave;
            let state = if self.files[fi].autosave { "ON" } else { "OFF" };
            let name = &self.files[fi].name;
            self.status_message = format!("Autosave {name}: {state}");
        }
    }

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
