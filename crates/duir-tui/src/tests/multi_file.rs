#![allow(clippy::panic)]
use super::*;

// ── cross-file move ─────────────────────────────────────────────

#[test]
fn move_item_to_next_file() {
    let mut app = make_app_multi_file();
    // cursor on A-third (last item in file-a)
    // rows: [0]=file-a, [1]=A-first, [2]=A-second, [3]=A-third, [4]=file-b, [5]=B-first, [6]=B-second
    app.cursor = 3; // A-third
    assert_eq!(app.rows[app.cursor].path, vec![2]);
    app.swap_down(); // already last → should move to file-b
    assert_eq!(app.files[0].data.items.len(), 2); // file-a lost one
    assert_eq!(app.files[1].data.items.len(), 3); // file-b gained one
    assert_eq!(app.files[1].data.items[0].title, "A-third"); // inserted at top
}

#[test]
fn move_item_to_prev_file() {
    let mut app = make_app_multi_file();
    // cursor on B-first (first item in file-b)
    app.cursor = 5; // B-first
    assert_eq!(app.rows[app.cursor].path, vec![0]);
    assert_eq!(app.files[1].data.items[0].title, "B-first");
    app.swap_up(); // already first → should move to file-a
    assert_eq!(app.files[0].data.items.len(), 4); // file-a gained one
    assert_eq!(app.files[1].data.items.len(), 1); // file-b lost one
    assert_eq!(app.files[0].data.items[3].title, "B-first"); // appended at end
}

#[test]
fn move_does_not_cross_when_nested() {
    let mut app = make_app_with_tree();
    // Child 1.1 is nested (path [0,0]) — should NOT cross file boundary
    app.cursor = 2; // Child 1.1
    let items_before = app.files[0].data.items[0].items.len();
    app.swap_up(); // first child, swap_up fails, but nested → no cross-file
    assert_eq!(app.files[0].data.items[0].items.len(), items_before);
}

#[test]
fn no_move_past_first_file() {
    let mut app = make_app_multi_file();
    app.cursor = 1; // A-first (first item in first file)
    app.swap_up(); // should do nothing (no file above)
    assert_eq!(app.files[0].data.items.len(), 3);
}

#[test]
fn no_move_past_last_file() {
    let mut app = make_app_multi_file();
    app.cursor = 6; // B-second (last item in last file)
    app.swap_down(); // should do nothing (no file below)
    assert_eq!(app.files[1].data.items.len(), 2);
}

// ── file reorder ────────────────────────────────────────────────

#[test]
fn reorder_file_down() {
    let mut app = make_app_multi_file();
    app.cursor = 0; // file-a header
    assert!(app.rows[0].is_file_root);
    app.swap_down(); // reorder file-a below file-b
    assert_eq!(app.files[0].name, "file-b");
    assert_eq!(app.files[1].name, "file-a");
}

#[test]
fn reorder_file_up() {
    let mut app = make_app_multi_file();
    app.cursor = 4; // file-b header
    assert!(app.rows[4].is_file_root);
    app.swap_up(); // reorder file-b above file-a
    assert_eq!(app.files[0].name, "file-b");
    assert_eq!(app.files[1].name, "file-a");
}

#[test]
fn reorder_first_file_up_noop() {
    let mut app = make_app_multi_file();
    app.cursor = 0; // file-a header (already first)
    app.swap_up();
    assert_eq!(app.files[0].name, "file-a");
}

#[test]
fn reorder_last_file_down_noop() {
    let mut app = make_app_multi_file();
    app.cursor = 4; // file-b header (already last)
    app.swap_down();
    assert_eq!(app.files[1].name, "file-b");
}

// ── help search ─────────────────────────────────────────────────

#[test]
fn help_search_activates_on_slash() {
    let mut app = make_app_with_tree();
    app.state = FocusState::Help {
        scroll: 0,
        search: String::new(),
    };
    crate::help::handle_help_input(&mut app, key(KeyCode::Char('/')));
    let FocusState::Help { ref search, .. } = app.state else {
        panic!("expected Help state");
    };
    assert!(!search.is_empty());
}

#[test]
fn help_search_typing_filters() {
    let mut app = make_app_with_tree();
    app.state = FocusState::Help {
        scroll: 0,
        search: "\0".to_owned(),
    };
    crate::help::handle_help_input(&mut app, key(KeyCode::Char('e')));
    crate::help::handle_help_input(&mut app, key(KeyCode::Char('x')));
    let FocusState::Help { ref search, .. } = app.state else {
        panic!("expected Help state");
    };
    assert_eq!(search, "ex");
}

#[test]
fn help_search_esc_clears() {
    let mut app = make_app_with_tree();
    app.state = FocusState::Help {
        scroll: 5,
        search: "test".to_owned(),
    };
    crate::help::handle_help_input(&mut app, key(KeyCode::Esc));
    let FocusState::Help { ref search, scroll, .. } = app.state else {
        panic!("expected Help state");
    };
    assert!(search.is_empty());
    assert_eq!(scroll, 0);
}

#[test]
fn help_q_closes_when_not_searching() {
    let mut app = make_app_with_tree();
    app.state = FocusState::Help {
        scroll: 0,
        search: String::new(),
    };
    crate::help::handle_help_input(&mut app, key(KeyCode::Char('q')));
    assert!(matches!(app.state, FocusState::Tree));
}
