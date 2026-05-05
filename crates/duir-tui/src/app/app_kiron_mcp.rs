//! MCP socket server and agent file management for kiron sessions.

use crate::mcp_log::{LoggingReader, LoggingWriter};
use std::io::BufReader;
use std::sync::{Arc, Mutex};

/// Compute the Unix socket path for a kiron session.
pub fn mcp_socket_path(session_id: &str) -> std::path::PathBuf {
    let dir = std::env::var("XDG_RUNTIME_DIR").unwrap_or_else(|_| {
        let uid = std::env::var("UID")
            .or_else(|_| std::env::var("EUID"))
            .unwrap_or_else(|_| "0".to_owned());
        format!("/tmp/duir-{uid}")
    });

    let dir = std::path::PathBuf::from(dir);
    let _ = std::fs::create_dir_all(&dir);
    dir.join(format!("duir-mcp-{session_id}.sock"))
}

/// Start an MCP server thread listening on a Unix socket.
/// Returns the socket path on success.
pub fn start_mcp_listener(
    snapshot: Arc<Mutex<duir_core::TodoFile>>,
    mutation_tx: std::sync::mpsc::Sender<duir_core::mcp_server::McpMutation>,
    session_id: &str,
) -> Result<std::path::PathBuf, String> {
    let path = mcp_socket_path(session_id);
    crate::mcp_log::log("listener", &format!("binding {}", path.display()));

    if path.exists() {
        if std::os::unix::net::UnixStream::connect(&path).is_ok() {
            return Err("Kiron already active in another duir instance".to_owned());
        }
        let _ = std::fs::remove_file(&path);
    }

    let listener =
        std::os::unix::net::UnixListener::bind(&path).map_err(|e| format!("Failed to bind MCP socket: {e}"))?;

    listener
        .set_nonblocking(false)
        .map_err(|e| format!("Failed to set socket blocking: {e}"))?;

    crate::mcp_log::log("listener", "accepting connections");

    std::thread::spawn(move || {
        for stream in listener.incoming() {
            let Ok(stream) = stream else {
                break;
            };

            crate::mcp_log::log("listener", "client connected");

            let _ = stream.set_read_timeout(Some(std::time::Duration::from_secs(300)));
            let _ = stream.set_write_timeout(Some(std::time::Duration::from_secs(30)));

            let reader = BufReader::new(match stream.try_clone() {
                Ok(s) => s,
                Err(_) => continue,
            });
            let writer = std::io::BufWriter::new(stream);

            let snap = Arc::clone(&snapshot);
            let tx = mutation_tx.clone();

            std::thread::spawn(move || {
                let server = duir_core::mcp_server::McpServer::new(snap, tx);
                let logged_reader = LoggingReader::new(reader);
                let logged_writer = LoggingWriter::new(writer);

                let result = server.run(logged_reader, logged_writer);
                crate::mcp_log::log("listener", &format!("connection closed: {result:?}"));
            });
        }
    });

    Ok(path)
}

/// Ensure the `.kiro/agents/duir.json` agent file exists.
/// Includes the SOP from config as `customInstructions`.
/// Build `.kiro/agents/duir.json` by merging the user's agent config with duir MCP.
/// Always rewrites so that the latest binary path and SOP are used.
pub fn ensure_agent_file(agent_name: &str, sop: &str) {
    let out_path = std::path::PathBuf::from(".kiro/agents/duir.json");
    let _ = std::fs::create_dir_all(".kiro/agents");

    let bin = std::env::current_exe().map_or_else(|_| "duir".to_owned(), |p| p.to_string_lossy().into_owned());

    let duir_mcp = serde_json::json!({
        "command": bin,
        "args": ["--mcp-connect"],
        "env": {"DUIR_MCP_SOCKET": "${DUIR_MCP_SOCKET}"},
        "autoApprove": ["*"]
    });

    // Load base agent if not "duir"
    let mut config = if agent_name == "duir" {
        serde_json::Map::new()
    } else {
        load_agent_config(agent_name).unwrap_or_default()
    };

    config.insert("name".to_owned(), serde_json::json!("duir"));

    // Merge MCP servers
    let servers = config.entry("mcpServers").or_insert_with(|| serde_json::json!({}));
    if let Some(obj) = servers.as_object_mut() {
        obj.insert("duir".to_owned(), duir_mcp);
    }

    // Append duir SOP to existing instructions
    let existing = config
        .get("customInstructions")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_owned();
    let merged = if existing.is_empty() {
        sop.to_owned()
    } else {
        format!("{existing}\n\n{sop}")
    };
    config.insert("customInstructions".to_owned(), serde_json::json!(merged));

    config.entry("includeMcpJson").or_insert(serde_json::json!(true));
    config.entry("tools").or_insert(serde_json::json!(["*"]));

    // Ensure @duir in allowedTools
    let tools = config.entry("allowedTools").or_insert_with(|| serde_json::json!([]));
    if let Some(arr) = tools.as_array_mut() {
        let tag = serde_json::json!("@duir");
        if !arr.contains(&tag) {
            arr.push(tag);
        }
    }

    let json = serde_json::to_string_pretty(&config).unwrap_or_default();
    let _ = std::fs::write(&out_path, json);
}

fn load_agent_config(name: &str) -> Option<serde_json::Map<String, serde_json::Value>> {
    let filename = format!("{name}.json");
    let candidates = [
        std::path::PathBuf::from(format!(".kiro/agents/{filename}")),
        dirs::home_dir()
            .unwrap_or_default()
            .join(format!(".kiro/agents/{filename}")),
    ];
    for path in &candidates {
        if let Ok(content) = std::fs::read_to_string(path)
            && let Ok(serde_json::Value::Object(map)) = serde_json::from_str(&content)
        {
            return Some(map);
        }
    }
    None
}

/// Test-only wrapper for `mcp_socket_path`.
#[cfg(test)]
pub fn mcp_socket_path_for_test(s: &str) -> std::path::PathBuf {
    mcp_socket_path(s)
}

/// Test-only wrapper for `start_mcp_listener`.
#[cfg(test)]
pub fn start_mcp_listener_for_test(
    snap: std::sync::Arc<std::sync::Mutex<duir_core::TodoFile>>,
    tx: std::sync::mpsc::Sender<duir_core::mcp_server::McpMutation>,
    s: &str,
) -> Result<std::path::PathBuf, String> {
    start_mcp_listener(snap, tx, s)
}
