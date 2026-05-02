//! MCP (Model Context Protocol) server for exposing a kiron subtree as tools.
//!
//! Implements JSON-RPC 2.0 over stdio. Designed to run in a dedicated thread,
//! communicating with the main application via channels.

use std::io::{self, BufRead, Write};
use std::sync::{Arc, Mutex};

use serde_json::{Map, Value, json};

use crate::filter::{FilterOptions, filter_items};
use crate::model::{Completion, TodoFile, TodoItem};
use crate::tree_ops::{self, TreePath};

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
    fn from_item(item: &TodoItem, path: &TreePath) -> Self {
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

    fn to_json(&self) -> Value {
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
fn parse_path(s: &str) -> Option<TreePath> {
    if s.is_empty() {
        return Some(Vec::new());
    }
    s.split(',').map(|seg| seg.trim().parse::<usize>().ok()).collect()
}

#[allow(clippy::too_many_lines)]
fn tool_definitions() -> Value {
    json!([
        {
            "name": "read_node",
            "description": "Read a node's title, note, completion status, and importance",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "path": {"type": "string", "description": "Comma-separated indices, e.g. '0,1,2'"}
                },
                "required": ["path"]
            }
        },
        {
            "name": "list_children",
            "description": "List immediate children of a node",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "path": {"type": "string", "description": "Comma-separated indices"}
                },
                "required": ["path"]
            }
        },
        {
            "name": "list_subtree",
            "description": "List descendants of a node up to a maximum depth",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "path": {"type": "string", "description": "Comma-separated indices"},
                    "max_depth": {"type": "integer", "description": "Maximum depth (default 3)"}
                },
                "required": ["path"]
            }
        },
        {
            "name": "search",
            "description": "Search nodes by title or note content",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "query": {"type": "string", "description": "Search query"}
                },
                "required": ["query"]
            }
        },
        {
            "name": "add_child",
            "description": "Add a child node to the specified parent",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "parent_path": {"type": "string", "description": "Parent path"},
                    "title": {"type": "string", "description": "Title for the new node"},
                    "note": {"type": "string", "description": "Note (optional)", "default": ""}
                },
                "required": ["parent_path", "title"]
            }
        },
        {
            "name": "add_sibling",
            "description": "Add a sibling node after the specified node",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "path": {"type": "string", "description": "Comma-separated indices"},
                    "title": {"type": "string", "description": "Title for the new node"},
                    "note": {"type": "string", "description": "Note (optional)", "default": ""}
                },
                "required": ["path", "title"]
            }
        },
        {
            "name": "mark_done",
            "description": "Mark a node as completed",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "path": {"type": "string", "description": "Comma-separated indices"}
                },
                "required": ["path"]
            }
        },
        {
            "name": "mark_important",
            "description": "Toggle importance flag on a node",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "path": {"type": "string", "description": "Comma-separated indices"}
                },
                "required": ["path"]
            }
        },
        {
            "name": "reorder",
            "description": "Move a node up or down among its siblings",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "path": {"type": "string", "description": "Comma-separated indices"},
                    "direction": {"type": "string", "enum": ["up", "down"]}
                },
                "required": ["path", "direction"]
            }
        },
        {
            "name": "get_context",
            "description": "Get kiron root info and completion statistics",
            "inputSchema": {
                "type": "object",
                "properties": {}
            }
        }
    ])
}

/// MCP server that holds a shared snapshot of the kiron subtree.
///
/// Read operations use the snapshot directly. Mutation operations
/// apply to the snapshot and also send an `McpMutation` via channel
/// so the main thread can persist changes.
pub struct McpServer {
    snapshot: Arc<Mutex<TodoFile>>,
    mutation_tx: std::sync::mpsc::Sender<McpMutation>,
}

impl McpServer {
    pub const fn new(snapshot: Arc<Mutex<TodoFile>>, mutation_tx: std::sync::mpsc::Sender<McpMutation>) -> Self {
        Self { snapshot, mutation_tx }
    }

    fn handle_tool_call(&self, name: &str, args: &Map<String, Value>) -> Result<Value, String> {
        match name {
            "read_node" => self.tool_read_node(args),
            "list_children" => self.tool_list_children(args),
            "list_subtree" => self.tool_list_subtree(args),
            "search" => self.tool_search(args),
            "add_child" => self.tool_add_child(args),
            "add_sibling" => self.tool_add_sibling(args),
            "mark_done" => self.tool_mark_done(args),
            "mark_important" => self.tool_mark_important(args),
            "reorder" => self.tool_reorder(args),
            "get_context" => self.tool_get_context(),
            _ => Err(format!("Unknown tool: {name}")),
        }
    }

    fn require_path(args: &Map<String, Value>, key: &str) -> Result<TreePath, String> {
        let s = args
            .get(key)
            .and_then(Value::as_str)
            .ok_or_else(|| format!("Missing required string argument: {key}"))?;
        parse_path(s).ok_or_else(|| format!("Invalid path: {s}"))
    }

    fn tool_read_node(&self, args: &Map<String, Value>) -> Result<Value, String> {
        let path = Self::require_path(args, "path")?;
        let file = self.snapshot.lock().map_err(|e| e.to_string())?;
        let item = tree_ops::get_item(&file, &path).ok_or_else(|| format!("Node not found at path: {path:?}"))?;
        let result = NodeInfo::from_item(item, &path).to_json();
        drop(file);
        Ok(result)
    }

    fn tool_list_children(&self, args: &Map<String, Value>) -> Result<Value, String> {
        let path = Self::require_path(args, "path")?;
        let file = self.snapshot.lock().map_err(|e| e.to_string())?;
        let children = if path.is_empty() {
            &file.items
        } else {
            let item = tree_ops::get_item(&file, &path).ok_or_else(|| format!("Node not found at path: {path:?}"))?;
            &item.items
        };
        let infos: Vec<Value> = children
            .iter()
            .enumerate()
            .map(|(i, child)| {
                let mut child_path = path.clone();
                child_path.push(i);
                NodeInfo::from_item(child, &child_path).to_json()
            })
            .collect();
        drop(file);
        Ok(json!(infos))
    }

    fn tool_list_subtree(&self, args: &Map<String, Value>) -> Result<Value, String> {
        let path = Self::require_path(args, "path")?;
        let max_depth = args
            .get("max_depth")
            .and_then(Value::as_u64)
            .map_or(3, |v| usize::try_from(v).unwrap_or(usize::MAX));
        let file = self.snapshot.lock().map_err(|e| e.to_string())?;
        let items = if path.is_empty() {
            &file.items
        } else {
            let item = tree_ops::get_item(&file, &path).ok_or_else(|| format!("Node not found at path: {path:?}"))?;
            &item.items
        };
        let mut results = Vec::new();
        collect_subtree(items, &path, max_depth, 0, &mut results);
        drop(file);
        Ok(json!(results))
    }

    fn tool_search(&self, args: &Map<String, Value>) -> Result<Value, String> {
        let query = args
            .get("query")
            .and_then(Value::as_str)
            .ok_or("Missing required argument: query")?;
        let file = self.snapshot.lock().map_err(|e| e.to_string())?;
        let opts = FilterOptions {
            search_notes: true,
            case_sensitive: false,
        };
        let paths = filter_items(&file.items, query, &opts);
        let results: Vec<Value> = paths
            .iter()
            .filter_map(|p| tree_ops::get_item(&file, p).map(|item| NodeInfo::from_item(item, p).to_json()))
            .collect();
        drop(file);
        Ok(json!(results))
    }

    fn tool_add_child(&self, args: &Map<String, Value>) -> Result<Value, String> {
        let parent_path = Self::require_path(args, "parent_path")?;
        let title = args
            .get("title")
            .and_then(Value::as_str)
            .ok_or("Missing required argument: title")?
            .to_owned();
        let note = args.get("note").and_then(Value::as_str).unwrap_or("").to_owned();
        let mut item = TodoItem::new(&title);
        item.note.clone_from(&note);
        {
            let mut file = self.snapshot.lock().map_err(|e| e.to_string())?;
            tree_ops::add_child(&mut file, &parent_path, item).map_err(|e| e.to_string())?;
        }
        self.mutation_tx
            .send(McpMutation::AddChild {
                parent_path,
                title,
                note,
            })
            .map_err(|e| e.to_string())?;
        Ok(json!({"success": true}))
    }

    fn tool_add_sibling(&self, args: &Map<String, Value>) -> Result<Value, String> {
        let path = Self::require_path(args, "path")?;
        let title = args
            .get("title")
            .and_then(Value::as_str)
            .ok_or("Missing required argument: title")?
            .to_owned();
        let note = args.get("note").and_then(Value::as_str).unwrap_or("").to_owned();
        let mut item = TodoItem::new(&title);
        item.note.clone_from(&note);
        {
            let mut file = self.snapshot.lock().map_err(|e| e.to_string())?;
            tree_ops::add_sibling(&mut file, &path, item).map_err(|e| e.to_string())?;
        }
        self.mutation_tx
            .send(McpMutation::AddSibling { path, title, note })
            .map_err(|e| e.to_string())?;
        Ok(json!({"success": true}))
    }

    fn tool_mark_done(&self, args: &Map<String, Value>) -> Result<Value, String> {
        let path = Self::require_path(args, "path")?;
        self.snapshot.lock().map_err(|e| e.to_string()).and_then(|mut file| {
            let item =
                tree_ops::get_item_mut(&mut file, &path).ok_or_else(|| format!("Node not found at path: {path:?}"))?;
            item.completed = Completion::Done;
            Ok(())
        })?;
        self.mutation_tx
            .send(McpMutation::MarkDone { path })
            .map_err(|e| e.to_string())?;
        Ok(json!({"success": true}))
    }

    fn tool_mark_important(&self, args: &Map<String, Value>) -> Result<Value, String> {
        let path = Self::require_path(args, "path")?;
        self.snapshot.lock().map_err(|e| e.to_string()).and_then(|mut file| {
            let item =
                tree_ops::get_item_mut(&mut file, &path).ok_or_else(|| format!("Node not found at path: {path:?}"))?;
            item.important = !item.important;
            Ok(())
        })?;
        self.mutation_tx
            .send(McpMutation::MarkImportant { path })
            .map_err(|e| e.to_string())?;
        Ok(json!({"success": true}))
    }

    fn tool_reorder(&self, args: &Map<String, Value>) -> Result<Value, String> {
        let path = Self::require_path(args, "path")?;
        let dir_str = args
            .get("direction")
            .and_then(Value::as_str)
            .ok_or("Missing required argument: direction")?;
        let direction = match dir_str {
            "up" => ReorderDirection::Up,
            "down" => ReorderDirection::Down,
            other => return Err(format!("Invalid direction: {other}")),
        };
        {
            let mut file = self.snapshot.lock().map_err(|e| e.to_string())?;
            match direction {
                ReorderDirection::Up => {
                    tree_ops::swap_up(&mut file, &path).map_err(|e| e.to_string())?;
                }
                ReorderDirection::Down => {
                    tree_ops::swap_down(&mut file, &path).map_err(|e| e.to_string())?;
                }
            }
        }
        self.mutation_tx
            .send(McpMutation::Reorder { path, direction })
            .map_err(|e| e.to_string())?;
        Ok(json!({"success": true}))
    }

    fn tool_get_context(&self) -> Result<Value, String> {
        let file = self.snapshot.lock().map_err(|e| e.to_string())?;
        let stats = crate::stats::compute_file_stats(&file);
        let result = json!({
            "title": file.title,
            "note": file.note,
            "total_items": file.items.len(),
            "total_leaves": stats.total_leaves,
            "checked_leaves": stats.checked_leaves,
            "completion_percentage": stats.percentage,
        });
        drop(file);
        Ok(result)
    }
}

fn collect_subtree(
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

impl McpServer {
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
                let result = json!({"tools": tool_definitions()});
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

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::significant_drop_tightening)]
mod tests {
    use super::*;
    use std::sync::mpsc;

    fn sample_file() -> TodoFile {
        let mut file = TodoFile::new("Test Project");
        file.note = "Root note".to_owned();
        let mut a = TodoItem::new("Task A");
        a.note = "Note for A".to_owned();
        a.items.push(TodoItem::new("Subtask A1"));
        let mut a2 = TodoItem::new("Subtask A2");
        a2.completed = Completion::Done;
        a.items.push(a2);
        file.items.push(a);
        let mut b = TodoItem::new("Task B");
        b.important = true;
        file.items.push(b);
        file.items.push(TodoItem::new("Task C"));
        file
    }

    fn make_server() -> (McpServer, mpsc::Receiver<McpMutation>) {
        let (tx, rx) = mpsc::channel();
        let snapshot = Arc::new(Mutex::new(sample_file()));
        (McpServer::new(snapshot, tx), rx)
    }

    fn call_tool(server: &McpServer, name: &str, args: &Value) -> Value {
        let req = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "tools/call",
            "params": {"name": name, "arguments": args},
        });
        server.handle_request(&req).unwrap()
    }

    fn extract_text(response: &Value) -> String {
        response["result"]["content"][0]["text"].as_str().unwrap().to_owned()
    }

    #[test]
    fn initialize_returns_capabilities() {
        let (server, _rx) = make_server();
        let req = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {},
        });
        let resp = server.handle_request(&req).unwrap();
        assert_eq!(resp["result"]["protocolVersion"], "2024-11-05");
        assert!(resp["result"]["capabilities"]["tools"].is_object());
        assert_eq!(resp["result"]["serverInfo"]["name"], "duir");
    }

    #[test]
    fn tools_list_returns_all_tools() {
        let (server, _rx) = make_server();
        let req = json!({"jsonrpc": "2.0", "id": 2, "method": "tools/list"});
        let resp = server.handle_request(&req).unwrap();
        let tools = resp["result"]["tools"].as_array().unwrap();
        assert_eq!(tools.len(), 10);
    }

    #[test]
    fn read_node_returns_info() {
        let (server, _rx) = make_server();
        let resp = call_tool(&server, "read_node", &json!({"path": "0"}));
        let text = extract_text(&resp);
        let info: Value = serde_json::from_str(&text).unwrap();
        assert_eq!(info["title"], "Task A");
        assert_eq!(info["children_count"], 2);
    }

    #[test]
    fn read_node_nested() {
        let (server, _rx) = make_server();
        let resp = call_tool(&server, "read_node", &json!({"path": "0,1"}));
        let text = extract_text(&resp);
        let info: Value = serde_json::from_str(&text).unwrap();
        assert_eq!(info["title"], "Subtask A2");
        assert_eq!(info["completed"], "done");
    }

    #[test]
    fn read_node_invalid_path() {
        let (server, _rx) = make_server();
        let resp = call_tool(&server, "read_node", &json!({"path": "99"}));
        assert!(resp["result"]["isError"].as_bool().unwrap_or(false));
    }

    #[test]
    fn list_children_root() {
        let (server, _rx) = make_server();
        let resp = call_tool(&server, "list_children", &json!({"path": ""}));
        let text = extract_text(&resp);
        let children: Vec<Value> = serde_json::from_str(&text).unwrap();
        assert_eq!(children.len(), 3);
        assert_eq!(children[0]["title"], "Task A");
    }

    #[test]
    fn list_children_nested() {
        let (server, _rx) = make_server();
        let resp = call_tool(&server, "list_children", &json!({"path": "0"}));
        let text = extract_text(&resp);
        let children: Vec<Value> = serde_json::from_str(&text).unwrap();
        assert_eq!(children.len(), 2);
        assert_eq!(children[0]["title"], "Subtask A1");
    }

    #[test]
    fn add_child_creates_node_and_sends_mutation() {
        let (server, rx) = make_server();
        let resp = call_tool(
            &server,
            "add_child",
            &json!({"parent_path": "0", "title": "New Child", "note": "hello"}),
        );
        let text = extract_text(&resp);
        let result: Value = serde_json::from_str(&text).unwrap();
        assert_eq!(result["success"], true);

        {
            let file = server.snapshot.lock().unwrap();
            assert_eq!(file.items[0].items.len(), 3);
            assert_eq!(file.items[0].items[2].title, "New Child");
            drop(file);
        }

        let mutation = rx.try_recv().unwrap();
        assert!(matches!(mutation, McpMutation::AddChild { .. }));
    }

    #[test]
    fn search_finds_by_title() {
        let (server, _rx) = make_server();
        let resp = call_tool(&server, "search", &json!({"query": "subtask"}));
        let text = extract_text(&resp);
        let results: Vec<Value> = serde_json::from_str(&text).unwrap();
        assert!(results.len() >= 2);
        assert!(results.iter().any(|r| r["title"] == "Subtask A1"));
        assert!(results.iter().any(|r| r["title"] == "Subtask A2"));
    }

    #[test]
    fn search_finds_by_note() {
        let (server, _rx) = make_server();
        let resp = call_tool(&server, "search", &json!({"query": "note for"}));
        let text = extract_text(&resp);
        let results: Vec<Value> = serde_json::from_str(&text).unwrap();
        assert!(results.iter().any(|r| r["title"] == "Task A"));
    }

    #[test]
    fn mark_done_updates_snapshot() {
        let (server, rx) = make_server();
        call_tool(&server, "mark_done", &json!({"path": "0,0"}));

        {
            let file = server.snapshot.lock().unwrap();
            assert_eq!(file.items[0].items[0].completed, Completion::Done);
            drop(file);
        }

        let mutation = rx.try_recv().unwrap();
        assert!(matches!(mutation, McpMutation::MarkDone { .. }));
    }

    #[test]
    fn get_context_returns_stats() {
        let (server, _rx) = make_server();
        let resp = call_tool(&server, "get_context", &json!({}));
        let text = extract_text(&resp);
        let ctx: Value = serde_json::from_str(&text).unwrap();
        assert_eq!(ctx["title"], "Test Project");
        assert_eq!(ctx["total_items"], 3);
        assert!(ctx["total_leaves"].as_u64().unwrap() > 0);
    }

    #[test]
    fn run_processes_multiple_requests() {
        let (server, _rx) = make_server();
        let input = [
            r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}"#,
            r#"{"jsonrpc":"2.0","id":2,"method":"tools/list"}"#,
        ]
        .join("\n");
        let mut output = Vec::new();
        server.run(io::BufReader::new(input.as_bytes()), &mut output).unwrap();
        let output_str = String::from_utf8(output).unwrap();
        let lines: Vec<&str> = output_str.lines().filter(|l| !l.is_empty()).collect();
        assert_eq!(lines.len(), 2);
        let resp1: Value = serde_json::from_str(lines[0]).unwrap();
        assert_eq!(resp1["id"], 1);
        let resp2: Value = serde_json::from_str(lines[1]).unwrap();
        assert_eq!(resp2["id"], 2);
    }

    #[test]
    fn parse_path_empty() {
        assert_eq!(parse_path(""), Some(vec![]));
    }

    #[test]
    fn parse_path_single() {
        assert_eq!(parse_path("0"), Some(vec![0]));
    }

    #[test]
    fn parse_path_multi() {
        assert_eq!(parse_path("1,2,3"), Some(vec![1, 2, 3]));
    }

    #[test]
    fn parse_path_invalid() {
        assert_eq!(parse_path("abc"), None);
    }
}
