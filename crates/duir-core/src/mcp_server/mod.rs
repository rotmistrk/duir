//! MCP (Model Context Protocol) server for exposing a kiron subtree as tools.
//!
//! Implements JSON-RPC 2.0 over stdio. Designed to run in a dedicated thread,
//! communicating with the main application via channels.

mod tools;
mod tools_read;
mod tools_write;

#[cfg(test)]
mod tests;

use std::io::{self, BufRead, Write};
use std::sync::{Arc, Mutex};

use serde_json::{Map, Value, json};

use crate::model::{Completion, TodoFile, TodoItem};
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
}

/// Direction for the reorder tool.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReorderDirection {
    Up,
    Down,
}

/// Parse a comma-separated path string like `"0,1,2"` into a `TreePath`.
pub(crate) fn parse_path(s: &str) -> Option<TreePath> {
    if s.is_empty() {
        return Some(Vec::new());
    }
    s.split(',').map(|seg| seg.trim().parse::<usize>().ok()).collect()
}

pub(crate) fn collect_subtree(
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

// ---------------------------------------------------------------------------
// JSON-RPC protocol handling
// ---------------------------------------------------------------------------

fn jsonrpc_error(id: Option<&Value>, code: i64, message: &str) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": id.cloned().unwrap_or(Value::Null),
        "error": {"code": code, "message": message},
    })
}

fn jsonrpc_result(id: &Value, result: &Value) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "result": result,
    })
}

/// MCP server that holds a shared snapshot of the kiron subtree.
///
/// Read operations use the snapshot directly. Mutation operations
/// apply to the snapshot and also send an `McpMutation` via channel
/// so the main thread can persist changes.
pub struct McpServer {
    pub(crate) snapshot: Arc<Mutex<TodoFile>>,
    pub(crate) mutation_tx: std::sync::mpsc::Sender<McpMutation>,
}

impl McpServer {
    pub const fn new(snapshot: Arc<Mutex<TodoFile>>, mutation_tx: std::sync::mpsc::Sender<McpMutation>) -> Self {
        Self { snapshot, mutation_tx }
    }

    /// Process a single JSON-RPC request and return the response.
    #[must_use]
    pub fn handle_request(&self, request: &Value) -> Option<Value> {
        let id = request.get("id");
        let method = request.get("method").and_then(Value::as_str);

        let Some(method) = method else {
            return Some(jsonrpc_error(id, -32600, "Missing method"));
        };

        // Notifications (no id) get no response per JSON-RPC spec.
        if method == "notifications/initialized" {
            return None;
        }

        let id = id?;

        match method {
            "initialize" => {
                let result = json!({
                    "protocolVersion": "2024-11-05",
                    "capabilities": {"tools": {}},
                    "serverInfo": {"name": "duir", "version": "0.1.0"},
                });
                Some(jsonrpc_result(id, &result))
            }
            "tools/list" => {
                let result = json!({"tools": tools::tool_definitions()});
                Some(jsonrpc_result(id, &result))
            }
            "tools/call" => Some(self.handle_tools_call(id, request)),
            _ => Some(jsonrpc_error(Some(id), -32601, &format!("Method not found: {method}"))),
        }
    }

    fn handle_tools_call(&self, id: &Value, request: &Value) -> Value {
        let params = request.get("params").cloned().unwrap_or_else(|| json!({}));
        let tool_name = params.get("name").and_then(Value::as_str).unwrap_or("");
        let empty_map = Map::new();
        let arguments = params.get("arguments").and_then(Value::as_object).unwrap_or(&empty_map);

        match self.handle_tool_call(tool_name, arguments) {
            Ok(result) => {
                let text = if result.is_string() {
                    result.as_str().unwrap_or("").to_owned()
                } else {
                    serde_json::to_string_pretty(&result).unwrap_or_default()
                };
                let r = json!({"content": [{"type": "text", "text": text}]});
                jsonrpc_result(id, &r)
            }
            Err(msg) => {
                let r = json!({
                    "isError": true,
                    "content": [{"type": "text", "text": msg}],
                });
                jsonrpc_result(id, &r)
            }
        }
    }

    /// Run the MCP server, reading JSON-RPC from `reader` and writing to `writer`.
    ///
    /// # Errors
    ///
    /// Returns `io::Error` on read/write failures.
    pub fn run<R: BufRead, W: Write>(&self, reader: R, mut writer: W) -> io::Result<()> {
        for line_result in reader.lines() {
            let line = line_result?;
            if line.trim().is_empty() {
                continue;
            }
            let Ok(request) = serde_json::from_str::<Value>(&line) else {
                let err = jsonrpc_error(None, -32700, "Parse error");
                writeln!(writer, "{err}")?;
                writer.flush()?;
                continue;
            };
            if let Some(response) = self.handle_request(&request) {
                writeln!(writer, "{response}")?;
                writer.flush()?;
            }
        }
        Ok(())
    }

    /// Run the MCP server on real stdin/stdout.
    ///
    /// # Errors
    ///
    /// Returns `io::Error` on read/write failures.
    pub fn run_stdio(&self) -> io::Result<()> {
        let stdin = io::stdin().lock();
        let stdout = io::stdout().lock();
        self.run(stdin, stdout)
    }
}
