use serde_json::{Map, Value, json};

use crate::filter::{FilterOptions, filter_items};
use crate::model::{Completion, TodoItem};
use crate::tree_ops::{self, TreePath};

use super::{McpMutation, McpServer, NodeInfo, ReorderDirection, collect_subtree, parse_path};

pub(super) fn tool_definitions() -> Value {
    let mut tools = read_tools();
    if let Value::Array(ref mut arr) = tools
        && let Value::Array(write) = write_tools()
    {
        arr.extend(write);
    }
    tools
}

fn read_tools() -> Value {
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
            "name": "get_context",
            "description": "Get kiron root info and completion statistics",
            "inputSchema": {
                "type": "object",
                "properties": {}
            }
        }
    ])
}

fn write_tools() -> Value {
    json!([
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
            "inputSchema": { "type": "object", "properties": {
                "path": {"type": "string", "description": "Comma-separated indices"}
            }, "required": ["path"] }
        },
        {
            "name": "mark_important",
            "description": "Toggle importance flag on a node",
            "inputSchema": { "type": "object", "properties": {
                "path": {"type": "string", "description": "Comma-separated indices"}
            }, "required": ["path"] }
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
        }
    ])
}

impl McpServer {
    pub(crate) fn handle_tool_call(&self, name: &str, args: &Map<String, Value>) -> Result<Value, String> {
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
