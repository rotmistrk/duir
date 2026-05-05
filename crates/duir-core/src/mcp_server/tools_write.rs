use serde_json::{Map, Value, json};

use crate::model::{Completion, TodoItem};
use crate::tree_ops;

use super::{McpMutation, McpServer, ReorderDirection};

pub(super) fn definitions() -> Value {
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
    pub(super) fn tool_add_child(&self, args: &Map<String, Value>) -> Result<Value, String> {
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

    pub(super) fn tool_add_sibling(&self, args: &Map<String, Value>) -> Result<Value, String> {
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

    pub(super) fn tool_mark_done(&self, args: &Map<String, Value>) -> Result<Value, String> {
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

    pub(super) fn tool_mark_important(&self, args: &Map<String, Value>) -> Result<Value, String> {
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

    pub(super) fn tool_reorder(&self, args: &Map<String, Value>) -> Result<Value, String> {
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
}
