// MCP tests use crate:: paths directly.

#[test]
fn mcp_socket_path_contains_session_id() {
    let path = crate::app::app_kiron_mcp::mcp_socket_path_for_test("abc123");
    assert!(path.to_string_lossy().contains("duir-mcp-abc123.sock"));
}

#[test]
fn mcp_apply_add_child() {
    let mut file = duir_core::TodoFile::new("test");
    file.items.push(duir_core::TodoItem::new("task1"));
    let mutation = duir_core::mcp_server::McpMutation::AddChild {
        parent_path: vec![0],
        title: "subtask".to_owned(),
        note: String::new(),
    };
    let result = crate::app::app_kiron::apply_mcp_mutation_for_test(&mut file, &[], &mutation);
    assert!(result);
    assert_eq!(file.items[0].items.len(), 1);
    assert_eq!(file.items[0].items[0].title, "subtask");
}

#[test]
fn mcp_apply_mark_done() {
    let mut file = duir_core::TodoFile::new("test");
    file.items.push(duir_core::TodoItem::new("task1"));
    let mutation = duir_core::mcp_server::McpMutation::MarkDone { path: vec![0] };
    let result = crate::app::app_kiron::apply_mcp_mutation_for_test(&mut file, &[], &mutation);
    assert!(result);
    assert_eq!(file.items[0].completed, duir_core::Completion::Done);
}

#[test]
fn mcp_apply_invalid_path() {
    let mut file = duir_core::TodoFile::new("test");
    let mutation = duir_core::mcp_server::McpMutation::MarkDone { path: vec![99] };
    let result = crate::app::app_kiron::apply_mcp_mutation_for_test(&mut file, &[], &mutation);
    assert!(!result);
}

#[test]
fn mcp_apply_add_sibling() {
    let mut file = duir_core::TodoFile::new("test");
    file.items.push(duir_core::TodoItem::new("task1"));
    file.items.push(duir_core::TodoItem::new("task2"));
    let mutation = duir_core::mcp_server::McpMutation::AddSibling {
        path: vec![0],
        title: "between".to_owned(),
        note: String::new(),
    };
    let result = crate::app::app_kiron::apply_mcp_mutation_for_test(&mut file, &[], &mutation);
    assert!(result);
    assert_eq!(file.items.len(), 3);
    assert_eq!(file.items[1].title, "between");
}

#[test]
fn mcp_apply_mark_important() {
    let mut file = duir_core::TodoFile::new("test");
    file.items.push(duir_core::TodoItem::new("task1"));
    assert!(!file.items[0].important);
    let mutation = duir_core::mcp_server::McpMutation::MarkImportant { path: vec![0] };
    let result = crate::app::app_kiron::apply_mcp_mutation_for_test(&mut file, &[], &mutation);
    assert!(result);
    assert!(file.items[0].important);
}

#[test]
fn mcp_apply_reorder() {
    let mut file = duir_core::TodoFile::new("test");
    file.items.push(duir_core::TodoItem::new("first"));
    file.items.push(duir_core::TodoItem::new("second"));
    let mutation = duir_core::mcp_server::McpMutation::Reorder {
        path: vec![1],
        direction: duir_core::mcp_server::ReorderDirection::Up,
    };
    let result = crate::app::app_kiron::apply_mcp_mutation_for_test(&mut file, &[], &mutation);
    assert!(result);
    assert_eq!(file.items[0].title, "second");
    assert_eq!(file.items[1].title, "first");
}

#[test]
fn mcp_apply_with_kiron_path_offset() {
    let mut file = duir_core::TodoFile::new("test");
    let mut branch = duir_core::TodoItem::new("kiron-branch");
    branch.items.push(duir_core::TodoItem::new("child"));
    file.items.push(branch);
    // Mutation path [0] relative to kiron at [0] → absolute [0,0]
    let mutation = duir_core::mcp_server::McpMutation::MarkDone { path: vec![0] };
    let result = crate::app::app_kiron::apply_mcp_mutation_for_test(&mut file, &[0], &mutation);
    assert!(result);
    assert_eq!(file.items[0].items[0].completed, duir_core::Completion::Done);
}

#[test]
fn mcp_ensure_agent_file_creates_and_preserves() {
    let dir = tempfile::tempdir().unwrap();
    let agents_dir = dir.path().join(".kiro/agents");
    std::fs::create_dir_all(&agents_dir).unwrap();
    let path = agents_dir.join("duir.json");

    // Should not exist yet
    assert!(!path.exists());

    // Run ensure in the temp dir
    let original_dir = std::env::current_dir().unwrap();
    std::env::set_current_dir(dir.path()).unwrap();
    crate::app::app_kiron_mcp::ensure_agent_file("test sop");
    std::env::set_current_dir(&original_dir).unwrap();

    assert!(path.exists());
    let content = std::fs::read_to_string(&path).unwrap();
    assert!(content.contains("DUIR_MCP_SOCKET"));
    assert!(content.contains("includeMcpJson"));

    // Write custom content, ensure it's preserved
    std::fs::write(&path, "custom").unwrap();
    std::env::set_current_dir(dir.path()).unwrap();
    crate::app::app_kiron_mcp::ensure_agent_file("test sop");
    std::env::set_current_dir(&original_dir).unwrap();
    assert_eq!(std::fs::read_to_string(&path).unwrap(), "custom");
}

#[test]
fn mcp_socket_already_in_use() {
    let session = uuid::Uuid::new_v4().to_string();
    let path = crate::app::app_kiron_mcp::mcp_socket_path_for_test(&session);
    // Bind a listener to simulate another duir instance
    let _listener = std::os::unix::net::UnixListener::bind(&path).unwrap();
    let snapshot = std::sync::Arc::new(std::sync::Mutex::new(duir_core::TodoFile::new("t")));
    let (tx, _rx) = std::sync::mpsc::channel();
    let result = crate::app::app_kiron_mcp::start_mcp_listener_for_test(snapshot, tx, &session);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("already active"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn mcp_sync_snapshot() {
    let snapshot = std::sync::Arc::new(std::sync::Mutex::new(duir_core::TodoFile::new("old")));
    let mut item = duir_core::TodoItem::new("new title");
    item.note = "new note".to_owned();
    item.items.push(duir_core::TodoItem::new("child"));
    crate::app::app_kiron::sync_mcp_snapshot(&snapshot, &item);
    let guard = snapshot.lock().unwrap();
    assert_eq!(guard.title, "new title");
    assert_eq!(guard.note, "new note");
    assert_eq!(guard.items.len(), 1);
    drop(guard);
}
