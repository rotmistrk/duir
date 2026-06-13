//! MCP write tool implementations.

use serde_json::{Map, Value, json};

use crate::model::{Completion, TodoItem};
use crate::tree_ops;

use super::{McpMutation, McpServer, ReorderDirection};

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
