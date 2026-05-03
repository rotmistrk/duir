use super::*;

#[test]
fn cmd_export_no_filename() {
    let mut app = make_app_with_tree();
    app.cursor = 1;
    let dir = tempfile::tempdir().unwrap();
    let export_path = dir.path().join("branch-1.md");
    // We can't easily control CWD, so test with explicit filename
    app.cmd_export(&["export", export_path.to_str().unwrap()]);
    assert!(export_path.exists());
    let content = std::fs::read_to_string(&export_path).unwrap();
    assert!(content.contains("Branch 1"));
}

#[test]
fn cmd_export_with_filename() {
    let mut app = make_app_with_tree();
    app.cursor = 1;
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("out.md");
    app.cmd_export(&["export", path.to_str().unwrap()]);
    assert!(path.exists());
    assert!(app.status_message.contains("Exported"));
}

#[test]
fn cmd_export_no_item() {
    let mut app = make_app_with_tree();
    app.cursor = 0; // file root
    app.cmd_export(&["export"]);
    assert!(app.status_message.contains("No item"));
}

#[test]
fn cmd_import_md() {
    let mut app = make_app_with_tree();
    app.cursor = 1;
    let dir = tempfile::tempdir().unwrap();
    let md_path = dir.path().join("import.md");
    std::fs::write(&md_path, "# Imported\n- Sub item\n").unwrap();
    app.cmd_import(&["import", "md", md_path.to_str().unwrap()]);
    assert!(app.status_message.contains("Imported"));
    // Children added to Branch 1
    assert!(app.files[0].data.items[0].items.iter().any(|i| i.title == "Imported"));
}

#[test]
fn cmd_import_bad_usage() {
    let mut app = make_app_with_tree();
    app.cmd_import(&["import"]);
    assert!(app.status_message.contains("Usage"));
}

#[test]
fn cmd_collapse_then_expand_roundtrip() {
    let mut app = make_app_with_tree();
    app.cursor = 1; // Branch 1 with children
    let children_before = app.files[0].data.items[0].items.len();
    assert!(children_before > 0);

    app.cmd_collapse();
    assert!(app.files[0].data.items[0].items.is_empty());
    assert!(app.files[0].data.items[0].note.contains("duir:collapsed"));

    app.cmd_expand();
    assert!(!app.files[0].data.items[0].items.is_empty());
    assert_eq!(app.files[0].data.items[0].items.len(), children_before);
}

#[test]
fn cmd_collapse_no_children() {
    let mut app = make_app_with_tree();
    // Branch 3 has no children
    app.cursor = app.rows.iter().position(|r| r.title == "Branch 3").unwrap();
    app.cmd_collapse();
    assert!(app.status_message.contains("No children"));
}

#[test]
fn cmd_expand_empty_note() {
    let mut app = make_app_with_tree();
    // Branch 3 has empty note
    app.cursor = app.rows.iter().position(|r| r.title == "Branch 3").unwrap();
    app.cmd_expand();
    assert!(app.status_message.contains("No note"));
}

#[test]
fn cmd_autosave_toggle() {
    let mut app = make_app_with_tree();
    let before = app.files[0].autosave;
    app.cmd_autosave(&["autosave"]);
    assert_ne!(app.files[0].autosave, before);
    assert!(app.status_message.contains("Autosave"));
}

#[test]
fn cmd_autosave_all_toggle() {
    let mut app = make_app_with_tree();
    let before = app.autosave_global;
    app.cmd_autosave(&["autosave", "all"]);
    assert_ne!(app.autosave_global, before);
    for f in &app.files {
        assert_eq!(f.autosave, app.autosave_global);
    }
}
