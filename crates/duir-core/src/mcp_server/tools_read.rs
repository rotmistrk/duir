use serde_json::{Map, Value, json};

use crate::filter::{FilterOptions, filter_items};
use crate::tree_ops::{self, TreePath};

use super::{McpServer, NodeInfo, collect_subtree, parse_path};

pub(super) fn definitions() -> Value {
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

impl McpServer {
    pub(super) fn require_path(args: &Map<String, Value>, key: &str) -> Result<TreePath, String> {
        let s = args
            .get(key)
            .and_then(Value::as_str)
            .ok_or_else(|| format!("Missing required string argument: {key}"))?;
        parse_path(s).ok_or_else(|| format!("Invalid path: {s}"))
    }

    pub(super) fn tool_read_node(&self, args: &Map<String, Value>) -> Result<Value, String> {
        let path = Self::require_path(args, "path")?;
        let file = self.snapshot.lock().map_err(|e| e.to_string())?;
        let item = tree_ops::get_item(&file, &path).ok_or_else(|| format!("Node not found at path: {path:?}"))?;
        let result = NodeInfo::from_item(item, &path).to_json();
        drop(file);
        Ok(result)
    }

    pub(super) fn tool_list_children(&self, args: &Map<String, Value>) -> Result<Value, String> {
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

    pub(super) fn tool_list_subtree(&self, args: &Map<String, Value>) -> Result<Value, String> {
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

    pub(super) fn tool_search(&self, args: &Map<String, Value>) -> Result<Value, String> {
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

    pub(super) fn tool_get_context(&self) -> Result<Value, String> {
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
