use duir_core::stats::compute_stats;
use duir_core::tree_ops::TreePath;
use duir_core::{Completion, TodoFile, TodoItem};

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
    pub encrypted: bool,
    pub locked: bool,
    pub has_encrypted_children: bool,
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
    pub filter_exclude: bool,
    pub command_active: bool,
    pub command_buffer: String,
    pub autosave_global: bool,
    pub editor: Option<crate::note_editor::NoteEditor<'static>>,
    pub editor_file_index: usize,
    pub editor_path: duir_core::tree_ops::TreePath,
    pub command_history: Vec<String>,
    pub command_history_index: Option<usize>,
    pub note_panel_pct: u16,
    pub editor_cache: std::collections::HashMap<(usize, Vec<usize>), crate::note_editor::NoteEditor<'static>>,
    pub show_help: bool,
    pub help_scroll: u16,
    pub show_about: bool,
    pub pending_delete: bool,
    pub password_prompt: Option<crate::password::PasswordPrompt>,
    pub passwords: std::collections::HashMap<(usize, Vec<usize>), String>,
    pub completer: crate::completer::Completer,
}

impl App {
    #[must_use]
    pub fn new() -> Self {
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
            filter_exclude: false,
            command_active: false,
            command_buffer: String::new(),
            autosave_global: true,
            editor: None,
            editor_file_index: 0,
            editor_path: vec![],
            command_history: Vec::new(),
            command_history_index: None,
            note_panel_pct: 50,
            editor_cache: std::collections::HashMap::new(),
            show_help: false,
            help_scroll: 0,
            show_about: false,
            pending_delete: false,
            password_prompt: None,
            passwords: std::collections::HashMap::new(),
            completer: crate::completer::Completer::new(crate::completer::APP_COMMANDS),
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
                encrypted: false,
                locked: false,
                has_encrypted_children: false,
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
            encrypted: item.is_encrypted(),
            locked: item.is_locked(),
            has_encrypted_children: has_enc_children,
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
        duir_core::tree_ops::get_item(&self.files[row.file_index].data, &row.path)
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

    /// Save editor content back to the model, then load the new item's note.
    pub fn sync_editor(&mut self) {
        // Save current editor content and cache its state
        if let Some(mut editor) = self.editor.take() {
            if editor.dirty {
                let content = editor.content();
                let fi = self.editor_file_index;
                let path = self.editor_path.clone();
                if path.is_empty() {
                    if fi < self.files.len() {
                        self.files[fi].data.note = content;
                        self.files[fi].modified = true;
                    }
                } else if fi < self.files.len()
                    && let Some(item) = duir_core::tree_ops::get_item_mut(&mut self.files[fi].data, &path)
                {
                    item.note = content;
                    self.files[fi].modified = true;
                }
                editor.dirty = false;
            }
            // Cache editor state (preserves undo history, cursor, etc.)
            let key = (self.editor_file_index, self.editor_path.clone());
            self.editor_cache.insert(key, editor);
        }

        // Load or restore editor for new item
        if let Some(row) = self.current_row().cloned() {
            let key = (row.file_index, row.path.clone());
            let editor = self.editor_cache.remove(&key).unwrap_or_else(|| {
                let note = self.current_note();
                crate::note_editor::NoteEditor::new(&note)
            });
            self.editor = Some(editor);
            self.editor_file_index = row.file_index;
            self.editor_path = row.path;
        } else {
            self.editor = None;
        }
    }

    fn navigate_to(&mut self, file_index: usize, path: &[usize]) {
        if let Some(pos) = self
            .rows
            .iter()
            .position(|r| r.file_index == file_index && !r.is_file_root && r.path == path)
        {
            self.cursor = pos;
            self.note_scroll = 0;
            self.sync_editor();
        }
    }
    pub fn move_up(&mut self) {
        if self.cursor > 0 {
            self.cursor -= 1;
            self.note_scroll = 0;
            self.sync_editor();
        }
    }

    pub fn move_down(&mut self) {
        if self.cursor + 1 < self.rows.len() {
            self.cursor += 1;
            self.note_scroll = 0;
            self.sync_editor();
        }
    }

    pub fn collapse_current(&mut self) {
        if let Some(row) = self.rows.get(self.cursor) {
            if row.is_file_root {
                return;
            }
            let fi = row.file_index;
            let path = row.path.clone();
            if let Some(item) = duir_core::tree_ops::get_item_mut(&mut self.files[fi].data, &path)
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
            // Check if locked — prompt for password
            if duir_core::tree_ops::get_item(&self.files[fi].data, &path)
                .is_some_and(duir_core::model::TodoItem::is_locked)
            {
                self.try_expand_encrypted();
                return;
            }
            if let Some(item) = duir_core::tree_ops::get_item_mut(&mut self.files[fi].data, &path)
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
            if let Some(item) = duir_core::tree_ops::get_item_mut(&mut self.files[fi].data, &path) {
                item.completed = match item.completed {
                    Completion::Done => Completion::Open,
                    _ => Completion::Done,
                };
            }
            self.files[fi].modified = true;
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
            if duir_core::tree_ops::add_sibling(&mut self.files[fi].data, &row.path, new_item).is_ok() {
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
                duir_core::tree_ops::get_item(&self.files[fi].data, &row.path).map_or(0, |item| item.items.len());
            let mut new_path = row.path.clone();
            new_path.push(child_idx);
            let new_item = TodoItem::new("<new task>");
            if duir_core::tree_ops::add_child(&mut self.files[fi].data, &row.path, new_item).is_ok() {
                if let Some(item) = duir_core::tree_ops::get_item_mut(&mut self.files[fi].data, &row.path) {
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
            let has_children = duir_core::tree_ops::get_item(&self.files[fi].data, &row.path)
                .is_some_and(|item| !item.items.is_empty());
            if has_children {
                self.pending_delete = true;
                "Node has children! Press y to confirm, any other key to cancel".clone_into(&mut self.status_message);
                return;
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
                self.files[fi].modified = true;
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
            if let Ok(new_path) = duir_core::tree_ops::swap_down(&mut self.files[fi].data, &row.path) {
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
            if let Ok(new_path) = duir_core::tree_ops::promote(&mut self.files[fi].data, &row.path) {
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
            if let Ok(new_path) = duir_core::tree_ops::demote(&mut self.files[fi].data, &row.path) {
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
            if duir_core::tree_ops::sort_children(&mut self.files[fi].data, &row.path).is_ok() {
                self.files[fi].modified = true;
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
            // New clone path: same parent, index + 1
            let mut new_path = row.path.clone();
            if let Some(last) = new_path.last_mut() {
                *last += 1;
            }
            if duir_core::tree_ops::clone_subtree(&mut self.files[fi].data, &row.path).is_ok() {
                self.files[fi].modified = true;
                self.rebuild_rows();
                self.navigate_to(fi, &new_path);
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
            if let Some(item) = duir_core::tree_ops::get_item_mut(&mut self.files[fi].data, &row.path)
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
    pub fn execute_command(&mut self, storage: &dyn duir_core::TodoStorage) {
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
            "import" => self.cmd_import(&parts),
            "open" => self.cmd_open_md(&parts),
            "collapse" => self.cmd_collapse(),
            "expand" => self.cmd_expand(),
            "autosave" => self.cmd_autosave(&parts),
            "init" => {
                let config = duir_core::config::Config::load();
                match config.init_local() {
                    Ok(()) => "Initialized .duir/ in current directory".clone_into(&mut self.status_message),
                    Err(e) => self.status_message = format!("Init error: {e}"),
                }
            }
            "config" => {
                if parts.get(1).copied() == Some("write") {
                    let config = duir_core::config::Config::default();
                    let path = if duir_core::config::Config::has_local() {
                        std::path::PathBuf::from(".duir/config.toml")
                    } else if let Some(d) = dirs::config_dir() {
                        d.join("duir").join("config.toml")
                    } else {
                        std::path::PathBuf::from(".duir/config.toml")
                    };
                    match config.write_to(&path) {
                        Ok(()) => self.status_message = format!("Config written to {}", path.display()),
                        Err(e) => self.status_message = format!("Config error: {e}"),
                    }
                } else {
                    let config = duir_core::config::Config::load();
                    self.status_message = format!(
                        "central={} local={} autosave={}",
                        config.storage.central.display(),
                        config.storage.local.display(),
                        config.editor.autosave,
                    );
                }
            }
            "help" => {
                self.show_help = true;
                self.help_scroll = 0;
            }
            "encrypt" => self.cmd_encrypt(),
            "decrypt" => self.cmd_decrypt(),
            "about" => {
                self.show_about = true;
            }
            _ => {
                self.status_message = format!("Unknown command: {cmd}");
            }
        }
    }

    fn save_current(&mut self, storage: &dyn duir_core::TodoStorage) {
        if let Some(row) = self.current_row().cloned() {
            let fi = row.file_index;
            let pw_map: std::collections::HashMap<Vec<usize>, String> = self
                .passwords
                .iter()
                .filter(|((f, _), _)| *f == fi)
                .map(|((_, path), pw)| (path.clone(), pw.clone()))
                .collect();

            let file = &mut self.files[fi];
            let saved = duir_core::crypto::lock_for_save(&mut file.data.items, &pw_map, &[]);
            match saved {
                Ok(saved_state) => {
                    match storage.save(&file.name, &file.data) {
                        Ok(()) => {
                            file.modified = false;
                            self.status_message = format!("Saved {}", file.name);
                        }
                        Err(e) => self.status_message = format!("Save error: {e}"),
                    }
                    duir_core::crypto::restore_after_save(&mut file.data.items, &saved_state);
                }
                Err(e) => self.status_message = format!("Encrypt error on save: {e}"),
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

    fn open_file_path(&mut self, path_str: &str, storage: &dyn duir_core::TodoStorage) {
        let path = std::path::Path::new(path_str);
        if path.exists() {
            match duir_core::file_storage::load_path(path) {
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
        // :export md [filename]
        if parts.len() < 2 {
            "Usage: :export md [file.md]".clone_into(&mut self.status_message);
            return;
        }
        if let Some(item) = self.current_item() {
            let md = duir_core::markdown_export::export_subtree(item, 3);
            let path = if let Some(&fname) = parts.get(2) {
                std::path::PathBuf::from(fname)
            } else {
                std::env::temp_dir().join("duir-export.md")
            };
            match std::fs::write(&path, &md) {
                Ok(()) => self.status_message = format!("Exported to {}", path.display()),
                Err(e) => self.status_message = format!("Export error: {e}"),
            }
        } else {
            "No item selected".clone_into(&mut self.status_message);
        }
    }

    fn cmd_import(&mut self, parts: &[&str]) {
        // :import md <file.md> — import as children of current item
        if parts.len() < 3 || parts[1] != "md" {
            "Usage: :import md <file.md>".clone_into(&mut self.status_message);
            return;
        }
        let path = std::path::Path::new(parts[2]);
        match std::fs::read_to_string(path) {
            Ok(content) => {
                let parsed = duir_core::markdown_import::import_markdown(&content);
                if let Some(row) = self.rows.get(self.cursor).cloned() {
                    let fi = row.file_index;
                    if row.is_file_root {
                        self.files[fi].data.items.extend(parsed.items);
                    } else if let Some(item) = duir_core::tree_ops::get_item_mut(&mut self.files[fi].data, &row.path) {
                        item.items.extend(parsed.items);
                        item.folded = false;
                    }
                    self.files[fi].modified = true;
                    self.rebuild_rows();
                    self.status_message = format!("Imported {}", path.display());
                }
            }
            Err(e) => self.status_message = format!("Import error: {e}"),
        }
    }

    fn cmd_open_md(&mut self, parts: &[&str]) {
        // :open md <file.md> — open markdown as new top-level tree
        if parts.len() < 3 || parts[1] != "md" {
            "Usage: :open md <file.md>".clone_into(&mut self.status_message);
            return;
        }
        let path = std::path::Path::new(parts[2]);
        match std::fs::read_to_string(path) {
            Ok(content) => {
                let parsed = duir_core::markdown_import::import_markdown(&content);
                let name = path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("imported")
                    .to_owned();
                self.add_file(name.clone(), parsed);
                self.status_message = format!("Opened {name} as tree");
            }
            Err(e) => self.status_message = format!("Open error: {e}"),
        }
    }

    fn cmd_collapse(&mut self) {
        // Collapse subtree children into markdown note
        if let Some(row) = self.rows.get(self.cursor).cloned() {
            if row.is_file_root {
                return;
            }
            let fi = row.file_index;
            if let Some(item) = duir_core::tree_ops::get_item_mut(&mut self.files[fi].data, &row.path) {
                if item.items.is_empty() {
                    "No children to collapse".clone_into(&mut self.status_message);
                    return;
                }
                // Export children as markdown, append to note with marker
                let mut md = String::new();
                for child in &item.items {
                    md.push_str(&duir_core::markdown_export::export_subtree(child, 3));
                }
                if !item.note.is_empty() {
                    item.note.push_str("\n\n");
                }
                item.note.push_str("<!-- duir:collapsed -->\n");
                item.note.push_str(&md);
                item.items.clear();
                self.files[fi].modified = true;
                self.rebuild_rows();
                "Children collapsed to note".clone_into(&mut self.status_message);
            }
        }
    }

    fn cmd_expand(&mut self) {
        if let Some(row) = self.rows.get(self.cursor).cloned() {
            if row.is_file_root {
                return;
            }
            let fi = row.file_index;
            if let Some(item) = duir_core::tree_ops::get_item_mut(&mut self.files[fi].data, &row.path) {
                if item.note.trim().is_empty() {
                    "No note to expand".clone_into(&mut self.status_message);
                    return;
                }
                let marker = "<!-- duir:collapsed -->";
                let (keep_note, md_part) = if let Some(pos) = item.note.find(marker) {
                    (
                        item.note[..pos].trim_end().to_owned(),
                        item.note[pos + marker.len()..].to_owned(),
                    )
                } else {
                    (String::new(), item.note.clone())
                };
                let parsed = duir_core::markdown_import::import_markdown(&md_part);
                if parsed.items.is_empty() {
                    "No tree structure found in note".clone_into(&mut self.status_message);
                    return;
                }
                item.items.extend(parsed.items);
                item.note = keep_note;
                item.folded = false;
                self.files[fi].modified = true;
                self.rebuild_rows();
                "Note expanded to children".clone_into(&mut self.status_message);
            }
        }
    }

    fn cmd_encrypt(&mut self) {
        if let Some(row) = self.rows.get(self.cursor).cloned() {
            if row.is_file_root {
                "Cannot encrypt file root".clone_into(&mut self.status_message);
                return;
            }
            let fi = row.file_index;
            let item = duir_core::tree_ops::get_item(&self.files[fi].data, &row.path);
            let already_encrypted = item.is_some_and(duir_core::TodoItem::is_encrypted);
            let title = if already_encrypted {
                "Change encryption password"
            } else {
                "Encrypt subtree"
            };
            let action = if already_encrypted {
                crate::password::PasswordAction::ChangePassword {
                    file_index: fi,
                    path: row.path,
                }
            } else {
                crate::password::PasswordAction::Encrypt {
                    file_index: fi,
                    path: row.path,
                }
            };
            self.password_prompt = Some(crate::password::PasswordPrompt::new(title, action));
        }
    }

    fn cmd_decrypt(&mut self) {
        if let Some(row) = self.rows.get(self.cursor).cloned() {
            if row.is_file_root {
                return;
            }
            let fi = row.file_index;
            if let Some(item) = duir_core::tree_ops::get_item_mut(&mut self.files[fi].data, &row.path) {
                if !item.is_encrypted() {
                    "Node is not encrypted".clone_into(&mut self.status_message);
                    return;
                }
                duir_core::crypto::strip_encryption(item);
                self.passwords.remove(&(fi, row.path));
                self.files[fi].modified = true;
                self.rebuild_rows();
                "Encryption removed".clone_into(&mut self.status_message);
            }
        }
    }

    /// Handle password prompt result.
    pub fn handle_password_result(&mut self, password: &str) {
        let Some(prompt) = self.password_prompt.take() else {
            return;
        };
        match prompt.callback {
            crate::password::PasswordAction::Encrypt { file_index, path } => {
                if let Some(item) = duir_core::tree_ops::get_item_mut(&mut self.files[file_index].data, &path) {
                    match duir_core::crypto::encrypt_item(item, password) {
                        Ok(()) => {
                            self.passwords.insert((file_index, path), password.to_owned());
                            self.files[file_index].modified = true;
                            self.rebuild_rows();
                            "Subtree encrypted".clone_into(&mut self.status_message);
                        }
                        Err(e) => self.status_message = format!("Encrypt error: {e}"),
                    }
                }
            }
            crate::password::PasswordAction::Decrypt { file_index, path } => {
                if let Some(item) = duir_core::tree_ops::get_item_mut(&mut self.files[file_index].data, &path) {
                    match duir_core::crypto::decrypt_item(item, password) {
                        Ok(()) => {
                            self.passwords.insert((file_index, path), password.to_owned());
                            self.files[file_index].modified = true;
                            self.rebuild_rows();
                            "Subtree unlocked".clone_into(&mut self.status_message);
                        }
                        Err(_) => "Wrong password".clone_into(&mut self.status_message),
                    }
                }
            }
            crate::password::PasswordAction::ChangePassword { file_index, path } => {
                // Already decrypted — re-encrypt with new password
                if let Some(item) = duir_core::tree_ops::get_item_mut(&mut self.files[file_index].data, &path) {
                    match duir_core::crypto::encrypt_item(item, password) {
                        Ok(()) => {
                            self.passwords.insert((file_index, path), password.to_owned());
                            self.files[file_index].modified = true;
                            self.rebuild_rows();
                            "Password changed".clone_into(&mut self.status_message);
                        }
                        Err(e) => self.status_message = format!("Encrypt error: {e}"),
                    }
                }
            }
        }
    }

    /// Try to expand an encrypted node — prompts for password.
    pub fn try_expand_encrypted(&mut self) {
        if let Some(row) = self.rows.get(self.cursor).cloned() {
            if row.is_file_root {
                return;
            }
            let fi = row.file_index;
            let is_locked = duir_core::tree_ops::get_item(&self.files[fi].data, &row.path)
                .is_some_and(duir_core::model::TodoItem::is_locked);
            if is_locked {
                self.password_prompt = Some(crate::password::PasswordPrompt::new(
                    "Unlock encrypted node",
                    crate::password::PasswordAction::Decrypt {
                        file_index: fi,
                        path: row.path,
                    },
                ));
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

    pub fn save_all(&mut self, storage: &dyn duir_core::TodoStorage) {
        for (fi, file) in self.files.iter_mut().enumerate() {
            if file.modified {
                // Build password map for this file (strip file_index from keys)
                let pw_map: std::collections::HashMap<Vec<usize>, String> = self
                    .passwords
                    .iter()
                    .filter(|((f, _), _)| *f == fi)
                    .map(|((_, path), pw)| (path.clone(), pw.clone()))
                    .collect();

                // Re-encrypt unlocked nodes for save
                let saved = duir_core::crypto::lock_for_save(&mut file.data.items, &pw_map, &[]);

                match saved {
                    Ok(saved_state) => {
                        match storage.save(&file.name, &file.data) {
                            Ok(()) => file.modified = false,
                            Err(e) => {
                                self.status_message = format!("Save error: {e}");
                            }
                        }
                        // Restore decrypted state in memory
                        duir_core::crypto::restore_after_save(&mut file.data.items, &saved_state);
                    }
                    Err(e) => {
                        self.status_message = format!("Encrypt error on save: {e}");
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

    /// Apply the current filter text, searching titles and notes.
    pub fn apply_filter(&mut self) {
        // First rebuild all rows
        self.rebuild_rows();

        if self.filter_text.is_empty() {
            return;
        }

        let opts = duir_core::filter::FilterOptions {
            search_notes: true,
            case_sensitive: false,
        };

        // Collect matching paths per file
        let mut match_set: std::collections::HashSet<(usize, Vec<usize>)> = std::collections::HashSet::new();
        for (fi, file) in self.files.iter().enumerate() {
            let matches = duir_core::filter::filter_items(&file.data.items, &self.filter_text, &opts);
            for path in matches {
                match_set.insert((fi, path));
            }
        }

        // Filter rows: keep file roots + matching items
        if self.filter_exclude {
            // Exclude mode: hide matching items
            self.rows
                .retain(|row| row.is_file_root || !match_set.contains(&(row.file_index, row.path.clone())));
        } else {
            // Include mode: show only matching items
            self.rows
                .retain(|row| row.is_file_root || match_set.contains(&(row.file_index, row.path.clone())));
        }

        let visible = self.rows.iter().filter(|r| !r.is_file_root).count();
        let mode = if self.filter_exclude { "exclude" } else { "include" };
        self.status_message = format!("Filter '{}' ({}): {} visible", self.filter_text, mode, visible);

        if self.cursor >= self.rows.len() && !self.rows.is_empty() {
            self.cursor = self.rows.len() - 1;
        }
    }
}
