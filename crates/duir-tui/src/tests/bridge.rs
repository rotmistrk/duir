// Bridge tests: verify fail-fast behavior and socket round-trip.
//
// All tests return Result and propagate errors with `?`.
// No unwrap, expect, assert, or panic.

use std::io::BufReader;
use std::os::unix::net::UnixStream;

type R = Result<(), Box<dyn std::error::Error>>;

fn check(condition: bool, msg: &str) -> R {
    if condition { Ok(()) } else { Err(msg.into()) }
}

fn check_eq<T: PartialEq + std::fmt::Debug>(a: &T, b: &T, msg: &str) -> R {
    if a == b {
        Ok(())
    } else {
        Err(format!("{msg}: {a:?} != {b:?}").into())
    }
}

#[test]
fn bridge_connect_nonexistent_socket_fails_immediately() -> R {
    let start = std::time::Instant::now();
    let result = UnixStream::connect("/tmp/duir-nonexistent-socket-path.sock");
    let elapsed = start.elapsed();

    check(result.is_err(), "connect should fail")?;
    check(elapsed.as_millis() < 100, "connect took too long")
}

#[test]
fn bridge_connect_to_real_listener_succeeds() -> R {
    let dir = tempfile::tempdir()?;
    let sock_path = dir.path().join("bridge-test.sock");
    let _listener = std::os::unix::net::UnixListener::bind(&sock_path)?;

    UnixStream::connect(&sock_path)?;
    Ok(())
}

#[test]
fn bridge_round_trip_through_socket() -> R {
    use std::io::{BufRead, Write};
    use std::sync::{Arc, Mutex, mpsc};

    let mut file = duir_core::TodoFile::new("Bridge Test");
    file.items.push(duir_core::TodoItem::new("Item 1"));
    let snapshot = Arc::new(Mutex::new(file));
    let (tx, _rx) = mpsc::channel();

    let dir = tempfile::tempdir()?;
    let sock_path = dir.path().join("bridge-rt.sock");
    let listener = std::os::unix::net::UnixListener::bind(&sock_path)?;

    let snap = Arc::clone(&snapshot);
    let server_thread = std::thread::spawn(move || {
        let stream = listener.accept().map(|(s, _)| s)?;
        let reader = BufReader::new(stream.try_clone()?);
        let server = duir_core::mcp_server::McpServer::new(snap, tx);
        server.run(reader, stream)
    });

    let socket = UnixStream::connect(&sock_path)?;
    let mut writer = socket.try_clone()?;
    let mut reader = BufReader::new(socket);

    writeln!(
        writer,
        r#"{{"jsonrpc":"2.0","id":1,"method":"initialize","params":{{}}}}"#
    )?;
    writer.flush()?;

    let mut line = String::new();
    reader.read_line(&mut line)?;
    let resp: serde_json::Value = serde_json::from_str(&line)?;

    check_eq(
        &resp["result"]["serverInfo"]["name"],
        &serde_json::json!("duir"),
        "server name",
    )?;

    drop(writer);
    drop(reader);
    let _ = server_thread.join();

    Ok(())
}

#[test]
fn bridge_peer_disconnect_unblocks() -> R {
    let dir = tempfile::tempdir()?;
    let sock_path = dir.path().join("disconnect.sock");
    let listener = std::os::unix::net::UnixListener::bind(&sock_path)?;

    let server_thread = std::thread::spawn(move || {
        let _ = listener.accept();
        // Drop immediately — peer sees EOF.
    });

    let socket = UnixStream::connect(&sock_path)?;
    socket.set_read_timeout(Some(std::time::Duration::from_secs(2)))?;
    let mut reader = BufReader::new(socket.try_clone()?);

    let start = std::time::Instant::now();
    let mut buf = String::new();
    let _ = std::io::BufRead::read_line(&mut reader, &mut buf);
    let elapsed = start.elapsed();

    check(elapsed.as_secs() < 2, "read should not hang")?;
    let _ = server_thread.join();

    Ok(())
}

/// Full session through the actual bridge binary: spawn duir --mcp-connect,
/// send initialize → tools/list → tools/call, verify responses.
#[test]
fn bridge_full_session_via_subprocess() -> R {
    use std::io::Write;
    use std::process::{Command, Stdio};
    use std::sync::{Arc, Mutex, mpsc};

    let mut file = duir_core::TodoFile::new("Bridge Session");
    file.items.push(duir_core::TodoItem::new("Alpha"));
    file.items.push(duir_core::TodoItem::new("Beta"));
    let snapshot = Arc::new(Mutex::new(file));
    let (tx, _rx) = mpsc::channel();

    let dir = tempfile::tempdir()?;
    let sock_path = dir.path().join("bridge-session.sock");
    let listener = std::os::unix::net::UnixListener::bind(&sock_path)?;

    let snap = Arc::clone(&snapshot);
    let server_thread = std::thread::spawn(move || {
        let stream = listener.accept().map(|(s, _)| s)?;
        let reader = BufReader::new(stream.try_clone()?);
        let server = duir_core::mcp_server::McpServer::new(snap, tx);
        server.run(reader, stream)
    });

    let bin = std::env::current_exe()?
        .parent()
        .and_then(|p| p.parent())
        .map(|p| p.join("duir-tui"))
        .ok_or("cannot resolve binary path")?;

    if !bin.exists() {
        return Ok(()); // skip if not built
    }

    let sock_str = sock_path.to_str().ok_or("non-utf8 path")?;

    let mut child = Command::new(&bin)
        .arg("--mcp-connect")
        .env("DUIR_MCP_SOCKET", sock_str)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()?;

    let mut stdin = child.stdin.take().ok_or("no stdin")?;
    let stdout = child.stdout.take().ok_or("no stdout")?;

    let (tx_line, rx_line) = std::sync::mpsc::channel();

    let stdout_thread = std::thread::spawn(move || {
        let mut reader = std::io::BufReader::new(stdout);

        loop {
            let mut line = String::new();

            match std::io::BufRead::read_line(&mut reader, &mut line) {
                Ok(0) | Err(_) => break,
                Ok(_) => {
                    if tx_line.send(line).is_err() {
                        break;
                    }
                }
            }
        }
    });

    let read_response = |rx: &std::sync::mpsc::Receiver<String>| -> Result<String, String> {
        rx.recv_timeout(std::time::Duration::from_secs(5))
            .map_err(|_| "Timed out waiting for MCP response".to_owned())
    };

    // 1. initialize
    writeln!(
        stdin,
        r#"{{"jsonrpc":"2.0","id":0,"method":"initialize","params":{{}}}}"#
    )?;
    stdin.flush()?;

    let line = read_response(&rx_line)?;
    let resp: serde_json::Value = serde_json::from_str(&line)?;
    check_eq(&resp["id"], &serde_json::json!(0), "initialize id")?;

    // 2. notification (no response)
    writeln!(stdin, r#"{{"jsonrpc":"2.0","method":"notifications/initialized"}}"#)?;
    stdin.flush()?;

    // 3. tools/list
    writeln!(
        stdin,
        r#"{{"jsonrpc":"2.0","id":1,"method":"tools/list","params":{{}}}}"#
    )?;
    stdin.flush()?;

    let line = read_response(&rx_line)?;
    let resp: serde_json::Value = serde_json::from_str(&line)?;
    check_eq(&resp["id"], &serde_json::json!(1), "tools/list id")?;

    // 4. tools/call get_context — the call that failed in production
    writeln!(
        stdin,
        r#"{{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{{"name":"get_context","arguments":{{}}}}}}"#
    )?;
    stdin.flush()?;

    let line = read_response(&rx_line)?;
    check(!line.is_empty(), "tools/call response empty")?;

    let resp: serde_json::Value = serde_json::from_str(&line)?;
    check_eq(&resp["id"], &serde_json::json!(2), "tools/call id")?;

    let text = resp["result"]["content"][0]["text"]
        .as_str()
        .ok_or("missing text in response")?;

    let ctx: serde_json::Value = serde_json::from_str(text)?;
    check_eq(&ctx["title"], &serde_json::json!("Bridge Session"), "title")?;

    // Cleanup
    drop(stdin);
    let _ = child.wait();
    let _ = server_thread.join();
    let _ = stdout_thread.join();

    Ok(())
}
