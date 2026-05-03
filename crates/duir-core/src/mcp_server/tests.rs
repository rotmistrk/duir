#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::significant_drop_tightening,
    clippy::indexing_slicing
)]

use std::io;
use std::sync::{Arc, Mutex, mpsc};

use serde_json::{Value, json};

use crate::model::{Completion, TodoFile, TodoItem};

use super::{McpMutation, McpServer, parse_path};

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

/// Full MCP session: initialize → notification → tools/list → tools/call.
/// This is the exact sequence kiro-cli sends.
#[test]
fn run_full_session_with_tool_call() {
    let (server, _rx) = make_server();

    let input = [
        r#"{"jsonrpc":"2.0","id":0,"method":"initialize","params":{"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"kiro","version":"1.0"}}}"#,
        r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#,
        r#"{"jsonrpc":"2.0","id":1,"method":"tools/list","params":{}}"#,
        r#"{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"get_context","arguments":{}}}"#,
    ]
    .join("\n");

    let mut output = Vec::new();
    server.run(io::BufReader::new(input.as_bytes()), &mut output).unwrap();

    let output_str = String::from_utf8(output).unwrap();
    let lines: Vec<&str> = output_str.lines().filter(|l| !l.is_empty()).collect();

    // initialize, tools/list, tools/call → 3 responses (notification has no response)
    assert_eq!(lines.len(), 3, "Expected 3 responses, got: {lines:?}");

    let resp_init: Value = serde_json::from_str(lines[0]).unwrap();
    assert_eq!(resp_init["id"], 0);
    assert!(resp_init["result"]["capabilities"].is_object());

    let resp_list: Value = serde_json::from_str(lines[1]).unwrap();
    assert_eq!(resp_list["id"], 1);
    assert!(resp_list["result"]["tools"].is_array());

    let resp_call: Value = serde_json::from_str(lines[2]).unwrap();
    assert_eq!(resp_call["id"], 2);
    let text = resp_call["result"]["content"][0]["text"].as_str().unwrap();
    let ctx: Value = serde_json::from_str(text).unwrap();
    assert_eq!(ctx["title"], "Test Project");
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
