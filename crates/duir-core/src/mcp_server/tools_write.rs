use serde_json::{Map, Value, json};

use crate::model::{Completion, TodoItem};
use crate::tree_ops;

use super::{McpMutation, McpServer, ReorderDirection};

pub(super) fn definitions() -> Value {
    let mut base = definitions_base();
    if let Value::Array(ref mut arr) = base
        && let Value::Array(extra) = definitions_extra()
    {
        arr.extend(extra);
    }
    base
}

fn definitions_base() -> Value {
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
        },
        {
            "name": "set_priority",
            "description": "Set priority (0-9) on a node",
            "inputSchema": { "type": "object", "properties": {
                "path": {"type": "string", "description": "Comma-separated indices"},
                "value": {"type": "integer", "description": "Priority 0-9"}
            }, "required": ["path", "value"] }
        },
        {
            "name": "set_effort",
            "description": "Set effort estimate (fibonacci: 0,1,2,3,5,8,13,21) on a node",
            "inputSchema": { "type": "object", "properties": {
                "path": {"type": "string", "description": "Comma-separated indices"},
                "value": {"type": "integer", "description": "Fibonacci effort value"}
            }, "required": ["path", "value"] }
        }
    ])
}

fn definitions_extra() -> Value {
    json!([
        {
            "name": "set_status",
            "description": "Set work status: idle, in_progress, paused",
            "inputSchema": { "type": "object", "properties": {
                "path": {"type": "string", "description": "Comma-separated indices"},
                "status": {"type": "string", "enum": ["idle", "in_progress", "paused"]}
            }, "required": ["path", "status"] }
        },
        {
            "name": "set_note",
            "description": "Set the note content of a node",
            "inputSchema": { "type": "object", "properties": {
                "path": {"type": "string", "description": "Comma-separated indices"},
                "content": {"type": "string", "description": "Note content"}
            }, "required": ["path", "content"] }
        },
        {
            "name": "promote",
            "description": "Promote a node (move left in hierarchy)",
            "inputSchema": { "type": "object", "properties": {
                "path": {"type": "string", "description": "Comma-separated indices"}
            }, "required": ["path"] }
        },
        {
            "name": "demote",
            "description": "Demote a node (move right in hierarchy)",
            "inputSchema": { "type": "object", "properties": {
                "path": {"type": "string", "description": "Comma-separated indices"}
            }, "required": ["path"] }
        },
        {
            "name": "fold",
            "description": "Fold (collapse) a node",
            "inputSchema": { "type": "object", "properties": {
                "path": {"type": "string", "description": "Comma-separated indices"}
            }, "required": ["path"] }
        },
        {
            "name": "unfold",
            "description": "Unfold (expand) a node",
            "inputSchema": { "type": "object", "properties": {
                "path": {"type": "string", "description": "Comma-separated indices"}
            }, "required": ["path"] }
        },
        {
            "name": "remove_item",
            "description": "Remove a node and its children",
            "inputSchema": { "type": "object", "properties": {
                "path": {"type": "string", "description": "Comma-separated indices"}
            }, "required": ["path"] }
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

impl McpServer {
    #[allow(clippy::significant_drop_tightening)]
    pub(super) fn tool_set_priority(&self, args: &Map<String, Value>) -> Result<Value, String> {
        let path = Self::require_path(args, "path")?;
        let value = u8::try_from(args.get("value").and_then(Value::as_u64).ok_or("Missing value")?)
            .map_err(|e| format!("invalid value: {e}"))?;
        {
            let mut file = self.snapshot.lock().map_err(|e| e.to_string())?;
            let item = tree_ops::get_item_mut(&mut file, &path).ok_or("Node not found")?;
            item.priority = if value == 0 { None } else { Some(value) };
        }
        self.mutation_tx
            .send(McpMutation::SetPriority { path, value })
            .map_err(|e| e.to_string())?;
        Ok(json!({"success": true}))
    }

    #[allow(clippy::significant_drop_tightening)]
    pub(super) fn tool_set_effort(&self, args: &Map<String, Value>) -> Result<Value, String> {
        let path = Self::require_path(args, "path")?;
        let value = u8::try_from(args.get("value").and_then(Value::as_u64).ok_or("Missing value")?)
            .map_err(|e| format!("invalid value: {e}"))?;
        {
            let mut file = self.snapshot.lock().map_err(|e| e.to_string())?;
            let item = tree_ops::get_item_mut(&mut file, &path).ok_or("Node not found")?;
            item.effort = if value == 0 { None } else { Some(value) };
        }
        self.mutation_tx
            .send(McpMutation::SetEffort { path, value })
            .map_err(|e| e.to_string())?;
        Ok(json!({"success": true}))
    }

    #[allow(clippy::significant_drop_tightening)]
    pub(super) fn tool_set_status(&self, args: &Map<String, Value>) -> Result<Value, String> {
        let path = Self::require_path(args, "path")?;
        let status = args
            .get("status")
            .and_then(Value::as_str)
            .ok_or("Missing status")?
            .to_owned();
        let ws = match status.as_str() {
            "idle" => crate::model::WorkStatus::Idle,
            "in_progress" => crate::model::WorkStatus::InProgress,
            "paused" => crate::model::WorkStatus::Paused,
            other => return Err(format!("Invalid status: {other}")),
        };
        {
            let mut file = self.snapshot.lock().map_err(|e| e.to_string())?;
            let item = tree_ops::get_item_mut(&mut file, &path).ok_or("Node not found")?;
            item.work_status = ws;
        }
        self.mutation_tx
            .send(McpMutation::SetStatus { path, status })
            .map_err(|e| e.to_string())?;
        Ok(json!({"success": true}))
    }

    #[allow(clippy::significant_drop_tightening)]
    pub(super) fn tool_set_note(&self, args: &Map<String, Value>) -> Result<Value, String> {
        let path = Self::require_path(args, "path")?;
        let content = args
            .get("content")
            .and_then(Value::as_str)
            .ok_or("Missing content")?
            .to_owned();
        {
            let mut file = self.snapshot.lock().map_err(|e| e.to_string())?;
            let item = tree_ops::get_item_mut(&mut file, &path).ok_or("Node not found")?;
            item.note.clone_from(&content);
        }
        self.mutation_tx
            .send(McpMutation::SetNote { path, content })
            .map_err(|e| e.to_string())?;
        Ok(json!({"success": true}))
    }

    pub(super) fn tool_promote(&self, args: &Map<String, Value>) -> Result<Value, String> {
        let path = Self::require_path(args, "path")?;
        {
            let mut file = self.snapshot.lock().map_err(|e| e.to_string())?;
            tree_ops::promote(&mut file, &path).map_err(|e| e.to_string())?;
        }
        self.mutation_tx
            .send(McpMutation::Promote { path })
            .map_err(|e| e.to_string())?;
        Ok(json!({"success": true}))
    }

    pub(super) fn tool_demote(&self, args: &Map<String, Value>) -> Result<Value, String> {
        let path = Self::require_path(args, "path")?;
        {
            let mut file = self.snapshot.lock().map_err(|e| e.to_string())?;
            tree_ops::demote(&mut file, &path).map_err(|e| e.to_string())?;
        }
        self.mutation_tx
            .send(McpMutation::Demote { path })
            .map_err(|e| e.to_string())?;
        Ok(json!({"success": true}))
    }

    #[allow(clippy::significant_drop_tightening)]
    pub(super) fn tool_fold(&self, args: &Map<String, Value>) -> Result<Value, String> {
        let path = Self::require_path(args, "path")?;
        {
            let mut file = self.snapshot.lock().map_err(|e| e.to_string())?;
            let item = tree_ops::get_item_mut(&mut file, &path).ok_or("Node not found")?;
            item.folded = true;
        }
        self.mutation_tx
            .send(McpMutation::Fold { path })
            .map_err(|e| e.to_string())?;
        Ok(json!({"success": true}))
    }

    #[allow(clippy::significant_drop_tightening)]
    pub(super) fn tool_unfold(&self, args: &Map<String, Value>) -> Result<Value, String> {
        let path = Self::require_path(args, "path")?;
        {
            let mut file = self.snapshot.lock().map_err(|e| e.to_string())?;
            let item = tree_ops::get_item_mut(&mut file, &path).ok_or("Node not found")?;
            item.folded = false;
        }
        self.mutation_tx
            .send(McpMutation::Unfold { path })
            .map_err(|e| e.to_string())?;
        Ok(json!({"success": true}))
    }

    pub(super) fn tool_remove_item(&self, args: &Map<String, Value>) -> Result<Value, String> {
        let path = Self::require_path(args, "path")?;
        {
            let mut file = self.snapshot.lock().map_err(|e| e.to_string())?;
            tree_ops::remove_item(&mut file, &path).map_err(|e| e.to_string())?;
        }
        self.mutation_tx
            .send(McpMutation::Remove { path })
            .map_err(|e| e.to_string())?;
        Ok(json!({"success": true}))
    }
}
