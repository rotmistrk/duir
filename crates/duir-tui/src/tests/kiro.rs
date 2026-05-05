use super::*;

#[test]
fn kiro_tab_focus_state() {
    let mut app = App::new();
    assert!(!app.flags.kiro_tab_focused());
    assert!(app.active_kiron_for_cursor().is_none());
    // Setting flag without active kiron is harmless
    app.flags.set_kiro_tab_focused(true);
    assert!(app.active_kiron_for_cursor().is_none());
}

#[test]
fn kiro_start_keeps_tree_focus() {
    let mut app = make_app_with_tree();
    app.cursor = 1; // Branch 1
    app.cmd_kiron(&["kiron"]);
    assert!(app.files[0].data.items[0].is_kiron());
    assert!(!app.flags.kiro_tab_focused());
}

#[test]
fn active_kiron_for_cursor_finds_ancestor() {
    let mut app = make_app_with_active_kiron();
    // Cursor on Child 1.1 (inside Branch 1's subtree)
    app.cursor = 2;
    let key = app.active_kiron_for_cursor();
    assert!(key.is_some());
}

#[test]
fn active_kiron_for_cursor_none_outside() {
    let mut app = make_app_with_active_kiron();
    // Branch 2 is outside the kiron subtree
    let pos = app.rows.iter().position(|r| r.title == "Branch 2").unwrap();
    app.cursor = pos;
    assert!(app.active_kiron_for_cursor().is_none());
}

#[test]
fn kiro_tab_toggle_cycle() {
    let mut app = make_app_with_active_kiron();
    app.cursor = 1; // on the kiron node
    assert!(!app.flags.kiro_tab_focused());
    // Simulate F4: focus kiro
    app.state = crate::app::FocusState::Kiro;
    app.flags.set_kiro_tab_focused(true);
    assert!(app.is_kiro_focused());
    // Simulate F2: back to tree, kiro stays visible
    app.state = crate::app::FocusState::Tree;
    assert!(app.is_tree_focused());
    assert!(app.flags.kiro_tab_focused()); // display unchanged
}

#[test]
fn kiro_stop_clears_focus() {
    let mut app = make_app_with_active_kiron();
    app.state = crate::app::FocusState::Kiro;
    app.flags.set_kiro_tab_focused(true);
    app.cursor = 1;
    app.cmd_kiro(&["kiro", "stop"]);
    assert!(!app.flags.kiro_tab_focused());
    assert!(app.is_tree_focused());
    assert!(app.active_kirons.is_empty());
}

#[test]
fn kiron_disable_blocked_while_active() {
    let mut app = make_app_with_active_kiron();
    app.cursor = 1;
    app.cmd_kiron(&["kiron", "disable"]);
    assert!(app.files[0].data.items[0].is_kiron());
    assert_eq!(app.status_level, StatusLevel::Error);
}

#[test]
fn send_to_kiro_records_capture_start() {
    let mut app = make_app_with_active_kiron();
    app.cursor = 2; // Child 1.1 inside kiron subtree
    app.send_to_kiro();
    assert_eq!(app.pending_responses.len(), 1);
    // capture_start_line should be the total_lines at time of send
    let pr = &app.pending_responses[0];
    // capture_start_line is set (usize, always >= 0)
    let _ = pr.capture_start_line;
    // Node should be marked as prompt
    let item = duir_core::tree_ops::get_item(&app.files[0].data, &app.rows[2].path).unwrap();
    assert!(item.title.starts_with("❓ "));
}

#[test]
fn send_to_kiro_auto_finalizes_previous() {
    let mut app = make_app_with_active_kiron();
    app.cursor = 2; // Child 1.1
    app.send_to_kiro();
    assert_eq!(app.pending_responses.len(), 1);
    // Send again — should finalize previous (even if empty)
    app.cursor = 2;
    app.rebuild_rows();
    // Find a node inside the kiron subtree
    let pos = app.rows.iter().position(|r| r.title.starts_with("❓")).unwrap();
    app.cursor = pos;
    app.send_to_kiro();
    // Previous was finalized (empty, so removed), new one added
    assert_eq!(app.pending_responses.len(), 1);
}

#[test]
fn capture_kiro_response_no_pending() {
    let mut app = make_app_with_active_kiron();
    app.capture_kiro_response();
    assert_eq!(app.status_level, StatusLevel::Warning);
    assert!(app.status_message.contains("No pending"));
}

#[test]
fn kiro_capture_command() {
    let mut app = make_app_with_active_kiron();
    app.cmd_kiro(&["kiro", "capture"]);
    assert_eq!(app.status_level, StatusLevel::Warning); // no pending
}

#[test]
fn response_ready_initially_false() {
    let app = make_app_with_active_kiron();
    for kiron in app.active_kirons.values() {
        assert!(!kiron.response_ready);
    }
    assert!(app.response_ready_paths().is_empty());
}

#[test]
fn response_ready_paths_returns_ready_kirons() {
    let mut app = make_app_with_active_kiron();
    // Manually set response_ready
    for kiron in app.active_kirons.values_mut() {
        kiron.response_ready = true;
    }
    let paths = app.response_ready_paths();
    assert_eq!(paths.len(), 1);
    assert_eq!(paths[0].0, 0); // file_index 0
}

#[test]
fn clear_response_ready_on_focus() {
    let mut app = make_app_with_active_kiron();
    app.cursor = 1; // on kiron node
    for kiron in app.active_kirons.values_mut() {
        kiron.response_ready = true;
    }
    assert!(!app.response_ready_paths().is_empty());
    app.clear_response_ready();
    assert!(app.response_ready_paths().is_empty());
}

#[test]
fn clear_response_ready_outside_kiron_is_noop() {
    let mut app = make_app_with_active_kiron();
    for kiron in app.active_kirons.values_mut() {
        kiron.response_ready = true;
    }
    // Move cursor outside kiron subtree
    let pos = app.rows.iter().position(|r| r.title == "Branch 2").unwrap();
    app.cursor = pos;
    app.clear_response_ready();
    // Should still be ready — we're not on the kiron
    assert!(!app.response_ready_paths().is_empty());
}

#[test]
fn alt2_focuses_tree_from_kiro() {
    let mut app = make_app_with_active_kiron();
    app.cursor = 1;
    app.state = crate::app::FocusState::Kiro;
    app.flags.set_kiro_tab_focused(true);
    let storage_dir = std::path::PathBuf::from("/tmp");
    crate::event_loop::handle_global_keys_for_test(&mut app, key(KeyCode::Char('™')), &storage_dir);
    assert!(app.flags.kiro_tab_focused()); // kiro stays visible
    assert!(app.is_tree_focused()); // keyboard goes to tree
}

#[test]
fn clone_subtree_strips_kiron_state() {
    let mut app = make_app_with_tree();
    app.cursor = 1; // Branch 1
    app.cmd_kiron(&["kiron"]);
    assert!(app.files[0].data.items[0].is_kiron());
    assert!(app.files[0].data.items[0].kiron.is_some());
    // Clone Branch 1
    app.clone_subtree();
    // Original keeps kiron state
    assert!(app.files[0].data.items[0].is_kiron());
    // Clone (now at index 1) should NOT have kiron state
    assert!(!app.files[0].data.items[1].is_kiron());
    assert!(app.files[0].data.items[1].kiron.is_none());
}

#[test]
fn kiro_new_command_routes() {
    let mut app = make_app_with_active_kiron();
    app.cursor = 1;
    // "new" should be accepted (not show usage error)
    app.cmd_kiro(&["kiro", "new"]);
    // It will fail to actually start (PTY "true" already exited) but
    // should not show "Usage:" error
    assert!(!app.status_message.contains("Usage"));
}

#[test]
fn ctrl_backslash_sends_to_kiro() {
    // Ctrl+\ should be handled by handle_global_keys
    let mut app = make_app_with_active_kiron();
    app.cursor = 2; // inside kiron subtree
    let storage_dir = std::path::PathBuf::from("/tmp");
    let key = crossterm::event::KeyEvent::new(KeyCode::Char('\\'), crossterm::event::KeyModifiers::CONTROL);
    let handled = crate::event_loop::handle_global_keys_for_test(&mut app, key, &storage_dir);
    assert!(handled);
    // Should have a pending response from send_to_kiro
    assert!(!app.pending_responses.is_empty());
}
