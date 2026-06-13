//! MCP (Model Context Protocol) server for exposing a kiron subtree as tools.
//!
//! Implements JSON-RPC 2.0 over stdio. Designed to run in a dedicated thread,
//! communicating with the main application via channels.

mod tools;
mod tools_read;
mod tools_write;
mod tools_write_defs;
mod types;

#[cfg(test)]
mod tests;

use std::io::{self, BufRead, Write};
use std::sync::{Arc, Mutex};

use serde_json::{Map, Value, json};

use crate::model::TodoFile;

pub use types::{McpMutation, NodeInfo, ReorderDirection, collect_subtree, parse_path};

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

/// Tool permission filter type.
pub type ToolFilter = Arc<dyn Fn(&str) -> bool + Send + Sync>;

/// MCP server that holds a shared snapshot of the kiron subtree.
///
/// Read operations use the snapshot directly. Mutation operations
/// apply to the snapshot and also send an `McpMutation` via channel
/// so the main thread can persist changes.
pub struct McpServer {
    pub(crate) snapshot: Arc<Mutex<TodoFile>>,
    pub(crate) mutation_tx: std::sync::mpsc::Sender<McpMutation>,
    /// Optional tool permission filter. Returns true if the tool is allowed.
    pub tool_filter: Option<ToolFilter>,
}

impl McpServer {
    pub fn new(snapshot: Arc<Mutex<TodoFile>>, mutation_tx: std::sync::mpsc::Sender<McpMutation>) -> Self {
        Self {
            snapshot,
            mutation_tx,
            tool_filter: None,
        }
    }

    /// Set a tool permission filter.
    #[must_use]
    pub fn with_filter(mut self, filter: ToolFilter) -> Self {
        self.tool_filter = Some(filter);
        self
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

        // Permission check
        if let Some(ref filter) = self.tool_filter
            && !filter(tool_name)
        {
            let r = json!({
                "isError": true,
                "content": [{"type": "text", "text": format!("Permission denied: {tool_name}")}],
            });
            return jsonrpc_result(id, &r);
        }

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
