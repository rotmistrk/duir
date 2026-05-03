//! Integration tests for MCP server over Unix socket.

#![allow(clippy::unwrap_used, clippy::assigning_clones, clippy::indexing_slicing)]

use duir_core::mcp_server::McpServer;
use duir_core::model::{TodoFile, TodoItem};
use std::io::{BufRead, BufReader, Write};
use std::sync::{Arc, Mutex, mpsc};

#[test]
fn mcp_server_over_unix_socket_round_trip() {
    let mut file = TodoFile::new("Socket Test");
    file.items.push(TodoItem::new("Task A"));
    let snapshot = Arc::new(Mutex::new(file));
    let (tx, _rx) = mpsc::channel();
    let server = McpServer::new(Arc::clone(&snapshot), tx);

    let dir = tempfile::tempdir().unwrap();
    let sock_path = dir.path().join("test.sock");
    let listener = std::os::unix::net::UnixListener::bind(&sock_path).unwrap();

    // Server thread: accept one connection, run MCP protocol
    let server_thread = std::thread::spawn(move || {
        let (stream, _) = listener.accept().unwrap();
        let reader = BufReader::new(stream.try_clone().unwrap());
        let writer = stream;
        server.run(reader, writer).unwrap();
    });

    // Client: connect and send initialize + tools/call
    let client = std::os::unix::net::UnixStream::connect(&sock_path).unwrap();
    let mut writer = client.try_clone().unwrap();
    let mut reader = BufReader::new(client);

    // Send initialize
    writeln!(
        writer,
        r#"{{"jsonrpc":"2.0","id":1,"method":"initialize","params":{{}}}}"#
    )
    .unwrap();
    writer.flush().unwrap();

    let mut line = String::new();
    reader.read_line(&mut line).unwrap();
    let resp: serde_json::Value = serde_json::from_str(&line).unwrap();
    assert_eq!(resp["id"], 1);
    assert_eq!(resp["result"]["serverInfo"]["name"], "duir");

    // Send tools/call read_node
    line.clear();
    writeln!(
        writer,
        r#"{{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{{"name":"read_node","arguments":{{"path":"0"}}}}}}"#
    )
    .unwrap();
    writer.flush().unwrap();

    reader.read_line(&mut line).unwrap();
    let resp: serde_json::Value = serde_json::from_str(&line).unwrap();
    assert_eq!(resp["id"], 2);
    let text = resp["result"]["content"][0]["text"].as_str().unwrap();
    let node: serde_json::Value = serde_json::from_str(text).unwrap();
    assert_eq!(node["title"], "Task A");

    // Close client → server exits
    drop(writer);
    drop(reader);
    server_thread.join().unwrap();
}

#[test]
fn mcp_server_handles_multiple_connections_sequentially() {
    let file = TodoFile::new("Multi");
    let snapshot = Arc::new(Mutex::new(file));
    let (tx, _rx) = mpsc::channel();

    let dir = tempfile::tempdir().unwrap();
    let sock_path = dir.path().join("multi.sock");
    let listener = std::os::unix::net::UnixListener::bind(&sock_path).unwrap();

    let snap = Arc::clone(&snapshot);
    let server_thread = std::thread::spawn(move || {
        // Accept two connections
        for stream in listener.incoming().take(2) {
            let stream = stream.unwrap();
            let reader = BufReader::new(stream.try_clone().unwrap());
            let writer = stream;
            let server = McpServer::new(Arc::clone(&snap), tx.clone());
            let _ = server.run(reader, writer);
        }
    });

    for i in 0..2 {
        let client = std::os::unix::net::UnixStream::connect(&sock_path).unwrap();
        let mut w = client.try_clone().unwrap();
        let mut r = BufReader::new(client);
        writeln!(w, r#"{{"jsonrpc":"2.0","id":{i},"method":"initialize","params":{{}}}}"#).unwrap();
        w.flush().unwrap();
        let mut line = String::new();
        r.read_line(&mut line).unwrap();
        let resp: serde_json::Value = serde_json::from_str(&line).unwrap();
        assert_eq!(resp["result"]["serverInfo"]["name"], "duir");
    }

    server_thread.join().unwrap();
}

/// Full kiro-cli session over socket: initialize → notification → tools/list → tools/call.
/// This is the exact sequence that failed in production.
#[test]
fn mcp_full_kiro_session_over_socket() {
    let mut file = TodoFile::new("Kiro Session Test");
    file.items.push(TodoItem::new("Task Alpha"));
    file.items.push(TodoItem::new("Task Beta"));
    let snapshot = Arc::new(Mutex::new(file));
    let (tx, _rx) = mpsc::channel();

    let dir = tempfile::tempdir().unwrap();
    let sock_path = dir.path().join("kiro-session.sock");
    let listener = std::os::unix::net::UnixListener::bind(&sock_path).unwrap();

    let snap = Arc::clone(&snapshot);
    let server_thread = std::thread::spawn(move || {
        let (stream, _) = listener.accept().unwrap();
        let reader = BufReader::new(stream.try_clone().unwrap());
        let writer = stream;
        let server = McpServer::new(snap, tx);
        let _ = server.run(reader, writer);
    });

    let client = std::os::unix::net::UnixStream::connect(&sock_path).unwrap();
    client
        .set_read_timeout(Some(std::time::Duration::from_secs(5)))
        .unwrap();
    let mut writer = client.try_clone().unwrap();
    let mut reader = BufReader::new(client);

    // 1. initialize
    writeln!(
        writer,
        r#"{{"jsonrpc":"2.0","id":0,"method":"initialize","params":{{"protocolVersion":"2025-06-18","capabilities":{{}},"clientInfo":{{"name":"kiro","version":"1.0"}}}}}}"#
    ).unwrap();
    writer.flush().unwrap();

    let mut line = String::new();
    reader.read_line(&mut line).unwrap();
    let resp: serde_json::Value = serde_json::from_str(&line).unwrap();
    assert_eq!(resp["id"], 0);

    // 2. notifications/initialized (no response expected)
    writeln!(writer, r#"{{"jsonrpc":"2.0","method":"notifications/initialized"}}"#).unwrap();
    writer.flush().unwrap();

    // 3. tools/list
    line.clear();
    writeln!(
        writer,
        r#"{{"jsonrpc":"2.0","id":1,"method":"tools/list","params":{{"_meta":{{"progressToken":0}}}}}}"#
    )
    .unwrap();
    writer.flush().unwrap();

    reader.read_line(&mut line).unwrap();
    let resp: serde_json::Value = serde_json::from_str(&line).unwrap();
    assert_eq!(resp["id"], 1);
    assert!(resp["result"]["tools"].is_array());

    // 4. tools/call get_context — this is the call that failed in production
    line.clear();
    writeln!(
        writer,
        r#"{{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{{"name":"get_context","arguments":{{}}}}}}"#
    )
    .unwrap();
    writer.flush().unwrap();

    reader.read_line(&mut line).unwrap();
    assert!(!line.is_empty(), "Expected response for tools/call, got empty");
    let resp: serde_json::Value = serde_json::from_str(&line).unwrap();
    assert_eq!(resp["id"], 2);
    let text = resp["result"]["content"][0]["text"].as_str().unwrap();
    let ctx: serde_json::Value = serde_json::from_str(text).unwrap();
    assert_eq!(ctx["title"], "Kiro Session Test");
    assert_eq!(ctx["total_items"], 2);

    // 5. tools/call list_children — verify a second tool call works too
    line.clear();
    writeln!(
        writer,
        r#"{{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{{"name":"list_children","arguments":{{"path":""}}}}}}"#
    ).unwrap();
    writer.flush().unwrap();

    reader.read_line(&mut line).unwrap();
    let resp: serde_json::Value = serde_json::from_str(&line).unwrap();
    assert_eq!(resp["id"], 3);
    let text = resp["result"]["content"][0]["text"].as_str().unwrap();
    let children: Vec<serde_json::Value> = serde_json::from_str(text).unwrap();
    assert_eq!(children.len(), 2);
    assert_eq!(children[0]["title"], "Task Alpha");

    drop(writer);
    drop(reader);
    server_thread.join().unwrap();
}
