use super::*;

#[test]
fn close_current_file_unsaved_blocked() {
    let mut app = make_app_with_tree();
    app.mark_modified(0, &[]);
    app.close_current_file();
    assert_eq!(app.files.len(), 1); // not removed
    assert!(app.status_message.contains("unsaved"));
}

#[test]
fn close_current_file_saved_removes() {
    let mut app = make_app_with_tree();
    // modified defaults to false from add_file
    app.close_current_file();
    assert!(app.flags.should_quit()); // last file → quit
}

#[test]
fn apply_filter_exclude_mode() {
    let mut app = make_app_with_tree();
    app.filter_committed_text = "Branch 1".to_owned();
    app.flags.set_filter_committed_exclude(true);
    app.apply_filter();
    // Branch 1 should be hidden
    assert!(!app.rows.iter().any(|r| r.title == "Branch 1"));
    assert!(app.status_message.contains("exclude"));
}

#[test]
fn apply_filter_live_updates() {
    let mut app = make_app_with_tree();
    let total = app.rows.len();
    app.state = FocusState::Filter {
        text: "Child 1.1".to_owned(),
        saved: String::new(),
    };
    app.apply_filter_live();
    assert!(app.rows.len() < total);
    assert!(app.status_message.contains("matches"));
}

#[test]
fn apply_filter_live_empty_restores() {
    let mut app = make_app_with_tree();
    let total = app.rows.len();
    app.state = FocusState::Filter {
        text: "Child".to_owned(),
        saved: String::new(),
    };
    app.apply_filter_live();
    if let FocusState::Filter { ref mut text, .. } = app.state {
        text.clear();
    }
    app.apply_filter_live();
    assert_eq!(app.rows.len(), total);
}

#[test]
fn apply_filter_live_exclude_preview() {
    let mut app = make_app_with_tree();
    app.state = FocusState::Filter {
        text: "!Branch 1".to_owned(),
        saved: String::new(),
    };
    app.apply_filter_live();
    assert!(!app.rows.iter().any(|r| r.title == "Branch 1"));
}

#[test]
fn mark_modified_invalidates_cipher() {
    let mut app = make_app_with_tree();
    app.cursor = 1;
    // Encrypt Branch 1
    app.cmd_encrypt();
    let cb = app.password_prompt.take().unwrap().callback;
    app.handle_password_result("pass", cb);
    assert!(app.files[0].data.items[0].cipher.is_some());

    // Unlock it
    app.cursor = 1;
    app.expand_current();
    let cb = app.password_prompt.take().unwrap().callback;
    app.handle_password_result("pass", cb);
    let cipher_before = app.files[0].data.items[0].cipher.clone();

    // Modify a child — should invalidate parent cipher
    let child_path: duir_core::tree_ops::TreePath = [0, 0].into();
    if let Some(child) = duir_core::tree_ops::get_item_mut(&mut app.files[0].data, &child_path) {
        child.title = "Modified".to_owned();
    }
    app.mark_modified(0, &child_path);
    assert_ne!(app.files[0].data.items[0].cipher, cipher_before);
}

#[test]
fn pending_delete_cleared_on_other_key() {
    let mut app = make_app_with_tree();
    app.cursor = 1;
    app.delete_current();
    assert!(app.flags.pending_delete());
    // Press any key other than 'y'
    input::handle_key(&mut app, key(KeyCode::Char('n')));
    // pending_delete cleared (though 'n' also creates sibling)
    assert!(!app.flags.pending_delete());
}

#[test]
fn pending_delete_y_confirms() {
    let mut app = make_app_with_tree();
    app.cursor = 1;
    app.delete_current();
    assert!(app.flags.pending_delete());
    input::handle_key(&mut app, key(KeyCode::Char('y')));
    assert!(!app.flags.pending_delete());
    assert_ne!(app.files[0].data.items[0].title, "Branch 1");
}

#[test]
fn space_toggles_completion() {
    let mut app = make_app_with_tree();
    app.cursor = 1;
    let before = app.files[0].data.items[0].completed.clone();
    input::handle_key(&mut app, key(KeyCode::Char(' ')));
    assert_ne!(app.files[0].data.items[0].completed, before);
}

#[test]
fn e_starts_editing() {
    let mut app = make_app_with_tree();
    app.cursor = 1;
    input::handle_key(&mut app, key(KeyCode::Char('e')));
    assert!(app.is_editing_title());
}

#[test]
fn enter_is_noop_in_tree() {
    let mut app = make_app_with_tree();
    app.cursor = 1;
    input::handle_key(&mut app, key(KeyCode::Enter));
    assert!(!app.is_editing_title());
}

#[test]
fn bracket_resizes_note_panel() {
    let mut app = make_app_with_tree();
    let before = app.note_panel_pct;
    input::handle_key(&mut app, key(KeyCode::Char(']')));
    assert_eq!(app.note_panel_pct, before + 5);
    input::handle_key(&mut app, key(KeyCode::Char('[')));
    assert_eq!(app.note_panel_pct, before);
}
