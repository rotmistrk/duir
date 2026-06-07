//! MCP permissions — gate tool access by category.

use std::fs;
use std::path::Path;

use serde::Deserialize;

/// Permission categories for MCP tools.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolCategory {
    Read,
    Write,
    Navigate,
    Execute,
}

/// Permission configuration.
#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
#[allow(clippy::struct_excessive_bools)]
pub struct Permissions {
    pub read: bool,
    pub write: bool,
    pub navigate: bool,
    pub execute: bool,
    /// Per-tool overrides (`tool_name` → allowed).
    #[serde(default)]
    pub tools: std::collections::HashMap<String, bool>,
}

impl Default for Permissions {
    fn default() -> Self {
        Self {
            read: true,
            write: false,
            navigate: true,
            execute: false,
            tools: std::collections::HashMap::new(),
        }
    }
}

impl Permissions {
    /// Load from `.duir/mcp-permissions.toml`, falling back to defaults.
    pub fn load(root_dir: &Path) -> Self {
        let path = root_dir.join(".duir").join("mcp-permissions.toml");
        fs::read_to_string(&path)
            .ok()
            .and_then(|s| toml::from_str(&s).ok())
            .unwrap_or_default()
    }

    /// Check if a tool is allowed by name and category.
    pub fn is_allowed(&self, tool_name: &str, category: ToolCategory) -> bool {
        // Per-tool override takes priority
        if let Some(&allowed) = self.tools.get(tool_name) {
            return allowed;
        }
        // Fall back to category
        match category {
            ToolCategory::Read => self.read,
            ToolCategory::Write => self.write,
            ToolCategory::Navigate => self.navigate,
            ToolCategory::Execute => self.execute,
        }
    }
}

/// Classify a tool name into a permission category.
pub fn categorize_tool(name: &str) -> ToolCategory {
    match name {
        "get_tree" | "get_item" | "get_subtree" | "get_note" | "get_cursor" | "get_timestamps" | "get_badges" => {
            ToolCategory::Read
        }
        "set_cursor" | "focus_panel" | "zoom" => ToolCategory::Navigate,
        "exec" | "eval_tcl" => ToolCategory::Execute,
        _ => ToolCategory::Write, // All mutation tools + unknown default to write
    }
}
