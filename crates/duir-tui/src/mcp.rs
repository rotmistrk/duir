//! MCP server — exposes duir tree operations over Unix socket.
//!
//! Uses duir-core's `McpServer` with a shared `TodoFile`. Mutations from MCP
//! clients are applied to the file and saved to disk. The tree view's
//! `reload_if_changed` picks up changes on the next tick.

use std::io::{BufReader, BufWriter};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, mpsc};
use std::thread;
use std::{fs, io};

use duir_core::mcp_server::{McpMutation, McpServer, ToolFilter};
use duir_core::model::TodoFile;

use crate::mcp_permissions::{Permissions, categorize_tool};
use crate::todo_tree::model;

/// Start the MCP listener on `.duir/mcp.sock`.
/// Returns the socket path (for cleanup on exit).
#[must_use]
pub fn start_mcp(root_dir: &Path) -> Option<PathBuf> {
    let duir_dir = root_dir.join(".duir");
    let sock_path = duir_dir.join("mcp.sock");
    let file_path = duir_dir.join("todo.json");
    let permissions = Arc::new(Permissions::load(root_dir));

    if let Err(e) = fs::create_dir_all(&duir_dir) {
        log::error!("MCP: cannot create {}: {e}", duir_dir.display());
        return None;
    }

    // Clean up stale socket
    if sock_path.exists() {
        if UnixStream::connect(&sock_path).is_ok() {
            log::warn!("MCP: another instance running, skipping");
            return None;
        }
        let _ = fs::remove_file(&sock_path);
    }

    let listener = match UnixListener::bind(&sock_path) {
        Ok(l) => l,
        Err(e) => {
            eprintln!("duir: MCP socket bind failed: {}: {e}", sock_path.display());
            log::error!("MCP: failed to bind {}: {e}", sock_path.display());
            return None;
        }
    };

    let file = Arc::new(Mutex::new(model::load_todo_file(&file_path)));

    thread::spawn(move || {
        accept_loop(listener, file, file_path, permissions);
    });

    log::info!("MCP: listening on {}", sock_path.display());
    Some(sock_path)
}

fn accept_loop(listener: UnixListener, file: Arc<Mutex<TodoFile>>, save_path: PathBuf, permissions: Arc<Permissions>) {
    let (tx, rx) = mpsc::channel();

    // Mutation applier thread
    let file_for_apply = Arc::clone(&file);
    thread::spawn(move || {
        for mutation in rx {
            apply_mutation(&file_for_apply, &mutation);
            if let Ok(f) = file_for_apply.lock()
                && !model::save_todo_file(&save_path, &f)
            {
                log::error!("MCP: failed to save after mutation");
            }
        }
    });

    for stream in listener.incoming() {
        let Ok(stream) = stream else { break };
        let _ = stream.set_read_timeout(None);
        let _ = stream.set_write_timeout(Some(std::time::Duration::from_secs(30)));
        let file = Arc::clone(&file);
        let tx = tx.clone();
        let perms = Arc::clone(&permissions);
        thread::spawn(move || {
            if let Err(e) = handle_connection(stream, file, tx, perms) {
                log::warn!("MCP client error: {e}");
            }
        });
    }
}

fn handle_connection(
    stream: UnixStream,
    file: Arc<Mutex<TodoFile>>,
    tx: mpsc::Sender<McpMutation>,
    permissions: Arc<Permissions>,
) -> io::Result<()> {
    let reader = BufReader::new(stream.try_clone()?);
    let writer = BufWriter::new(stream);
    let filter: ToolFilter = {
        let perms = Arc::clone(&permissions);
        Arc::new(move |tool_name: &str| {
            let cat = categorize_tool(tool_name);
            perms.is_allowed(tool_name, cat)
        })
    };
    let server = McpServer::new(file, tx).with_filter(filter);
    server.run(reader, writer)
}

fn apply_mutation(file: &Arc<Mutex<TodoFile>>, mutation: &McpMutation) {
    let Ok(mut f) = file.lock() else { return };
    match mutation {
        McpMutation::AddChild {
            parent_path,
            title,
            note,
        } => {
            let mut item = duir_core::TodoItem::new(title);
            item.note.clone_from(note);
            let _ = duir_core::tree_ops::add_child(&mut f, parent_path, item);
        }
        McpMutation::AddSibling { path, title, note } => {
            let mut item = duir_core::TodoItem::new(title);
            item.note.clone_from(note);
            let _ = duir_core::tree_ops::add_sibling(&mut f, path, item);
        }
        McpMutation::MarkDone { path } => {
            if let Some(item) = duir_core::tree_ops::get_item_mut(&mut f, path) {
                item.completed = duir_core::model::Completion::Done;
            }
        }
        McpMutation::MarkImportant { path } => {
            if let Some(item) = duir_core::tree_ops::get_item_mut(&mut f, path) {
                item.important = !item.important;
            }
        }
        McpMutation::Reorder { path, direction } => match direction {
            duir_core::mcp_server::ReorderDirection::Up => {
                let _ = duir_core::tree_ops::swap_up(&mut f, path);
            }
            duir_core::mcp_server::ReorderDirection::Down => {
                let _ = duir_core::tree_ops::swap_down(&mut f, path);
            }
        },
        McpMutation::SetPriority { path, value } => {
            if let Some(item) = duir_core::tree_ops::get_item_mut(&mut f, path) {
                item.priority = if *value == 0 { None } else { Some(*value) };
            }
        }
        McpMutation::SetEffort { path, value } => {
            if let Some(item) = duir_core::tree_ops::get_item_mut(&mut f, path) {
                item.effort = if *value == 0 { None } else { Some(*value) };
            }
        }
        McpMutation::SetStatus { path, status } => {
            if let Some(item) = duir_core::tree_ops::get_item_mut(&mut f, path) {
                item.work_status = match status.as_str() {
                    "in_progress" => duir_core::model::WorkStatus::InProgress,
                    "paused" => duir_core::model::WorkStatus::Paused,
                    _ => duir_core::model::WorkStatus::Idle,
                };
            }
        }
        McpMutation::SetNote { path, content } => {
            if let Some(item) = duir_core::tree_ops::get_item_mut(&mut f, path) {
                item.note.clone_from(content);
            }
        }
        McpMutation::Promote { path } => {
            let _ = duir_core::tree_ops::promote(&mut f, path);
        }
        McpMutation::Demote { path } => {
            let _ = duir_core::tree_ops::demote(&mut f, path);
        }
        McpMutation::Fold { path } => {
            if let Some(item) = duir_core::tree_ops::get_item_mut(&mut f, path) {
                item.folded = true;
            }
        }
        McpMutation::Unfold { path } => {
            if let Some(item) = duir_core::tree_ops::get_item_mut(&mut f, path) {
                item.folded = false;
            }
        }
        McpMutation::Remove { path } => {
            let _ = duir_core::tree_ops::remove_item(&mut f, path);
        }
    }
}

/// Clean up socket on shutdown.
pub fn cleanup_mcp(sock_path: &Path) {
    let _ = fs::remove_file(sock_path);
}
