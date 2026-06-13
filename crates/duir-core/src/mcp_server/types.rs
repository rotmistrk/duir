//! MCP server types — mutations, node info, and utility functions.

use serde_json::{Value, json};

use crate::model::{Completion, TodoItem};
use crate::tree_ops::TreePath;

/// Serializable snapshot of a single node, returned by tool handlers.
#[derive(Debug, Clone)]
pub struct NodeInfo {
    pub path: TreePath,
    pub title: String,
    pub note: String,
    pub completed: String,
    pub important: bool,
    pub children_count: usize,
}

impl NodeInfo {
    pub(crate) fn from_item(item: &TodoItem, path: &TreePath) -> Self {
        let completed = match item.completed {
            Completion::Open => "open",
            Completion::Done => "done",
            Completion::Partial => "partial",
        };
        Self {
            path: path.clone(),
            title: item.title.clone(),
            note: item.note.clone(),
            completed: completed.to_owned(),
            important: item.important,
            children_count: item.items.len(),
        }
    }

    pub(crate) fn to_json(&self) -> Value {
        let path_str = self.path.iter().map(ToString::to_string).collect::<Vec<_>>().join(",");
        json!({
            "path": path_str,
            "title": self.title,
            "note": self.note,
            "completed": self.completed,
            "important": self.important,
            "children_count": self.children_count,
        })
    }
}

/// Mutation request sent from the MCP server thread to the main thread.
pub enum McpMutation {
    AddChild {
        parent_path: TreePath,
        title: String,
        note: String,
    },
    AddSibling {
        path: TreePath,
        title: String,
        note: String,
    },
    MarkDone {
        path: TreePath,
    },
    MarkImportant {
        path: TreePath,
    },
    Reorder {
        path: TreePath,
        direction: ReorderDirection,
    },
    SetPriority {
        path: TreePath,
        value: u8,
    },
    SetEffort {
        path: TreePath,
        value: u8,
    },
    SetStatus {
        path: TreePath,
        status: String,
    },
    SetNote {
        path: TreePath,
        content: String,
    },
    Promote {
        path: TreePath,
    },
    Demote {
        path: TreePath,
    },
    Fold {
        path: TreePath,
    },
    Unfold {
        path: TreePath,
    },
    Remove {
        path: TreePath,
    },
}

/// Direction for the reorder tool.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReorderDirection {
    Up,
    Down,
}

/// Parse a comma-separated path string like `"0,1,2"` into a `TreePath`.
#[must_use]
pub fn parse_path(s: &str) -> Option<TreePath> {
    if s.is_empty() {
        return Some(Vec::new());
    }
    s.split(',').map(|seg| seg.trim().parse::<usize>().ok()).collect()
}

/// Collect subtree nodes recursively into JSON values.
pub fn collect_subtree(
    items: &[TodoItem],
    base_path: &TreePath,
    max_depth: usize,
    current_depth: usize,
    results: &mut Vec<Value>,
) {
    if current_depth >= max_depth {
        return;
    }
    for (i, item) in items.iter().enumerate() {
        let mut child_path = base_path.clone();
        child_path.push(i);
        results.push(NodeInfo::from_item(item, &child_path).to_json());
        collect_subtree(&item.items, &child_path, max_depth, current_depth + 1, results);
    }
}
