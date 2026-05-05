use serde_json::{Map, Value};

use super::McpServer;

pub(super) fn tool_definitions() -> Value {
    let mut tools = super::tools_read::definitions();
    if let Value::Array(ref mut arr) = tools
        && let Value::Array(write) = super::tools_write::definitions()
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
            "add_child" => self.tool_add_child(args),
            "add_sibling" => self.tool_add_sibling(args),
            "mark_done" => self.tool_mark_done(args),
            "mark_important" => self.tool_mark_important(args),
            "reorder" => self.tool_reorder(args),
            "get_context" => self.tool_get_context(),
            _ => Err(format!("Unknown tool: {name}")),
        }
    }
}
