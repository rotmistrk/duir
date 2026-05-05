use super::*;

// ── edit mode (title editing) ───────────────────────────────────

#[test]
fn input_edit_chars() {
    let mut app = make_app_with_tree();
    app.cursor = 1;
    app.start_editing();
    if let FocusState::EditingTitle {
        ref mut select_all,
        ref mut cursor,
        ref buffer,
        ..
    } = app.state
    {
        *select_all = false;
        *cursor = buffer.len();
    }
    input::handle_key(&mut app, key(KeyCode::Char('X')));
    if let FocusState::EditingTitle { ref buffer, .. } = app.state {
        assert!(buffer.ends_with('X'));
    }
}

#[test]
fn input_edit_backspace() {
    let mut app = make_app_with_tree();
    app.cursor = 1;
    app.start_editing();
    let orig_len = if let FocusState::EditingTitle {
        ref mut select_all,
        ref buffer,
        ..
    } = app.state
    {
        *select_all = false;
        buffer.len()
    } else {
        0
    };
    input::handle_key(&mut app, key(KeyCode::Backspace));
    if let FocusState::EditingTitle { ref buffer, .. } = app.state {
        assert_eq!(buffer.len(), orig_len - 1);
    }
}

#[test]
fn input_edit_delete() {
    let mut app = make_app_with_tree();
    app.cursor = 1;
    app.start_editing();
    let orig_len = if let FocusState::EditingTitle {
        ref mut select_all,
        ref mut cursor,
        ref buffer,
    } = app.state
    {
        *select_all = false;
        *cursor = 0;
        buffer.len()
    } else {
        0
    };
    input::handle_key(&mut app, key(KeyCode::Delete));
    if let FocusState::EditingTitle { ref buffer, .. } = app.state {
        assert_eq!(buffer.len(), orig_len - 1);
    }
}

#[test]
fn input_edit_arrows_home_end() {
    let mut app = make_app_with_tree();
    app.cursor = 1;
    app.start_editing();
    if let FocusState::EditingTitle { ref mut select_all, .. } = app.state {
        *select_all = false;
    }
    input::handle_key(&mut app, key(KeyCode::Home));
    if let FocusState::EditingTitle { cursor, .. } = app.state {
        assert_eq!(cursor, 0);
    }
    input::handle_key(&mut app, key(KeyCode::End));
    if let FocusState::EditingTitle { cursor, ref buffer, .. } = app.state {
        assert_eq!(cursor, buffer.len());
    }
    input::handle_key(&mut app, key(KeyCode::Left));
    let pos = if let FocusState::EditingTitle { cursor, .. } = app.state {
        cursor
    } else {
        0
    };
    input::handle_key(&mut app, key(KeyCode::Right));
    if let FocusState::EditingTitle { cursor, .. } = app.state {
        assert_eq!(cursor, pos + 1);
    }
}

#[test]
fn input_edit_enter_finishes() {
    let mut app = make_app_with_tree();
    app.cursor = 1;
    app.start_editing();
    if let FocusState::EditingTitle {
        ref mut buffer,
        ref mut select_all,
        ..
    } = app.state
    {
        *select_all = false;
        "Renamed".clone_into(buffer);
    }
    input::handle_key(&mut app, key(KeyCode::Enter));
    assert!(!app.is_editing_title());
    assert_eq!(app.files[0].data.items[0].title, "Renamed");
}

#[test]
fn input_edit_esc_cancels() {
    let mut app = make_app_with_tree();
    app.cursor = 1;
    app.start_editing();
    input::handle_key(&mut app, key(KeyCode::Esc));
    assert!(!app.is_editing_title());
    assert_eq!(app.files[0].data.items[0].title, "Branch 1");
}

// ── note mode ───────────────────────────────────────────────────

#[test]
fn input_note_tab_back_to_tree() {
    let mut app = make_app_with_tree();
    app.cursor = 1;
    app.focus_note();
    // Editor starts in Normal mode, Tab returns to tree
    input::handle_key(&mut app, key(KeyCode::Tab));
    assert!(app.is_tree_focused());
}
