//! Session persistence — save/restore layout state between runs.

use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};

/// Saved session state.
#[derive(Default, Serialize, Deserialize)]
pub struct SessionState {
    /// Which panel is zoomed (None = unzoomed).
    pub zoomed_panel: Option<usize>,
    /// Focused panel index.
    pub focused_panel: usize,
    /// Tree cursor position.
    pub tree_cursor: usize,
    /// Whether timestamps are visible.
    pub show_timestamps: bool,
}

const SESSION_FILE: &str = ".duir/session.json";

/// Load session state from disk.
pub fn load_session(root_dir: &Path) -> SessionState {
    let path = root_dir.join(SESSION_FILE);
    fs::read_to_string(&path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

/// Save session state to disk.
pub fn save_session(root_dir: &Path, state: &SessionState) {
    let path = root_dir.join(SESSION_FILE);
    if let Ok(content) = serde_json::to_string_pretty(state) {
        let _ = fs::write(&path, content);
    }
}
