//! MCP write tool definitions (JSON schemas).

use serde_json::{Value, json};

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
