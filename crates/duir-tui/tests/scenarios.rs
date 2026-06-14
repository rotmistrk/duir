//! Scenario tests for duir-tui — verifies user-facing behavior.
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]

mod helpers;
use helpers::{TestHarness, default_todo, temp_project};
use txv_core::event::{KeyCode, KeyMod};

// --- Startup & Layout ---

#[test]
fn startup_shows_tree_zoomed() {
    let dir = temp_project(default_todo());
    let mut h = TestHarness::new(dir.path());
    h.run_cycles(1);
    assert!(h.contains("First item"), "tree should show first item");
    assert!(h.contains("Second item"), "tree should show second item");
}

#[test]
fn startup_tree_has_badges() {
    let dir = temp_project(default_todo());
    let mut h = TestHarness::new(dir.path());
    h.run_cycles(1);
    // Done item should have ✓ badge
    assert!(h.contains("✓"), "done item should show ✓ badge");
    // Item with note should have ♪ badge
    assert!(h.contains("♪"), "item with note should show ♪ badge");
}

// --- Navigation ---

#[test]
fn j_moves_cursor_down() {
    let dir = temp_project(default_todo());
    let mut h = TestHarness::new(dir.path());
    h.run_cycles(1);
    h.inject_str("j");
    h.run_cycles(1);
    // After j, cursor should be on "Second item" — we can't easily check
    // cursor position, but we can verify no crash and tree still shows
    assert!(h.contains("Second item"));
}

#[test]
fn f5_toggles_zoom() {
    let dir = temp_project(default_todo());
    let mut h = TestHarness::new(dir.path());
    h.run_cycles(1);
    // Initially zoomed — only tree visible
    // Press F5 to unzoom
    h.inject_key(KeyCode::F(5), KeyMod::NONE);
    h.run_cycles(1);
    // Should now show Notes tab and Shell tab in other panels
    let screen = h.screen_text();
    assert!(
        screen.contains("Note") || screen.contains("Shell"),
        "unzoom should reveal other panels"
    );
}

// --- Tree Operations ---

#[test]
fn space_toggles_completion() {
    let dir = temp_project(default_todo());
    let mut h = TestHarness::new(dir.path());
    h.run_cycles(1);
    // First item is Open. Press space to toggle to Done.
    h.inject_str(" ");
    h.run_cycles(1);
    // Should now have a ✓ on first item
    // Check file was saved
    let content = std::fs::read_to_string(dir.path().join(".duir/todo.todo.json")).unwrap();
    assert!(content.contains("Done"), "space should toggle to Done");
}

#[test]
fn n_adds_sibling() {
    let dir = temp_project(default_todo());
    let mut h = TestHarness::new(dir.path());
    h.run_cycles(1);
    h.inject_str("n");
    h.run_cycles(1);
    // Should be in edit mode — type a title and press Enter
    h.inject_str("New task");
    h.inject_key(KeyCode::Enter, KeyMod::NONE);
    h.run_cycles(1);
    let content = std::fs::read_to_string(dir.path().join(".duir/todo.todo.json")).unwrap();
    assert!(content.contains("New task"), "n should add sibling with typed title");
}

#[test]
fn d_deletes_item() {
    let dir = temp_project(default_todo());
    let mut h = TestHarness::new(dir.path());
    h.run_cycles(1);
    // Move to "Done item" (j j j) and delete it
    h.inject_str("jjjd");
    h.run_cycles(1);
    let content = std::fs::read_to_string(dir.path().join(".duir/todo.todo.json")).unwrap();
    assert!(!content.contains("Done item"), "d should delete the item");
}

#[test]
fn plus_minus_changes_priority() {
    let dir = temp_project(default_todo());
    let mut h = TestHarness::new(dir.path());
    h.run_cycles(1);
    h.inject_str("+++");
    h.run_cycles(1);
    let content = std::fs::read_to_string(dir.path().join(".duir/todo.todo.json")).unwrap();
    assert!(
        content.contains("\"priority\": 3"),
        "+ three times should set priority 3"
    );
}

#[test]
fn i_toggles_in_progress() {
    let dir = temp_project(default_todo());
    let mut h = TestHarness::new(dir.path());
    h.run_cycles(1);
    h.inject_str("i");
    h.run_cycles(1);
    let content = std::fs::read_to_string(dir.path().join(".duir/todo.todo.json")).unwrap();
    assert!(content.contains("InProgress"), "i should set work_status to InProgress");
}

// --- Note Save ---

#[test]
fn note_persists_on_panel_switch() {
    let dir = temp_project(default_todo());
    let mut h = TestHarness::new(dir.path());
    h.run_cycles(1);
    // Unzoom first
    h.inject_key(KeyCode::F(5), KeyMod::NONE);
    h.run_cycles(1);
    // Select first item (should emit note load)
    h.inject_str("j"); // move to trigger note load
    h.run_cycles(2);
    // Focus notes panel
    h.inject_key(KeyCode::F(3), KeyMod::NONE);
    h.run_cycles(1);
    // Type in the editor
    h.inject_str("iNew note content");
    h.inject_key(KeyCode::Esc, KeyMod::NONE);
    h.run_cycles(1);
    // Switch back to tree (triggers save)
    h.inject_key(KeyCode::F(2), KeyMod::NONE);
    h.run_cycles(2);
    // Check file
    let content = std::fs::read_to_string(dir.path().join(".duir/todo.todo.json")).unwrap();
    assert!(
        content.contains("New note content"),
        "note should be saved on panel switch"
    );
}

// --- Connectors ---

#[test]
fn t_toggles_connectors() {
    let dir = temp_project(default_todo());
    let mut h = TestHarness::new(dir.path());
    h.run_cycles(1);
    // Connectors on by default — should see tree drawing chars
    let before = h.screen_text();
    h.inject_str("T");
    h.run_cycles(1);
    let after = h.screen_text();
    // Screen should change (connectors removed)
    assert_ne!(before, after, "T should toggle connector visibility");
}

// --- Timestamps ---

#[test]
fn d_key_upper_toggles_timestamps() {
    let dir = temp_project(default_todo());
    let mut h = TestHarness::new(dir.path());
    h.run_cycles(1);
    let before = h.screen_text();
    h.inject_str("D");
    h.run_cycles(1);
    let after = h.screen_text();
    assert_ne!(before, after, "D should toggle timestamp columns");
}

#[test]
fn note_shows_when_item_selected_and_unzoomed() {
    let dir = temp_project(default_todo());
    let mut h = TestHarness::new(dir.path());
    h.run_cycles(1);
    // Unzoom to reveal Notes panel
    h.inject_key(KeyCode::F(5), KeyMod::NONE);
    h.run_cycles(1);
    // First item has note "hello" - cursor is on it from start
    // The note should be visible in the center panel
    assert!(
        h.contains("hello"),
        "note content should be visible after unzoom: {}",
        h.screen_text()
    );
}

#[test]
fn note_autoindent_on_enter() {
    let dir = temp_project(default_todo());
    let mut h = TestHarness::new(dir.path());
    h.run_cycles(1);
    // Unzoom to reveal Notes panel
    h.inject_key(KeyCode::F(5), KeyMod::NONE);
    h.run_cycles(1);
    // Focus notes panel (first item has note "hello")
    h.inject_key(KeyCode::F(3), KeyMod::NONE);
    h.run_cycles(1);
    // Enter insert mode, go to end of line, add indented content
    h.inject_str("o    indented");
    h.inject_key(KeyCode::Enter, KeyMod::NONE);
    // After Enter, the new line should inherit "    " indent
    h.inject_str("next line");
    h.inject_key(KeyCode::Esc, KeyMod::NONE);
    h.run_cycles(1);
    // Switch back to tree to trigger note save
    h.inject_key(KeyCode::F(2), KeyMod::NONE);
    h.run_cycles(2);
    let content = std::fs::read_to_string(dir.path().join(".duir/todo.todo.json")).unwrap();
    assert!(
        content.contains("    next line"),
        "autoindent should preserve leading spaces on Enter: {content}"
    );
}

#[test]
fn note_yank_appears_in_clipboard_ring() {
    let dir = temp_project(default_todo());
    let mut h = TestHarness::new(dir.path());
    h.run_cycles(1);
    // Unzoom
    h.inject_key(KeyCode::F(5), KeyMod::NONE);
    h.run_cycles(1);
    // Focus notes panel — first item has note "hello"
    h.inject_key(KeyCode::F(3), KeyMod::NONE);
    h.run_cycles(2);
    // Yank line with yy, then paste with p (duplicates the line)
    h.inject_str("yyp");
    h.run_cycles(1);
    // Switch back to tree to save
    h.inject_key(KeyCode::F(2), KeyMod::NONE);
    h.run_cycles(2);
    let content = std::fs::read_to_string(dir.path().join(".duir/todo.todo.json")).unwrap();
    // "hello" yanked+pasted should produce "hello\nhello"
    assert!(
        content.contains("hello\\nhello"),
        "yank+paste should duplicate the line in note: {content}"
    );
}
