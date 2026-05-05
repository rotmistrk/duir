mod app_commands;
mod app_commands_file;
mod app_crypto;
mod app_editor;
mod app_io;
pub mod app_kiron;
pub mod app_kiron_capture;
pub mod app_kiron_mcp;
mod app_tree;
mod app_tree_move;

pub use app_io::{find_available_path, read_file, write_file};

use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use duir_core::mcp_server::McpMutation;
use duir_core::tree_ops::TreePath;
use duir_core::{Completion, NodeId, TodoFile, TodoItem};

/// Stable file identity — monotonic, never reused, survives reorder/close.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FileId(pub u64);

/// An active kiron session: PTY process + optional MCP server.
pub struct ActiveKiron {
    pub pty: crate::pty_tab::PtyTab,
    /// True when kiro has gone idle after receiving output (response likely ready).
    pub response_ready: bool,
    /// Whether the PTY produced output during the last poll cycle.
    pub had_output: bool,
    /// Shared snapshot of the kiron subtree for the MCP server.
    pub mcp_snapshot: Option<Arc<Mutex<TodoFile>>>,
    /// Channel receiving mutations from the MCP server thread.
    pub mutation_rx: Option<std::sync::mpsc::Receiver<McpMutation>>,
    /// Unix socket path for cleanup on stop.
    pub socket_path: Option<PathBuf>,
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
    pub kiro_active: bool,
}

/// Loaded file with its data and metadata.
#[derive(Debug)]
pub struct LoadedFile {
    pub id: FileId,
    pub name: String,
    pub data: TodoFile,
    pub(crate) modified: bool,
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
    Kiro,
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
        search: String,
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

/// A pending response capture: tracks the buffer position at which a prompt
/// was sent so that everything after it can be captured as the response.
pub struct PendingResponse {
    pub kiron_file_id: FileId,
    pub kiron_node_id: NodeId,
    pub prompt_file_id: FileId,
    pub prompt_node_id: NodeId,
    /// Line number in the combined scrollback+grid at the time the prompt was sent.
    pub capture_start_line: usize,
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
    /// Whether the right panel shows Kiro (true) or Note (false).
    pub kiro_tab_focused: bool,
    /// Pending response captures awaiting idle timeout.
    pub pending_responses: Vec<PendingResponse>,
    /// Zoom: show focused panel fullscreen with no border.
    pub zoomed: bool,
    /// Keyboard label mode: "mac" shows ⌥, "linux" shows Alt.
    pub kbd_mac: bool,
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
            zoomed: false,
            kbd_mac: app_io::detect_mac_terminal(),
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

    #[must_use]
    pub fn current_row(&self) -> Option<&TreeRow> {
        self.rows.get(self.cursor)
    }

    #[must_use]
    pub fn current_item(&self) -> Option<&TodoItem> {
        let row = self.current_row()?;
        if row.is_file_root {
            return None;
        }
        let file = self.files.get(row.file_index)?;
        duir_core::tree_ops::get_item(&file.data, &row.path)
    }

    #[must_use]
    pub fn current_note(&self) -> String {
        self.current_row().map_or_else(String::new, |row| {
            if row.is_file_root {
                self.files
                    .get(row.file_index)
                    .map_or_else(String::new, |f| f.data.note.clone())
            } else if let Some(item) = self.current_item() {
                item.note.clone()
            } else {
                String::new()
            }
        })
    }

    pub fn set_status(&mut self, msg: &str, level: StatusLevel) {
        msg.clone_into(&mut self.status_message);
        self.status_level = level;
    }

    #[must_use]
    pub const fn is_tree_focused(&self) -> bool {
        matches!(self.state, FocusState::Tree)
    }

    #[must_use]
    pub const fn is_kiro_focused(&self) -> bool {
        matches!(self.state, FocusState::Kiro)
    }

    #[must_use]
    #[allow(dead_code)]
    pub const fn is_note_focused(&self) -> bool {
        matches!(self.state, FocusState::Note { .. })
    }

    #[must_use]
    pub const fn is_editing_title(&self) -> bool {
        matches!(self.state, FocusState::EditingTitle { .. })
    }

    #[must_use]
    pub const fn is_command_active(&self) -> bool {
        matches!(self.state, FocusState::Command { .. })
    }

    #[must_use]
    pub const fn is_filter_active(&self) -> bool {
        matches!(self.state, FocusState::Filter { .. })
    }

    #[must_use]
    #[allow(dead_code)]
    pub const fn is_help_shown(&self) -> bool {
        matches!(self.state, FocusState::Help { .. })
    }

    #[must_use]
    pub const fn is_about_shown(&self) -> bool {
        matches!(self.state, FocusState::About)
    }
}

#[cfg(test)]
#[allow(clippy::indexing_slicing)]
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
        app.files.remove(1);
        app.rebuild_rows();
        assert_eq!(app.files[0].id, id_a);
        assert_eq!(app.files[1].id, id_c);
    }
}
