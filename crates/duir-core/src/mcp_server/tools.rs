use serde_json::{Map, Value};

use super::McpServer;

pub(super) fn tool_definitions() -> Value {
    let mut tools = super::tools_read::definitions();
    if let Value::Array(ref mut arr) = tools
        && let Value::Array(write) = super::tools_write_defs::definitions()
    {
        arr.extend(write);
    }
    tools
}

impl McpServer {
    pub(crate) fn handle_tool_call(&self, name: &str, args: &Map<String, Value>) -> Result<Value, String> {
        match name {
            "read_node" => self.tool_read_node(args),
            "list_children" => self.tool_list_children(args),
            "list_subtree" => self.tool_list_subtree(args),
            "search" => self.tool_search(args),
            "get_context" => self.tool_get_context(),
            "get_note" => self.tool_get_note(args),
            "add_child" => self.tool_add_child(args),
            "add_sibling" => self.tool_add_sibling(args),
            "mark_done" => self.tool_mark_done(args),
            "mark_important" => self.tool_mark_important(args),
            "reorder" => self.tool_reorder(args),
            "set_priority" => self.tool_set_priority(args),
            "set_effort" => self.tool_set_effort(args),
            "set_status" => self.tool_set_status(args),
            "set_note" => self.tool_set_note(args),
            "promote" => self.tool_promote(args),
            "demote" => self.tool_demote(args),
            "fold" => self.tool_fold(args),
            "unfold" => self.tool_unfold(args),
            "remove_item" => self.tool_remove_item(args),
            _ => Err(format!("Unknown tool: {name}")),
        }
    }
}
