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
