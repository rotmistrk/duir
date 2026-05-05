use super::*;

// ── filter mode ─────────────────────────────────────────────────

#[test]
fn input_filter_typing() {
    let mut app = make_app_with_tree();
    app.state = FocusState::Filter {
        text: String::new(),
        saved: String::new(),
    };
    input::handle_key(&mut app, key(KeyCode::Char('C')));
    input::handle_key(&mut app, key(KeyCode::Char('h')));
    let FocusState::Filter { ref text, .. } = app.state else {
        unreachable!();
    };
    assert_eq!(text, "Ch");
}

#[test]
fn input_filter_enter_applies() {
    let mut app = make_app_with_tree();
    app.state = FocusState::Filter {
        text: "Child 1.1".to_owned(),
        saved: String::new(),
    };
    input::handle_key(&mut app, key(KeyCode::Enter));
    assert!(!app.is_filter_active());
    // Filter applied — fewer rows
    assert!(app.rows.iter().filter(|r| !r.flags.is_file_root()).count() < 6);
}

#[test]
fn input_filter_esc_reverts() {
    let mut app = make_app_with_tree();
    let total = app.rows.len();
    app.state = FocusState::Filter {
        text: "xyz".to_owned(),
        saved: String::new(),
    };
    input::handle_key(&mut app, key(KeyCode::Esc));
    assert!(!app.is_filter_active());
    assert!(app.filter_committed_text.is_empty());
    assert_eq!(app.rows.len(), total);
}

#[test]
fn input_filter_exclude_prefix() {
    let mut app = make_app_with_tree();
    app.state = FocusState::Filter {
        text: "!Branch 1".to_owned(),
        saved: String::new(),
    };
    input::handle_key(&mut app, key(KeyCode::Enter));
    assert!(app.flags.filter_committed_exclude());
    assert_eq!(app.filter_committed_text, "Branch 1");
}

#[test]
fn input_filter_backspace() {
    let mut app = make_app_with_tree();
    app.state = FocusState::Filter {
        text: "abc".to_owned(),
        saved: String::new(),
    };
    input::handle_key(&mut app, key(KeyCode::Backspace));
    let FocusState::Filter { ref text, .. } = app.state else {
        unreachable!();
    };
    assert_eq!(text, "ab");
}

// ── command mode ────────────────────────────────────────────────

#[test]
fn input_command_typing() {
    let mut app = make_app_with_tree();
    app.state = FocusState::Command {
        buffer: String::new(),
        history_index: None,
    };
    input::handle_key(&mut app, key(KeyCode::Char('h')));
    input::handle_key(&mut app, key(KeyCode::Char('e')));
    if let FocusState::Command { ref buffer, .. } = app.state {
        assert_eq!(buffer, "he");
    } else {
        unreachable!();
    }
}

#[test]
fn input_command_esc_cancels() {
    let mut app = make_app_with_tree();
    app.state = FocusState::Command {
        buffer: "help".to_owned(),
        history_index: None,
    };
    input::handle_key(&mut app, key(KeyCode::Esc));
    assert!(!app.is_command_active());
}

#[test]
fn input_command_enter_pushes_history() {
    let mut app = make_app_with_tree();
    app.state = FocusState::Command {
        buffer: "help".to_owned(),
        history_index: None,
    };
    input::handle_key(&mut app, key(KeyCode::Enter));
    assert_eq!(app.command_history.last().unwrap(), "help");
}

#[test]
fn input_command_tab_completes() {
    let mut app = make_app_with_tree();
    app.state = FocusState::Command {
        buffer: "hel".to_owned(),
        history_index: None,
    };
    input::handle_key(&mut app, key(KeyCode::Tab));
    if let FocusState::Command { ref buffer, .. } = app.state {
        assert_eq!(buffer, "help");
    } else {
        unreachable!();
    }
}

#[test]
fn input_command_up_down_history() {
    let mut app = make_app_with_tree();
    app.command_history = vec!["first".to_owned(), "second".to_owned()];
    app.state = FocusState::Command {
        buffer: String::new(),
        history_index: None,
    };
    // Up → last history entry
    input::handle_key(&mut app, key(KeyCode::Up));
    if let FocusState::Command { ref buffer, .. } = app.state {
        assert_eq!(buffer, "second");
    }
    // Up again → first
    input::handle_key(&mut app, key(KeyCode::Up));
    if let FocusState::Command { ref buffer, .. } = app.state {
        assert_eq!(buffer, "first");
    }
    // Down → second
    input::handle_key(&mut app, key(KeyCode::Down));
    if let FocusState::Command { ref buffer, .. } = app.state {
        assert_eq!(buffer, "second");
    }
    // Down past end → clears
    input::handle_key(&mut app, key(KeyCode::Down));
    if let FocusState::Command { ref buffer, .. } = app.state {
        assert!(buffer.is_empty());
    }
}

#[test]
fn input_command_backspace_on_empty_exits() {
    let mut app = make_app_with_tree();
    app.state = FocusState::Command {
        buffer: String::new(),
        history_index: None,
    };
    input::handle_key(&mut app, key(KeyCode::Backspace));
    assert!(!app.is_command_active());
}

#[test]
fn input_command_backspace_deletes_char() {
    let mut app = make_app_with_tree();
    app.state = FocusState::Command {
        buffer: "hel".to_owned(),
        history_index: None,
    };
    input::handle_key(&mut app, key(KeyCode::Backspace));
    if let FocusState::Command { ref buffer, .. } = app.state {
        assert_eq!(buffer, "he");
    }
    assert!(app.is_command_active());
}

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
