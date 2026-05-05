use super::*;

#[test]
fn collapse_updates_editor() {
    let mut app = make_app_with_tree();
    app.cursor = 1;
    app.focus_note();
    app.save_editor();
    app.cmd_collapse();
    let FocusState::Note { ref editor, .. } = app.state else {
        unreachable!();
    };
    assert!(editor.content().contains("duir:collapsed"));
}

#[test]
fn delete_incomplete_requires_confirm() {
    let mut app = make_app_with_tree();
    app.cursor = 1;
    app.delete_current();
    assert!(app.flags.pending_delete());
    assert_eq!(app.files[0].data.items[0].title, "Branch 1");
}

#[test]
fn delete_completed_leaf_immediate() {
    let mut app = make_app_with_tree();
    app.cursor = 2; // Child 1.1 (Done)
    app.delete_current();
    assert!(!app.flags.pending_delete());
    assert_eq!(app.files[0].data.items[0].items[0].title, "Child 1.2");
}

#[test]
fn filter_hides_rows() {
    let mut app = make_app_with_tree();
    let total = app.rows.len();
    app.filter_committed_text = "Child 1.1".to_owned();
    app.apply_filter();
    assert!(app.rows.len() < total);
}

#[test]
fn filter_clear_restores() {
    let mut app = make_app_with_tree();
    let total = app.rows.len();
    app.filter_committed_text = "Child 1.1".to_owned();
    app.apply_filter();
    app.filter_committed_text.clear();
    app.apply_filter();
    assert_eq!(app.rows.len(), total);
}

#[test]
fn new_sibling_starts_editing() {
    let mut app = make_app_with_tree();
    app.cursor = 1;
    app.new_sibling();
    assert!(app.is_editing_title());
    assert_eq!(app.files[0].data.items.len(), 4);
}

#[test]
fn new_child_starts_editing() {
    let mut app = make_app_with_tree();
    app.cursor = 1;
    let old = app.files[0].data.items[0].items.len();
    app.new_child();
    assert!(app.is_editing_title());
    assert_eq!(app.files[0].data.items[0].items.len(), old + 1);
}

#[test]
fn adding_child_to_completed_parent_uncompletes_it() {
    let mut app = make_app_with_tree();
    app.cursor = 3; // Child 1.2
    app.toggle_completed();
    assert_eq!(app.files[0].data.items[0].completed, Completion::Done);

    app.cursor = 1; // Branch 1
    app.new_child();
    app.cancel_editing();

    assert_ne!(app.files[0].data.items[0].completed, Completion::Done);
}

#[test]
fn adding_sibling_updates_parent_completion() {
    let mut app = make_app_with_tree();
    app.cursor = 2; // Child 1.1 (already Done)
    app.cursor = 3; // Child 1.2
    app.toggle_completed();
    assert_eq!(app.files[0].data.items[0].completed, Completion::Done);

    app.cursor = 3;
    app.new_sibling();
    app.cancel_editing();

    assert_ne!(app.files[0].data.items[0].completed, Completion::Done);
}

#[test]
fn deleting_incomplete_child_may_complete_parent() {
    let mut app = make_app_with_tree();
    app.cursor = 3; // Child 1.2
    app.delete_current(); // pending
    app.force_delete_current(); // confirm

    assert_eq!(app.files[0].data.items[0].completed, Completion::Done);
}

#[test]
fn save_preserves_unencrypted_data() {
    let mut app = make_app_with_tree();
    app.mark_modified(0, &[]);
    let dir = tempfile::tempdir().unwrap();
    let storage = duir_core::FileStorage::new(dir.path()).unwrap();
    app.save_all(&storage);
    let loaded = storage.load("test").unwrap();
    assert_eq!(loaded.items[0].title, "Branch 1");
    assert_eq!(loaded.items[0].note, "branch1 note");
    assert_eq!(loaded.items[0].items[0].title, "Child 1.1");
    assert_eq!(loaded.items[1].title, "Branch 2");
    assert_eq!(loaded.items[2].title, "Branch 3");
}
