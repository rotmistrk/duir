use std::sync::{Arc, Mutex};

use duir_core::mcp_server::McpMutation;
use duir_core::stats::compute_stats;
use duir_core::tree_ops::TreePath;
use duir_core::{Completion, NodeId, TodoFile, TodoItem};

/// Stable file identity — monotonic, never reused, survives reorder/close.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FileId(pub u64);

/// An active kiron session: PTY + optional MCP server state.
pub struct ActiveKiron {
    pub pty: crate::pty_tab::PtyTab,
    pub mcp_mutations: Option<std::sync::mpsc::Receiver<McpMutation>>,
    /// Kept alive so the MCP server thread can access the snapshot.
    #[allow(dead_code)]
    pub mcp_snapshot: Option<Arc<Mutex<TodoFile>>>,
}

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
    #[allow(dead_code)]
    pub file_id: FileId,
    pub encrypted: bool,
    pub locked: bool,
    pub has_encrypted_children: bool,
    pub is_kiron: bool,
}

/// Loaded file with its data and metadata.
#[derive(Debug)]
pub struct LoadedFile {
    pub id: FileId,
    pub name: String,
    pub data: TodoFile,
    modified: bool,
    pub autosave: bool,
}

impl LoadedFile {
    #[must_use]
    pub const fn is_modified(&self) -> bool {
        self.modified
    }
}

/// Focus area in the UI — each variant carries the state specific to that mode.
pub enum FocusState {
    Tree,
    EditingTitle {
        buffer: String,
        cursor: usize,
        select_all: bool,
    },
    Note {
        editor: Box<crate::note_editor::NoteEditor<'static>>,
        file_id: FileId,
        node_id: NodeId,
    },
    Command {
        buffer: String,
        history_index: Option<usize>,
    },
    Filter {
        text: String,
        saved: String,
    },
    Help {
        scroll: u16,
    },
    About,
}

/// Application state.

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum StatusLevel {
    #[default]
    Info,
    Success,
    Warning,
    Error,
}

/// A pending response capture: tracks which kiron PTY to monitor
/// and which prompt node to insert the response after.
pub struct PendingResponse {
    pub kiron_file_id: FileId,
    pub kiron_node_id: NodeId,
    pub prompt_file_id: FileId,
    pub prompt_node_id: NodeId,
    pub start_time: std::time::Instant,
}

#[allow(clippy::struct_excessive_bools)]
pub struct App {
    pub files: Vec<LoadedFile>,
    pub next_file_id: u64,
    pub rows: Vec<TreeRow>,
    pub cursor: usize,
    pub scroll_offset: usize,
    pub state: FocusState,
    pub should_quit: bool,
    pub status_message: String,
    pub status_level: StatusLevel,
    pub note_scroll: usize,
    pub autosave_global: bool,
    pub command_history: Vec<String>,
    pub note_panel_pct: u16,
    pub pending_delete: bool,
    pub password_prompt: Option<crate::password::PasswordPrompt>,
    pub passwords: std::collections::HashMap<(FileId, NodeId), String>,
    pub pending_crypto: Option<(String, crate::password::PasswordAction)>,
    pub completer: crate::completer::Completer,
    pub editor_cache: std::collections::HashMap<(FileId, NodeId), crate::note_editor::NoteEditor<'static>>,
    pub filter_committed_text: String,
    pub filter_committed_exclude: bool,
    pub highlighter: crate::syntax::SyntaxHighlighter,
    /// Active kiron PTY sessions, keyed by (`FileId`, `NodeId`).
    pub active_kirons: std::collections::HashMap<(FileId, NodeId), ActiveKiron>,
    /// Whether the Kiro tab is focused (vs Note tab) in the note panel.
    pub kiro_tab_focused: bool,
    /// Pending response captures awaiting idle timeout.
    pub pending_responses: Vec<PendingResponse>,
}

impl App {
    #[must_use]
    pub fn new() -> Self {
        Self {
            files: Vec::new(),
            next_file_id: 0,
            rows: Vec::new(),
            cursor: 0,
            scroll_offset: 0,
            state: FocusState::Tree,
            should_quit: false,
            status_message: String::new(),
            status_level: StatusLevel::Info,
            note_scroll: 0,
            autosave_global: true,
            command_history: Vec::new(),
            note_panel_pct: 50,
            editor_cache: std::collections::HashMap::new(),
            pending_delete: false,
            password_prompt: None,
            passwords: std::collections::HashMap::new(),
            pending_crypto: None,
            completer: crate::completer::Completer::new(crate::completer::APP_COMMANDS),
            filter_committed_text: String::new(),
            filter_committed_exclude: false,
            highlighter: crate::syntax::SyntaxHighlighter::new(),
            active_kirons: std::collections::HashMap::new(),
            kiro_tab_focused: false,
            pending_responses: Vec::new(),
        }
    }

    pub fn add_file(&mut self, name: String, data: TodoFile) {
        let id = FileId(self.next_file_id);
        self.next_file_id += 1;
        let autosave = self.autosave_global;
        self.files.push(LoadedFile {
            id,
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

    #[must_use]
    #[allow(dead_code)]
    pub fn file_by_id(&self, id: FileId) -> Option<&LoadedFile> {
        self.files.iter().find(|f| f.id == id)
    }

    #[allow(dead_code)]
    pub fn file_by_id_mut(&mut self, id: FileId) -> Option<&mut LoadedFile> {
        self.files.iter_mut().find(|f| f.id == id)
    }

    #[must_use]
    pub fn file_index_for_id(&self, id: FileId) -> Option<usize> {
        self.files.iter().position(|f| f.id == id)
    }

    /// Rebuild the flattened row list from all loaded files.
    pub fn rebuild_rows(&mut self) {
        self.rebuild_rows_raw();
        // Reapply committed filter
        if !self.filter_committed_text.is_empty() && !self.is_filter_active() {
            self.reapply_filter();
        }
    }

    fn rebuild_rows_raw(&mut self) {
        // Invalidate editor cache — paths may have shifted
        self.editor_cache.clear();

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
                file_id: file.id,
                encrypted: false,
                locked: false,
                has_encrypted_children: false,
                is_kiron: false,
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

    /// Write editor content back to the model. Called before any operation
    /// that needs the model to be up-to-date while in Note state.
    pub fn save_editor(&mut self) {
        if let FocusState::Note {
            ref editor,
            file_id,
            ref node_id,
        } = self.state
        {
            let content = editor.content();
            let Some(fi) = self.file_index_for_id(file_id) else {
                return;
            };
            if node_id.0.is_empty() {
                // File-level note
                if self.files[fi].data.note != content {
                    self.files[fi].data.note = content;
                    self.mark_modified(fi, &[]);
                }
            } else if let Some(path) = duir_core::tree_ops::find_node_path(&self.files[fi].data, node_id)
                && let Some(item) = duir_core::tree_ops::get_item_mut(&mut self.files[fi].data, &path)
                && item.note != content
            {
                item.note = content;
                self.mark_modified(fi, &path);
            }
        }
    }

    /// Reload the editor from the model (used after commands that modify the note
    /// while in Note state, like :collapse/:expand).
    pub fn reload_editor(&mut self) {
        let note = if let FocusState::Note {
            file_id, ref node_id, ..
        } = self.state
        {
            let fi = self.file_index_for_id(file_id);
            if node_id.0.is_empty() {
                fi.and_then(|i| self.files.get(i))
                    .map_or(String::new(), |f| f.data.note.clone())
            } else {
                fi.and_then(|i| {
                    let file = self.files.get(i)?;
                    let path = duir_core::tree_ops::find_node_path(&file.data, node_id)?;
                    duir_core::tree_ops::get_item(&file.data, &path).map(|item| item.note.clone())
                })
                .unwrap_or_default()
            }
        } else {
            return;
        };
        if let FocusState::Note { ref mut editor, .. } = self.state {
            **editor = crate::note_editor::NoteEditor::new(&note);
        }
    }

    /// Set a status message with a severity level for coloring.
    pub fn set_status(&mut self, msg: &str, level: StatusLevel) {
        msg.clone_into(&mut self.status_message);
        self.status_level = level;
    }

    /// Switch focus to note pane. Loads editor from model.
    pub fn focus_note(&mut self) {
        self.editor_cache.clear();
        if let Some(row) = self.current_row().cloned() {
            let note = self.current_note();
            let node_id = if row.is_file_root || row.path.is_empty() {
                NodeId(String::new())
            } else {
                duir_core::tree_ops::get_item(&self.files[row.file_index].data, &row.path)
                    .map_or_else(|| NodeId(String::new()), |item| item.id.clone())
            };
            self.state = FocusState::Note {
                editor: Box::new(crate::note_editor::NoteEditor::new(&note)),
                file_id: row.file_id,
                node_id,
            };
        }
    }

    /// Switch focus to tree pane. Saves editor to model if in Note state.
    pub fn focus_tree(&mut self) {
        self.save_editor();
        self.state = FocusState::Tree;
    }

    /// Helper: is the tree focused (no overlay active)?
    #[must_use]
    pub const fn is_tree_focused(&self) -> bool {
        matches!(self.state, FocusState::Tree)
    }

    /// Helper: is the note editor active?
    #[must_use]
    #[allow(dead_code)]
    pub const fn is_note_focused(&self) -> bool {
        matches!(self.state, FocusState::Note { .. })
    }

    /// Helper: is title editing active?
    #[must_use]
    pub const fn is_editing_title(&self) -> bool {
        matches!(self.state, FocusState::EditingTitle { .. })
    }

    /// Helper: is command mode active?
    #[must_use]
    pub const fn is_command_active(&self) -> bool {
        matches!(self.state, FocusState::Command { .. })
    }

    /// Helper: is filter mode active?
    #[must_use]
    pub const fn is_filter_active(&self) -> bool {
        matches!(self.state, FocusState::Filter { .. })
    }

    /// Helper: is help overlay shown?
    #[must_use]
    #[allow(dead_code)]
    pub const fn is_help_shown(&self) -> bool {
        matches!(self.state, FocusState::Help { .. })
    }

    /// Helper: is about overlay shown?
    #[must_use]
    pub const fn is_about_shown(&self) -> bool {
        matches!(self.state, FocusState::About)
    }
    /// Mark a file as modified and invalidate cipher caches for encrypted ancestors.
    pub(crate) fn mark_modified(&mut self, fi: usize, path: &[usize]) {
        self.files[fi].modified = true;
        for len in (1..=path.len()).rev() {
            let ancestor = &path[..len];
            if let Some(item) = duir_core::tree_ops::get_item_mut(&mut self.files[fi].data, &ancestor.to_vec()) {
                duir_core::crypto::invalidate_cipher(item);
            }
        }
        // Sync MCP snapshot if the modified node is inside an active kiron's subtree
        self.sync_mcp_snapshot(fi, path);
    }

    /// Update the MCP snapshot for any active kiron whose subtree contains the given path.
    fn sync_mcp_snapshot(&self, fi: usize, path: &[usize]) {
        let file_id = self.files[fi].id;
        for (key, kiron) in &self.active_kirons {
            if key.0 != file_id {
                continue;
            }
            let Some(kiron_path) = duir_core::tree_ops::find_node_path(&self.files[fi].data, &key.1) else {
                continue;
            };
            // Check if the modified path is within this kiron's subtree
            if !path.starts_with(&kiron_path) {
                continue;
            }
            let Some(ref snapshot) = kiron.mcp_snapshot else {
                continue;
            };
            if let Some(item) = duir_core::tree_ops::get_item(&self.files[fi].data, &kiron_path) {
                let mut file = TodoFile::new(&item.title);
                file.items.clone_from(&item.items);
                file.note.clone_from(&item.note);
                if let Ok(mut guard) = snapshot.lock() {
                    *guard = file;
                }
            }
        }
    }

    /// Mark a file as saved (only valid after successful save).
    fn mark_saved(&mut self, fi: usize) {
        self.files[fi].modified = false;
    }

    /// Mark a file as modified without invalidating cipher caches.
    /// Used for operations that don't change encrypted content (e.g., unlock).
    fn mark_file_modified(&mut self, fi: usize) {
        self.files[fi].modified = true;
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
            let file_id = self.files[fi].id;
            if let Some(item) = duir_core::tree_ops::get_item_mut(&mut self.files[fi].data, &path)
                && !item.folded
                && !item.items.is_empty()
            {
                // If unlocked encrypted node, re-encrypt and forget password
                if item.unlocked {
                    let node_id = item.id.clone();
                    let key = (file_id, node_id);
                    if let Some(pw) = self.passwords.get(&key)
                        && let Err(e) = duir_core::crypto::encrypt_item(item, pw)
                    {
                        self.set_status(&format!("Encrypt error, node stays unlocked: {e}"), StatusLevel::Error);
                        return;
                    }
                    self.passwords.remove(&key);
                } else {
                    item.folded = true;
                }
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
            // New sibling path: same parent, index + 1
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
                let needs_confirm = !item.items.is_empty() || item.completed != duir_core::Completion::Done;
                if needs_confirm {
                    self.pending_delete = true;
                    let reason = if item.items.is_empty() {
                        "incomplete task"
                    } else {
                        "branch with children"
                    };
                    self.set_status(&format!("Delete {reason}? y/n"), StatusLevel::Warning);
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
            // New clone path: same parent, index + 1
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

    pub fn finish_editing(&mut self) {
        if let FocusState::EditingTitle { ref buffer, .. } = self.state {
            let new_title = buffer.clone();
            if let Some(row) = self.rows.get(self.cursor).cloned()
                && !row.is_file_root
            {
                let fi = row.file_index;
                if let Some(item) = duir_core::tree_ops::get_item_mut(&mut self.files[fi].data, &row.path)
                    && item.title != new_title
                {
                    item.title.clone_from(&new_title);
                    self.mark_modified(fi, &row.path);
                }
            }
            self.state = FocusState::Tree;
            self.rebuild_rows();
        }
    }

    pub fn cancel_editing(&mut self) {
        if matches!(self.state, FocusState::EditingTitle { .. }) {
            self.state = FocusState::Tree;
        }
    }

    /// Execute a `:` command. Returns an optional path for file operations.
    pub fn execute_command(&mut self, storage: &dyn duir_core::TodoStorage) {
        // Extract command buffer from state
        let cmd = if let FocusState::Command { ref buffer, .. } = self.state {
            buffer.trim().to_owned()
        } else {
            return;
        };
        self.state = FocusState::Tree;

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
            "yank" => self.cmd_yank_tree(),
            "import" => self.cmd_import(&parts),
            "open" => self.cmd_open(&parts, storage),
            "write" => self.cmd_write(&parts, storage),
            "saveas" => self.cmd_saveas(&parts, storage),
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
                self.state = FocusState::Help { scroll: 0 };
            }
            "encrypt" => self.cmd_encrypt(),
            "decrypt" => self.cmd_decrypt(),
            "kiron" => self.cmd_kiron(&parts),
            "kiro" => self.cmd_kiro(&parts),
            "about" => {
                self.state = FocusState::About;
            }
            _ => {
                self.status_message = format!("Unknown command: {cmd}");
            }
        }
    }

    fn save_current(&mut self, storage: &dyn duir_core::TodoStorage) {
        if let Some(row) = self.current_row().cloned() {
            let fi = row.file_index;
            let file_id = self.files[fi].id;
            let pw_map: std::collections::HashMap<Vec<usize>, String> = self
                .passwords
                .iter()
                .filter(|((fid, _), _)| *fid == file_id)
                .filter_map(|((_, nid), pw)| {
                    duir_core::tree_ops::find_node_path(&self.files[fi].data, nid).map(|path| (path, pw.clone()))
                })
                .collect();

            let file = &mut self.files[fi];
            let saved = duir_core::crypto::lock_for_save(&mut file.data.items, &pw_map, &[]);
            match saved {
                Ok(saved_state) => {
                    let save_result = storage.save(&file.name, &file.data);
                    duir_core::crypto::restore_after_save(&mut file.data.items, &saved_state);
                    match save_result {
                        Ok(()) => {
                            let name = self.files[fi].name.clone();
                            self.mark_saved(fi);
                            self.status_message = format!("Saved {name}");
                        }
                        Err(e) => self.status_message = format!("Save error: {e}"),
                    }
                }
                Err(e) => self.status_message = format!("Encrypt error on save: {e}"),
            }
        }
    }

    pub(crate) fn close_current_file(&mut self) {
        if let Some(row) = self.current_row().cloned() {
            let fi = row.file_index;
            if self.files[fi].is_modified() {
                "File has unsaved changes. Use :q! to force".clone_into(&mut self.status_message);
                return;
            }
            let closed_id = self.files[fi].id;
            // Clean up active kirons and pending responses for the closed file
            self.active_kirons.retain(|k, _| k.0 != closed_id);
            self.pending_responses
                .retain(|pr| pr.kiron_file_id != closed_id && pr.prompt_file_id != closed_id);
            self.passwords.retain(|k, _| k.0 != closed_id);
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

    fn cmd_yank_tree(&mut self) {
        if let Some(item) = self.current_item() {
            let md = duir_core::markdown_export::export_subtree_safe(item, 3);
            let lines = md.lines().count();
            crate::clipboard::copy_to_clipboard(&md);
            self.status_message = format!("Yanked {lines} lines to clipboard (encrypted nodes redacted)");
        } else {
            "No item selected".clone_into(&mut self.status_message);
        }
    }
    pub(crate) fn cmd_export(&mut self, parts: &[&str]) {
        // :export [filename] — suffix determines format (.md default)
        let Some(item) = self.current_item() else {
            "No item selected".clone_into(&mut self.status_message);
            return;
        };

        let path = if let Some(&fname) = parts.get(1) {
            std::path::PathBuf::from(fname)
        } else {
            // Auto-generate from item title
            let slug: String = item
                .title
                .chars()
                .map(|c| {
                    if c.is_alphanumeric() {
                        c.to_ascii_lowercase()
                    } else {
                        '-'
                    }
                })
                .collect();
            let slug = slug.trim_matches('-').to_owned();
            let base = if slug.is_empty() { "export".to_owned() } else { slug };
            find_available_path(&format!("{base}.md"))
        };

        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("md");

        let path_s = path.to_string_lossy().to_string();
        match ext {
            "md" => {
                let md = duir_core::markdown_export::export_subtree(item, 3);
                match write_file(&path_s, md.as_bytes()) {
                    Ok(()) => self.set_status(&format!("Exported to {path_s}"), StatusLevel::Success),
                    Err(e) => self.set_status(&format!("Export error: {e}"), StatusLevel::Error),
                }
            }
            "docx" => match duir_core::docx_export::export_subtree_docx(item) {
                Ok(bytes) => match write_file(&path_s, &bytes) {
                    Ok(()) => self.set_status(&format!("Exported to {path_s}"), StatusLevel::Success),
                    Err(e) => self.set_status(&format!("Write error: {e}"), StatusLevel::Error),
                },
                Err(e) => self.set_status(&format!("DOCX error: {e}"), StatusLevel::Error),
            },
            _ => {
                self.status_message = format!("Unknown format: .{ext} (supported: .md, .docx)");
            }
        }
    }

    pub(crate) fn cmd_import(&mut self, parts: &[&str]) {
        // :import <file.md> or :import md <file.md> (backward compat)
        let path_str = if parts.len() >= 3 && parts[1] == "md" {
            parts[2]
        } else if parts.len() >= 2 {
            parts[1]
        } else {
            "Usage: :import <file.md>".clone_into(&mut self.status_message);
            return;
        };
        let path = std::path::Path::new(path_str);
        match read_file(path_str) {
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
                    self.mark_modified(fi, &row.path);
                    self.rebuild_rows();
                    self.set_status(&format!("Imported {}", path.display()), StatusLevel::Success);
                }
            }
            Err(e) => self.set_status(&format!("Import error: {e}"), StatusLevel::Error),
        }
    }

    fn cmd_open(&mut self, parts: &[&str], storage: &dyn duir_core::TodoStorage) {
        // :open <path> — open file as new top-level tree (auto-detect format)
        // Backward compat: :open md <file> still works
        let path_str = if parts.len() >= 3 && parts[1] == "md" {
            parts[2]
        } else if let Some(&p) = parts.get(1) {
            p
        } else {
            "Usage: :open <file>".clone_into(&mut self.status_message);
            return;
        };
        let path = std::path::Path::new(path_str);
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
        match ext {
            "md" => match read_file(path_str) {
                Ok(content) => {
                    let parsed = duir_core::markdown_import::import_markdown(&content);
                    let name = path.file_stem().and_then(|s| s.to_str()).unwrap_or("imported");
                    self.add_file(name.to_owned(), parsed);
                    self.set_status(&format!("Opened {name}"), StatusLevel::Success);
                }
                Err(e) => self.set_status(&format!("Open error: {e}"), StatusLevel::Error),
            },
            "todo" => match read_file(path_str) {
                Ok(content) => match duir_core::legacy_import::import_legacy_todo(&content) {
                    Ok(parsed) => {
                        let name = path.file_stem().and_then(|s| s.to_str()).unwrap_or("imported");
                        self.add_file(name.to_owned(), parsed);
                        self.set_status(&format!("Imported legacy {name}"), StatusLevel::Success);
                    }
                    Err(e) => self.set_status(&format!("Import error: {e}"), StatusLevel::Error),
                },
                Err(e) => self.set_status(&format!("Open error: {e}"), StatusLevel::Error),
            },
            _ => {
                self.open_file_path(path_str, storage);
            }
        }
    }

    fn cmd_write(&mut self, parts: &[&str], storage: &dyn duir_core::TodoStorage) {
        // :write <name> — save todo JSON to a different name (doesn't switch)
        if let Some(&name) = parts.get(1) {
            if let Some(row) = self.current_row().cloned() {
                let fi = row.file_index;
                match storage.save(name, &self.files[fi].data) {
                    Ok(()) => self.set_status(&format!("Written to {name}"), StatusLevel::Success),
                    Err(e) => self.set_status(&format!("Write error: {e}"), StatusLevel::Error),
                }
            }
        } else {
            "Usage: :write <name>".clone_into(&mut self.status_message);
        }
    }

    fn cmd_saveas(&mut self, parts: &[&str], storage: &dyn duir_core::TodoStorage) {
        // :saveas <name> — save current file under new name and switch to it
        if let Some(&name) = parts.get(1) {
            if let Some(row) = self.current_row().cloned() {
                let fi = row.file_index;
                match storage.save(name, &self.files[fi].data) {
                    Ok(()) => {
                        name.clone_into(&mut self.files[fi].name);
                        self.mark_saved(fi);
                        self.rebuild_rows();
                        self.set_status(&format!("Saved as {name}"), StatusLevel::Success);
                    }
                    Err(e) => self.set_status(&format!("Save error: {e}"), StatusLevel::Error),
                }
            }
        } else {
            "Usage: :saveas <name>".clone_into(&mut self.status_message);
        }
    }

    pub(crate) fn cmd_collapse(&mut self) {
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
                self.mark_modified(fi, &row.path);
                self.rebuild_rows();
                self.reload_editor();
                "Children collapsed to note".clone_into(&mut self.status_message);
            }
        }
    }

    pub(crate) fn cmd_expand(&mut self) {
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
                self.mark_modified(fi, &row.path);
                self.rebuild_rows();
                self.reload_editor();
                "Note expanded to children".clone_into(&mut self.status_message);
            }
        }
    }

    pub(crate) fn cmd_encrypt(&mut self) {
        self.save_editor();
        if let Some(row) = self.rows.get(self.cursor).cloned() {
            if row.is_file_root {
                "Cannot encrypt file root".clone_into(&mut self.status_message);
                return;
            }
            let fi = row.file_index;
            let item = duir_core::tree_ops::get_item(&self.files[fi].data, &row.path);
            let already_encrypted = item.is_some_and(duir_core::TodoItem::is_encrypted);
            let node_id = item.map_or_else(|| NodeId(String::new()), |it| it.id.clone());
            let file_id = self.files[fi].id;
            let title = if already_encrypted {
                "Change encryption password"
            } else {
                "Encrypt subtree"
            };
            let action = if already_encrypted {
                crate::password::PasswordAction::ChangePassword { file_id, node_id }
            } else {
                crate::password::PasswordAction::Encrypt { file_id, node_id }
            };
            self.password_prompt = Some(crate::password::PasswordPrompt::new(title, action));
        }
    }

    pub(crate) fn cmd_decrypt(&mut self) {
        self.save_editor();
        if let Some(row) = self.rows.get(self.cursor).cloned() {
            if row.is_file_root {
                return;
            }
            let fi = row.file_index;
            if let Some(item) = duir_core::tree_ops::get_item_mut(&mut self.files[fi].data, &row.path) {
                if !item.is_encrypted() {
                    self.set_status("Node is not encrypted", StatusLevel::Warning);
                    return;
                }
                if !item.unlocked {
                    self.set_status("Unlock first: press → and enter password", StatusLevel::Warning);
                    return;
                }
                let node_id = item.id.clone();
                duir_core::crypto::strip_encryption(item);
                self.passwords.remove(&(self.files[fi].id, node_id));
                self.mark_modified(fi, &row.path);
                self.rebuild_rows();
                self.set_status("Encryption removed", StatusLevel::Success);
            }
        }
    }

    /// Handle password prompt result.
    pub fn handle_password_result(&mut self, password: &str, action: crate::password::PasswordAction) {
        match action {
            crate::password::PasswordAction::Encrypt { file_id, node_id } => {
                let Some(fi) = self.file_index_for_id(file_id) else {
                    return;
                };
                let Some(path) = duir_core::tree_ops::find_node_path(&self.files[fi].data, &node_id) else {
                    return;
                };
                if let Some(item) = duir_core::tree_ops::get_item_mut(&mut self.files[fi].data, &path) {
                    match duir_core::crypto::encrypt_item(item, password) {
                        Ok(()) => {
                            self.passwords.insert((file_id, node_id), password.to_owned());
                            self.mark_modified(fi, &path);
                            self.rebuild_rows();
                            self.set_status("Subtree encrypted", StatusLevel::Success);
                        }
                        Err(e) => self.status_message = format!("Encrypt error: {e}"),
                    }
                }
            }
            crate::password::PasswordAction::Decrypt { file_id, node_id } => {
                let Some(fi) = self.file_index_for_id(file_id) else {
                    return;
                };
                let Some(path) = duir_core::tree_ops::find_node_path(&self.files[fi].data, &node_id) else {
                    return;
                };
                if let Some(item) = duir_core::tree_ops::get_item_mut(&mut self.files[fi].data, &path) {
                    match duir_core::crypto::decrypt_item(item, password) {
                        Ok(()) => {
                            self.passwords.insert((file_id, node_id), password.to_owned());
                            self.mark_file_modified(fi);
                            self.rebuild_rows();
                            self.set_status("Subtree unlocked", StatusLevel::Success);
                        }
                        Err(_) => self.set_status("Wrong password", StatusLevel::Error),
                    }
                }
            }
            crate::password::PasswordAction::ChangePassword { file_id, node_id } => {
                self.save_editor();
                let Some(fi) = self.file_index_for_id(file_id) else {
                    return;
                };
                let Some(path) = duir_core::tree_ops::find_node_path(&self.files[fi].data, &node_id) else {
                    return;
                };
                if let Some(item) = duir_core::tree_ops::get_item_mut(&mut self.files[fi].data, &path) {
                    match duir_core::crypto::encrypt_item(item, password) {
                        Ok(()) => {
                            self.passwords.insert((file_id, node_id), password.to_owned());
                            self.mark_modified(fi, &path);
                            self.rebuild_rows();
                            self.set_status("Password changed", StatusLevel::Success);
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
            let item = duir_core::tree_ops::get_item(&self.files[fi].data, &row.path);
            if item.is_some_and(duir_core::model::TodoItem::is_locked) {
                let node_id = item.map_or_else(|| NodeId(String::new()), |it| it.id.clone());
                self.password_prompt = Some(crate::password::PasswordPrompt::new(
                    "Unlock encrypted node",
                    crate::password::PasswordAction::Decrypt {
                        file_id: self.files[fi].id,
                        node_id,
                    },
                ));
            }
        }
    }

    pub(crate) fn cmd_autosave(&mut self, parts: &[&str]) {
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

    /// Mark or disable a kiron on the current node.
    pub(crate) fn cmd_kiron(&mut self, parts: &[&str]) {
        if parts.get(1).copied() == Some("disable") {
            self.kiron_disable();
            return;
        }
        let Some(row) = self.current_row().cloned() else {
            "No node selected".clone_into(&mut self.status_message);
            return;
        };
        let fi = row.file_index;
        let path = &row.path;
        let item = if path.is_empty() {
            "Cannot mark file root as kiron".clone_into(&mut self.status_message);
            return;
        } else {
            duir_core::tree_ops::get_item_mut(&mut self.files[fi].data, path)
        };
        let Some(item) = item else {
            "Node not found".clone_into(&mut self.status_message);
            return;
        };
        if item.is_kiron() {
            self.set_status("Already a kiron", StatusLevel::Warning);
            return;
        }
        let session_id = uuid::Uuid::new_v4().to_string();
        item.node_type = Some(duir_core::NodeType::Kiron);
        item.kiron = Some(duir_core::KironMeta {
            session_id: session_id.clone(),
        });
        self.mark_modified(fi, path);
        self.rebuild_rows();
        self.set_status(
            &format!("Marked as kiron (session {})", &session_id[..8]),
            StatusLevel::Success,
        );
    }

    fn kiron_disable(&mut self) {
        let Some(row) = self.current_row().cloned() else {
            "No node selected".clone_into(&mut self.status_message);
            return;
        };
        let fi = row.file_index;
        let path = row.path;
        // Block if active
        let item = if path.is_empty() {
            None
        } else {
            duir_core::tree_ops::get_item(&self.files[fi].data, &path)
        };
        let node_id = item.map_or_else(|| NodeId(String::new()), |it| it.id.clone());
        if self.active_kirons.contains_key(&(self.files[fi].id, node_id)) {
            self.set_status("Stop kiro first (:kiro stop)", StatusLevel::Error);
            return;
        }
        let item = if path.is_empty() {
            None
        } else {
            duir_core::tree_ops::get_item_mut(&mut self.files[fi].data, &path)
        };
        let Some(item) = item else {
            "Node not found".clone_into(&mut self.status_message);
            return;
        };
        if !item.is_kiron() {
            self.set_status("Not a kiron node", StatusLevel::Warning);
            return;
        }
        item.node_type = None;
        item.kiron = None;
        self.mark_modified(fi, &path);
        self.rebuild_rows();
        self.set_status("Kiron disabled", StatusLevel::Success);
    }

    /// Start or stop a kiro session on the current kiron node.
    pub(crate) fn cmd_kiro(&mut self, parts: &[&str]) {
        let subcmd = parts.get(1).copied().unwrap_or("");
        match subcmd {
            "start" => self.kiro_start(),
            "stop" => self.kiro_stop(),
            _ => {
                "Usage: :kiro start | :kiro stop".clone_into(&mut self.status_message);
            }
        }
    }

    fn kiro_start(&mut self) {
        let Some(row) = self.current_row().cloned() else {
            "No node selected".clone_into(&mut self.status_message);
            return;
        };
        let fi = row.file_index;
        let path = row.path;
        let item = if path.is_empty() {
            None
        } else {
            duir_core::tree_ops::get_item(&self.files[fi].data, &path)
        };
        let Some(item) = item else {
            "Node not found".clone_into(&mut self.status_message);
            return;
        };
        if !item.is_kiron() {
            self.set_status("Not a kiron node. Use :kiron first", StatusLevel::Error);
            return;
        }
        let file_id = self.files[fi].id;
        let node_id = item.id.clone();
        let key = (file_id, node_id);
        if self.active_kirons.contains_key(&key) {
            self.set_status("Kiron already active", StatusLevel::Warning);
            return;
        }
        let config = duir_core::config::Config::load();
        let (cmd, args) = config.kiro.build_command(std::path::Path::new("."));
        let arg_refs: Vec<&str> = args.iter().map(String::as_str).collect();
        let cwd = std::env::current_dir().unwrap_or_default();
        match crate::pty_tab::PtyTab::spawn(&cmd, &arg_refs, 80, 24, &cwd) {
            Ok(pty) => {
                // Create MCP snapshot from kiron subtree
                let subtree = duir_core::tree_ops::get_item(&self.files[fi].data, &path).map_or_else(
                    || TodoFile::new("kiron"),
                    |item| {
                        let mut file = TodoFile::new(&item.title);
                        file.items.clone_from(&item.items);
                        file.note.clone_from(&item.note);
                        file
                    },
                );
                let snapshot = Arc::new(Mutex::new(subtree));
                let (tx, rx) = std::sync::mpsc::channel();
                let snap_clone = Arc::clone(&snapshot);
                std::thread::spawn(move || {
                    let server = duir_core::mcp_server::McpServer::new(snap_clone, tx);
                    let _ = server.run_stdio();
                });
                self.active_kirons.insert(
                    key,
                    ActiveKiron {
                        pty,
                        mcp_mutations: Some(rx),
                        mcp_snapshot: Some(snapshot),
                    },
                );
                self.kiro_tab_focused = true;
                self.set_status("Kiro session started", StatusLevel::Success);
            }
            Err(e) => {
                self.set_status(&format!("Failed to start kiro: {e}"), StatusLevel::Error);
            }
        }
    }

    fn kiro_stop(&mut self) {
        let Some(row) = self.current_row().cloned() else {
            "No node selected".clone_into(&mut self.status_message);
            return;
        };
        let fi = row.file_index;
        let node_id = if row.path.is_empty() {
            NodeId(String::new())
        } else {
            duir_core::tree_ops::get_item(&self.files[fi].data, &row.path)
                .map_or_else(|| NodeId(String::new()), |it| it.id.clone())
        };
        let key = (self.files[fi].id, node_id);
        if self.active_kirons.remove(&key).is_some() {
            self.kiro_tab_focused = false;
            self.set_status("Kiro session stopped", StatusLevel::Success);
        } else {
            self.set_status("No active kiro session on this node", StatusLevel::Warning);
        }
    }

    /// Find the active kiron for the current cursor position.
    /// Returns the key of the most specific (deepest) active kiron
    /// whose subtree contains the current node.
    pub fn active_kiron_for_cursor(&self) -> Option<(FileId, NodeId)> {
        let row = self.current_row()?;
        let fi = row.file_index;
        let file_id = self.files[fi].id;
        let path = &row.path;

        // Check the current node and each ancestor
        let mut best: Option<(&(FileId, NodeId), usize)> = None;
        for len in (1..=path.len()).rev() {
            let ancestor_path = &path[..len];
            if let Some(item) = duir_core::tree_ops::get_item(&self.files[fi].data, &ancestor_path.to_vec()) {
                let key_candidate = (file_id, item.id.clone());
                if self.active_kirons.contains_key(&key_candidate) {
                    // Find the actual key reference in the map
                    for key in self.active_kirons.keys() {
                        if *key == key_candidate && best.as_ref().is_none_or(|(_, d)| len > *d) {
                            best = Some((key, len));
                            break;
                        }
                    }
                }
            }
        }
        best.map(|(k, _)| k.clone())
    }

    /// Poll all active kiron PTYs for new output.
    pub fn poll_kirons(&mut self) {
        for kiron in self.active_kirons.values_mut() {
            kiron.pty.poll();
        }
    }

    /// Send the current node's content as a prompt to the active kiron's PTY.
    pub(crate) fn send_to_kiro(&mut self) {
        let Some(row) = self.current_row().cloned() else {
            return;
        };
        let Some(kiron_key) = self.active_kiron_for_cursor() else {
            return;
        };
        let fi = row.file_index;
        let path = row.path;

        // Get node content as markdown
        let content = if path.is_empty() {
            duir_core::markdown_export::export_file(&self.files[fi].data)
        } else {
            let Some(item) = duir_core::tree_ops::get_item(&self.files[fi].data, &path) else {
                return;
            };
            duir_core::markdown_export::export_subtree_safe(item, 3)
        };

        // Write to PTY as bracketed paste
        let Some(kiron) = self.active_kirons.get_mut(&kiron_key) else {
            return;
        };
        let mut payload = String::with_capacity(content.len() + 16);
        payload.push_str("\x1b[200~");
        payload.push_str(&content);
        payload.push_str("\x1b[201~");
        payload.push('\n');
        kiron.pty.write(payload.as_bytes());

        // Mark node as prompt type
        let prompt_node_id = if path.is_empty() {
            NodeId(String::new())
        } else if let Some(item) = duir_core::tree_ops::get_item_mut(&mut self.files[fi].data, &path) {
            item.node_type = Some(duir_core::NodeType::Prompt);
            if !item.title.starts_with("📤 ") {
                item.title = format!("📤 {}", item.title);
            }
            item.id.clone()
        } else {
            NodeId(String::new())
        };
        if !path.is_empty() {
            self.mark_modified(fi, &path);
        }

        // Record pending response
        self.pending_responses.push(PendingResponse {
            kiron_file_id: kiron_key.0,
            kiron_node_id: kiron_key.1,
            prompt_file_id: self.files[fi].id,
            prompt_node_id,
            start_time: std::time::Instant::now(),
        });

        self.rebuild_rows();
        self.set_status("Prompt sent to kiro", StatusLevel::Success);
    }

    /// Check pending responses for idle PTYs and capture output.
    pub(crate) fn check_response_capture(&mut self) {
        let idle_threshold = std::time::Duration::from_secs(5);
        let now = std::time::Instant::now();

        // Collect indices of completed responses
        let mut completed = Vec::new();
        for (i, pr) in self.pending_responses.iter().enumerate() {
            if now.duration_since(pr.start_time) < idle_threshold {
                continue;
            }
            let key = (pr.kiron_file_id, pr.kiron_node_id.clone());
            let Some(kiron) = self.active_kirons.get(&key) else {
                completed.push(i);
                continue;
            };
            let output = crate::termbuf::extract_last_output(&kiron.pty.termbuf);
            if output.trim().is_empty() {
                continue;
            }
            completed.push(i);
        }

        // Process in reverse to preserve indices
        for &i in completed.iter().rev() {
            let pr = self.pending_responses.remove(i);
            let key = (pr.kiron_file_id, pr.kiron_node_id.clone());
            let Some(kiron) = self.active_kirons.get(&key) else {
                continue;
            };
            let output = crate::termbuf::extract_last_output(&kiron.pty.termbuf);
            if output.trim().is_empty() {
                continue;
            }

            // Build response title from first non-empty line
            let first_line = output.lines().find(|l| !l.trim().is_empty()).unwrap_or("Response");
            let truncated: String = first_line.chars().take(80).collect();
            let title = format!("📥 {truncated}");

            // Resolve kiron file index and path for session_id lookup
            let Some(kiron_fi) = self.file_index_for_id(pr.kiron_file_id) else {
                continue;
            };
            let Some(kiron_path) = duir_core::tree_ops::find_node_path(&self.files[kiron_fi].data, &pr.kiron_node_id)
            else {
                continue;
            };

            let session_id = duir_core::tree_ops::get_item(&self.files[kiron_fi].data, &kiron_path)
                .and_then(|item| item.kiron.as_ref())
                .map_or_else(|| "unknown".to_owned(), |k| k.session_id.clone());

            let timestamp = chrono::Utc::now().to_rfc3339();
            let note = format!(
                "<!-- duir:response kiron={session_id} timestamp={timestamp} -->\n\
                 {output}"
            );

            let mut response_node = duir_core::TodoItem::new(&title);
            response_node.note = note;
            response_node.node_type = Some(duir_core::NodeType::Response);

            // Resolve prompt file index and path
            let Some(prompt_fi) = self.file_index_for_id(pr.prompt_file_id) else {
                continue;
            };
            let Some(prompt_path) =
                duir_core::tree_ops::find_node_path(&self.files[prompt_fi].data, &pr.prompt_node_id)
            else {
                continue;
            };

            if let Err(e) =
                duir_core::tree_ops::add_sibling(&mut self.files[prompt_fi].data, &prompt_path, response_node)
            {
                self.set_status(&format!("Failed to insert response: {e}"), StatusLevel::Error);
                continue;
            }
            self.mark_modified(prompt_fi, &prompt_path);
        }

        if !completed.is_empty() {
            self.rebuild_rows();
        }
    }

    /// Drain MCP mutation channels and apply to the actual tree model.
    pub fn process_mcp_mutations(&mut self) {
        let keys: Vec<_> = self.active_kirons.keys().cloned().collect();
        let mut changed = false;
        for key in keys {
            let Some(kiron) = self.active_kirons.get(&key) else {
                continue;
            };
            let Some(rx) = kiron.mcp_mutations.as_ref() else {
                continue;
            };
            let mutations: Vec<McpMutation> = rx.try_iter().collect();
            let (file_id, ref node_id) = key;
            let Some(fi) = self.file_index_for_id(file_id) else {
                continue;
            };
            if fi >= self.files.len() {
                continue;
            }
            let Some(base_path) = duir_core::tree_ops::find_node_path(&self.files[fi].data, node_id) else {
                continue;
            };
            for mutation in mutations {
                match mutation {
                    McpMutation::AddChild {
                        parent_path,
                        title,
                        note,
                    } => {
                        let abs = absolute_path(&base_path, &parent_path);
                        let mut item = TodoItem::new(&title);
                        item.note = note;
                        if duir_core::tree_ops::add_child(&mut self.files[fi].data, &abs, item).is_ok() {
                            changed = true;
                            self.mark_modified(fi, &abs);
                        }
                    }
                    McpMutation::AddSibling { path, title, note } => {
                        let abs = absolute_path(&base_path, &path);
                        let mut item = TodoItem::new(&title);
                        item.note = note;
                        if duir_core::tree_ops::add_sibling(&mut self.files[fi].data, &abs, item).is_ok() {
                            changed = true;
                            self.mark_modified(fi, &abs);
                        }
                    }
                    McpMutation::MarkDone { path } => {
                        let abs = absolute_path(&base_path, &path);
                        if let Some(item) = duir_core::tree_ops::get_item_mut(&mut self.files[fi].data, &abs) {
                            item.completed = Completion::Done;
                            changed = true;
                            self.mark_modified(fi, &abs);
                        }
                    }
                    McpMutation::MarkImportant { path } => {
                        let abs = absolute_path(&base_path, &path);
                        if let Some(item) = duir_core::tree_ops::get_item_mut(&mut self.files[fi].data, &abs) {
                            item.important = !item.important;
                            changed = true;
                            self.mark_modified(fi, &abs);
                        }
                    }
                    McpMutation::Reorder { path, direction } => {
                        let abs = absolute_path(&base_path, &path);
                        let result = match direction {
                            duir_core::mcp_server::ReorderDirection::Up => {
                                duir_core::tree_ops::swap_up(&mut self.files[fi].data, &abs)
                            }
                            duir_core::mcp_server::ReorderDirection::Down => {
                                duir_core::tree_ops::swap_down(&mut self.files[fi].data, &abs)
                            }
                        };
                        if result.is_ok() {
                            changed = true;
                            self.mark_modified(fi, &abs);
                        }
                    }
                }
            }
        }
        if changed {
            self.rebuild_rows();
        }
    }

    pub fn save_all(&mut self, storage: &dyn duir_core::TodoStorage) {
        let mut errors: Vec<String> = Vec::new();
        for file in &mut self.files {
            if !file.is_modified() {
                continue;
            }
            // Build password map for this file (resolve NodeId to path)
            let pw_map: std::collections::HashMap<Vec<usize>, String> = self
                .passwords
                .iter()
                .filter(|((fid, _), _)| *fid == file.id)
                .filter_map(|((_, nid), pw)| {
                    duir_core::tree_ops::find_node_path(&file.data, nid).map(|path| (path, pw.clone()))
                })
                .collect();

            // Re-encrypt unlocked nodes for save
            let saved = duir_core::crypto::lock_for_save(&mut file.data.items, &pw_map, &[]);

            match saved {
                Ok(saved_state) => {
                    match storage.save(&file.name, &file.data) {
                        Ok(()) => file.modified = false,
                        Err(e) => errors.push(format!("{}: {e}", file.name)),
                    }
                    // Restore decrypted state in memory
                    duir_core::crypto::restore_after_save(&mut file.data.items, &saved_state);
                }
                Err(e) => {
                    errors.push(format!("{}: encrypt error: {e}", file.name));
                }
            }
        }
        if errors.is_empty() {
            self.set_status("Saved", StatusLevel::Success);
        } else {
            self.set_status(&format!("Save errors: {}", errors.join("; ")), StatusLevel::Error);
        }
    }

    /// Check if any file has unsaved modifications.
    #[must_use]
    pub fn has_unsaved(&self) -> bool {
        self.files.iter().any(LoadedFile::is_modified)
    }

    /// Apply the current committed filter text, searching titles and notes.
    pub fn apply_filter(&mut self) {
        self.rebuild_rows_raw();
        self.reapply_filter();
    }

    fn reapply_filter(&mut self) {
        if self.filter_committed_text.is_empty() {
            return;
        }

        let opts = duir_core::filter::FilterOptions {
            search_notes: true,
            case_sensitive: false,
        };

        let mut match_set: std::collections::HashSet<(usize, Vec<usize>)> = std::collections::HashSet::new();
        for (fi, file) in self.files.iter().enumerate() {
            let matches = duir_core::filter::filter_items(&file.data.items, &self.filter_committed_text, &opts);
            for path in matches {
                match_set.insert((fi, path));
            }
        }

        if self.filter_committed_exclude {
            self.rows
                .retain(|row| row.is_file_root || !match_set.contains(&(row.file_index, row.path.clone())));
        } else {
            self.rows
                .retain(|row| row.is_file_root || match_set.contains(&(row.file_index, row.path.clone())));
        }

        let visible = self.rows.iter().filter(|r| !r.is_file_root).count();
        let mode = if self.filter_committed_exclude {
            "exclude"
        } else {
            "include"
        };
        self.status_message = format!(
            "Filter '{}' ({}): {} visible",
            self.filter_committed_text, mode, visible
        );

        if self.cursor >= self.rows.len() && !self.rows.is_empty() {
            self.cursor = self.rows.len() - 1;
        }
    }

    /// Live filter — called on each keystroke while typing the filter.
    pub fn apply_filter_live(&mut self) {
        let filter_text = if let FocusState::Filter { ref text, .. } = self.state {
            text.clone()
        } else {
            return;
        };

        if filter_text.is_empty() {
            self.rebuild_rows_raw();
            self.status_message.clear();
            return;
        }
        let (text, exclude) = filter_text
            .strip_prefix('!')
            .map_or_else(|| (filter_text.clone(), false), |rest| (rest.to_owned(), true));
        if text.is_empty() {
            self.rebuild_rows_raw();
            return;
        }

        self.rebuild_rows_raw();
        let opts = duir_core::filter::FilterOptions {
            search_notes: true,
            case_sensitive: false,
        };
        let mut match_set: std::collections::HashSet<(usize, Vec<usize>)> = std::collections::HashSet::new();
        for (fi, file) in self.files.iter().enumerate() {
            let matches = duir_core::filter::filter_items(&file.data.items, &text, &opts);
            for path in matches {
                match_set.insert((fi, path));
            }
        }
        if exclude {
            self.rows
                .retain(|row| row.is_file_root || !match_set.contains(&(row.file_index, row.path.clone())));
        } else {
            self.rows
                .retain(|row| row.is_file_root || match_set.contains(&(row.file_index, row.path.clone())));
        }
        let visible = self.rows.iter().filter(|r| !r.is_file_root).count();
        self.status_message = format!("/{filter_text}: {visible} matches");
        if self.cursor >= self.rows.len() && !self.rows.is_empty() {
            self.cursor = self.rows.len() - 1;
        }
    }
}

/// Read a file from local filesystem or S3.
fn read_file(path: &str) -> Result<String, String> {
    if duir_core::s3_storage::S3Path::is_s3(path) {
        let s3path = duir_core::s3_storage::S3Path::parse(path).ok_or("Invalid S3 path")?;
        let s3 = duir_core::s3_storage::S3Storage::new().map_err(|e| format!("{e}"))?;
        let bytes = s3.read_bytes(&s3path.bucket, &s3path.key).map_err(|e| format!("{e}"))?;
        String::from_utf8(bytes).map_err(|e| format!("{e}"))
    } else {
        std::fs::read_to_string(path).map_err(|e| format!("{e}"))
    }
}

/// Write bytes to local filesystem or S3.
fn write_file(path: &str, data: &[u8]) -> Result<(), String> {
    if duir_core::s3_storage::S3Path::is_s3(path) {
        let s3path = duir_core::s3_storage::S3Path::parse(path).ok_or("Invalid S3 path")?;
        let s3 = duir_core::s3_storage::S3Storage::new().map_err(|e| format!("{e}"))?;
        s3.write_bytes(&s3path.bucket, &s3path.key, data.to_vec())
            .map_err(|e| format!("{e}"))
    } else {
        std::fs::write(path, data).map_err(|e| format!("{e}"))
    }
}
/// Translate a relative MCP path to an absolute tree path by prepending the kiron base.
fn absolute_path(base: &[usize], relative: &[usize]) -> Vec<usize> {
    let mut abs = base.to_vec();
    abs.extend_from_slice(relative);
    abs
}

fn find_available_path(base: &str) -> std::path::PathBuf {
    let path = std::path::PathBuf::from(base);
    if !path.exists() {
        return path;
    }
    let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("export");
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("md");
    for i in 1..100 {
        let candidate = std::path::PathBuf::from(format!("{stem}.{i}.{ext}"));
        if !candidate.exists() {
            return candidate;
        }
    }
    std::path::PathBuf::from(format!("{stem}.99.{ext}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn file_id_unique() {
        let mut app = App::new();
        app.add_empty_file("a");
        app.add_empty_file("b");
        assert_ne!(app.files[0].id, app.files[1].id);
    }

    #[test]
    fn file_id_survives_close() {
        let mut app = App::new();
        app.add_empty_file("a");
        app.add_empty_file("b");
        app.add_empty_file("c");
        let id_a = app.files[0].id;
        let id_c = app.files[2].id;
        // Remove middle file
        app.files.remove(1);
        app.rebuild_rows();
        assert_eq!(app.files[0].id, id_a);
        assert_eq!(app.files[1].id, id_c);
    }
}
