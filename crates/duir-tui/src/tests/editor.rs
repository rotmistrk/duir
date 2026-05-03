use super::*;

#[test]
fn tab_into_note_loads_editor() {
    let mut app = make_app_with_tree();
    app.cursor = 1;
    app.focus_note();
    assert!(app.is_note_focused());
    let FocusState::Note { ref editor, .. } = app.state else {
        unreachable!();
    };
    assert_eq!(editor.content(), "branch1 note");
}

#[test]
fn tab_back_saves_editor_to_model() {
    let mut app = make_app_with_tree();
    app.cursor = 1;
    app.focus_note();
    if let FocusState::Note { ref mut editor, .. } = app.state {
        editor.textarea.insert_str("MODIFIED");
        editor.dirty = true;
    }
    app.save_editor();
    assert!(app.files[0].data.items[0].note.contains("MODIFIED"));
}

#[test]
fn editor_not_written_without_save() {
    let mut app = make_app_with_tree();
    app.cursor = 1;
    app.focus_note();
    if let FocusState::Note { ref mut editor, .. } = app.state {
        editor.textarea.insert_str("SHOULD NOT PERSIST");
    }
    assert_eq!(app.files[0].data.items[0].note, "branch1 note");
}

#[test]
fn cursor_move_does_not_affect_model() {
    let mut app = make_app_with_tree();
    let original = app.files[0].data.items[0].note.clone();
    app.move_down();
    app.move_down();
    app.move_up();
    assert_eq!(app.files[0].data.items[0].note, original);
}

/// THE BUG: edit note, tab back, navigate, add items → note content lost.
/// This tests the exact real-world scenario.
#[test]
fn edit_note_tab_back_navigate_add_items_preserves_note() {
    let mut app = make_app_with_tree();
    app.cursor = 1; // Branch 1

    // Tab into note, edit
    app.focus_note();
    if let FocusState::Note { ref mut editor, .. } = app.state {
        editor.textarea.insert_str("EDITED TEXT ");
    }

    // Tab back to tree
    app.save_editor();
    app.focus_tree();
    assert!(app.files[0].data.items[0].note.contains("EDITED TEXT"));

    // Navigate to different items
    app.move_down(); // Child 1.1
    app.move_down(); // Child 1.2

    // current_note should show Child 1.2's note, not the edited one
    assert_eq!(app.current_note(), "child12 note");

    // Add new items
    app.new_sibling();
    app.cancel_editing();

    // Original edit should still be in the model
    assert!(
        app.files[0].data.items[0].note.contains("EDITED TEXT"),
        "Note was lost! Got: {}",
        app.files[0].data.items[0].note
    );

    // All other notes should be intact
    assert_eq!(app.files[0].data.items[0].items[0].note, "child11 note");
}

/// Verify `current_note` reads from model based on cursor, not from editor.
#[test]
fn current_note_reads_model_not_editor() {
    let mut app = make_app_with_tree();

    // Without loading editor, current_note should work from model
    app.cursor = 1; // Branch 1
    assert_eq!(app.current_note(), "branch1 note");

    app.cursor = 2; // Child 1.1
    assert_eq!(app.current_note(), "child11 note");

    app.cursor = 3; // Child 1.2
    assert_eq!(app.current_note(), "child12 note");
}

#[test]
fn clone_then_navigate_correct_items() {
    let mut app = make_app_with_tree();
    app.cursor = 1;
    app.clone_subtree();
    assert_eq!(app.files[0].data.items[0].title, "Branch 1");
    assert_eq!(app.files[0].data.items[1].title, "Branch 1");
    assert_eq!(app.files[0].data.items[2].title, "Branch 2");
    assert_eq!(app.files[0].data.items[3].title, "Branch 3");
}
