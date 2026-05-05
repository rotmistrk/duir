use super::*;

// ── tree-mode navigation ────────────────────────────────────────

#[test]
fn input_tree_up() {
    let mut app = make_app_with_tree();
    app.cursor = 2;
    input::handle_key(&mut app, key(KeyCode::Up));
    assert_eq!(app.cursor, 1);
}

#[test]
fn input_tree_down() {
    let mut app = make_app_with_tree();
    app.cursor = 1;
    input::handle_key(&mut app, key(KeyCode::Down));
    assert_eq!(app.cursor, 2);
}

#[test]
fn input_tree_left_collapses() {
    let mut app = make_app_with_tree();
    app.cursor = 1; // Branch 1 (expanded)
    let rows_before = app.rows.len();
    input::handle_key(&mut app, key(KeyCode::Left));
    assert!(app.rows.len() < rows_before);
}

#[test]
fn input_tree_right_expands() {
    let mut app = make_app_with_tree();
    app.cursor = 1;
    app.collapse_current();
    let rows_collapsed = app.rows.len();
    input::handle_key(&mut app, key(KeyCode::Right));
    assert!(app.rows.len() > rows_collapsed);
}

// ── tree-mode operations ────────────────────────────────────────

#[test]
fn input_tree_n_new_sibling() {
    let mut app = make_app_with_tree();
    app.cursor = 1;
    input::handle_key(&mut app, key(KeyCode::Char('n')));
    assert!(app.is_editing_title());
    assert_eq!(app.files[0].data.items.len(), 4);
}

#[test]
fn input_tree_b_new_child() {
    let mut app = make_app_with_tree();
    app.cursor = 1;
    let old = app.files[0].data.items[0].items.len();
    input::handle_key(&mut app, key(KeyCode::Char('b')));
    assert!(app.is_editing_title());
    assert_eq!(app.files[0].data.items[0].items.len(), old + 1);
}

#[test]
fn input_tree_d_delete() {
    let mut app = make_app_with_tree();
    app.cursor = 1;
    input::handle_key(&mut app, key(KeyCode::Char('d')));
    assert!(app.flags.pending_delete());
}

#[test]
fn input_tree_c_clone() {
    let mut app = make_app_with_tree();
    app.cursor = 1;
    input::handle_key(&mut app, key(KeyCode::Char('c')));
    assert_eq!(app.files[0].data.items.len(), 4);
}

#[test]
fn input_tree_bang_importance() {
    let mut app = make_app_with_tree();
    app.cursor = 1;
    assert!(!app.files[0].data.items[0].important);
    input::handle_key(&mut app, key(KeyCode::Char('!')));
    assert!(app.files[0].data.items[0].important);
}

#[test]
fn input_tree_s_sort() {
    let mut app = make_app_with_tree();
    app.cursor = 1; // Branch 1
    input::handle_key(&mut app, key(KeyCode::Char('S')));
    assert!(!app.files[0].data.items[0].items.is_empty());
}

#[test]
fn input_tree_q_quits() {
    let mut app = make_app_with_tree();
    input::handle_key(&mut app, key(KeyCode::Char('q')));
    assert!(app.flags.should_quit());
}

// ── tree-mode move (Shift+Arrow, HJKL) ─────────────────────────

#[test]
fn input_tree_shift_up_swaps() {
    let mut app = make_app_with_tree();
    app.cursor = 4; // Branch 2
    input::handle_key(&mut app, shift_key(KeyCode::Up));
    assert_eq!(app.files[0].data.items[0].title, "Branch 2");
}

#[test]
fn input_tree_shift_down_swaps() {
    let mut app = make_app_with_tree();
    app.cursor = 1; // Branch 1
    input::handle_key(&mut app, shift_key(KeyCode::Down));
    assert_eq!(app.files[0].data.items[1].title, "Branch 1");
}

#[test]
fn input_tree_shift_left_promotes() {
    let mut app = make_app_with_tree();
    app.cursor = 2; // Child 1.1
    input::handle_key(&mut app, shift_key(KeyCode::Left));
    assert!(app.files[0].data.items.iter().any(|i| i.title == "Child 1.1"));
}

#[test]
fn input_tree_shift_right_demotes() {
    let mut app = make_app_with_tree();
    app.cursor = 4;
    input::handle_key(&mut app, shift_key(KeyCode::Right));
    assert!(app.files[0].data.items[0].items.iter().any(|i| i.title == "Branch 2"));
}

#[test]
fn input_tree_k_swaps_up() {
    let mut app = make_app_with_tree();
    app.cursor = 4; // Branch 2
    input::handle_key(&mut app, key(KeyCode::Char('K')));
    assert_eq!(app.files[0].data.items[0].title, "Branch 2");
}

#[test]
fn input_tree_j_swaps_down() {
    let mut app = make_app_with_tree();
    app.cursor = 1; // Branch 1
    input::handle_key(&mut app, key(KeyCode::Char('J')));
    assert_eq!(app.files[0].data.items[1].title, "Branch 1");
}

#[test]
fn input_tree_h_promotes() {
    let mut app = make_app_with_tree();
    app.cursor = 2; // Child 1.1
    input::handle_key(&mut app, key(KeyCode::Char('H')));
    assert!(app.files[0].data.items.iter().any(|i| i.title == "Child 1.1"));
}

#[test]
fn input_tree_l_demotes() {
    let mut app = make_app_with_tree();
    app.cursor = 4; // Branch 2
    input::handle_key(&mut app, key(KeyCode::Char('L')));
    assert!(app.files[0].data.items[0].items.iter().any(|i| i.title == "Branch 2"));
}

// ── tree-mode switches ──────────────────────────────────────────

#[test]
fn input_tree_tab_to_note() {
    let mut app = make_app_with_tree();
    app.cursor = 1;
    input::handle_key(&mut app, key(KeyCode::Tab));
    assert!(app.is_note_focused());
}

#[test]
fn input_tree_colon_to_command() {
    let mut app = make_app_with_tree();
    input::handle_key(&mut app, key(KeyCode::Char(':')));
    assert!(app.is_command_active());
}

#[test]
fn input_tree_slash_to_filter() {
    let mut app = make_app_with_tree();
    input::handle_key(&mut app, key(KeyCode::Char('/')));
    assert!(app.is_filter_active());
}

#[test]
fn input_tree_f1_help() {
    let mut app = make_app_with_tree();
    input::handle_key(&mut app, key(KeyCode::F(1)));
    assert!(app.is_help_shown());
}
