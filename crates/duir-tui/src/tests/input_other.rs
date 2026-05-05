use super::*;

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
