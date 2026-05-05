mod app_commands;
mod app_commands_file;
mod app_commands_misc;
mod app_commands_save;
mod app_crypto;
mod app_editor;
mod app_editor_filter;
mod app_files;
mod app_flags;
mod app_io;
pub mod app_kiron;
pub mod app_kiron_capture;
pub mod app_kiron_mcp;
pub mod app_kiron_mutation;
mod app_kiron_start;
mod app_resolve;
pub use app_resolve::ConflictState;
mod app_state;
mod app_tree;
mod app_tree_edit;
mod app_tree_move;
mod tree_row;

pub use app_flags::AppFlags;
pub use app_io::{find_available_path, read_file, write_file};
pub use app_state::App;
pub use tree_row::{FileSource, LoadedFile, TreeRow, TreeRowFlags};

use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use duir_core::mcp_server::McpMutation;
use duir_core::{NodeId, TodoFile};

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
    Resolve(app_resolve::ConflictState),
}

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
