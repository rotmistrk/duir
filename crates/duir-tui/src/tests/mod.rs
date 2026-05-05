use super::*;
use app::{App, FocusState, StatusLevel};
use duir_core::TodoStorage;
use duir_core::model::{Completion, TodoFile, TodoItem};

mod app_behavior;
mod bridge;
mod commands;
mod completer_tests;
mod crypto;
mod editor;
mod input_other;
mod input_tree;
mod kiro;
mod mcp;
mod multi_file;
mod tree_ops;

fn make_app_with_tree() -> App {
    let mut app = App::new();
    let mut file = TodoFile::new("test");
    let mut branch1 = TodoItem::new("Branch 1");
    branch1.note = "branch1 note".to_owned();
    let mut child11 = TodoItem::new("Child 1.1");
    child11.completed = Completion::Done;
    child11.note = "child11 note".to_owned();
    let mut child12 = TodoItem::new("Child 1.2");
    child12.important = true;
    child12.note = "child12 note".to_owned();
    branch1.items.push(child11);
    branch1.items.push(child12);
    let mut branch2 = TodoItem::new("Branch 2");
    branch2.note = "branch2 note".to_owned();
    branch2.items.push(TodoItem::new("Child 2.1"));
    file.items.push(branch1);
    file.items.push(branch2);
    file.items.push(TodoItem::new("Branch 3"));
    app.add_file("test".to_owned(), file);
    app
}

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

fn key(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}
fn shift_key(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::SHIFT)
}

fn make_app_multi_file() -> App {
    let mut app = App::new();
    let mut file_a = TodoFile::new("file-a");
    file_a.items.push(TodoItem::new("A-first"));
    file_a.items.push(TodoItem::new("A-second"));
    file_a.items.push(TodoItem::new("A-third"));
    app.add_file("file-a".to_owned(), file_a);
    let mut file_b = TodoFile::new("file-b");
    file_b.items.push(TodoItem::new("B-first"));
    file_b.items.push(TodoItem::new("B-second"));
    app.add_file("file-b".to_owned(), file_b);
    app
}

fn make_app_with_active_kiron() -> App {
    let mut app = make_app_with_tree();
    // Mark Branch 1 as kiron
    app.cursor = 1;
    app.cmd_kiron(&["kiron"]);
    // Manually insert an ActiveKiron (can't spawn real PTY in tests)
    let fi = app.rows[app.cursor].file_index;
    let node_id = duir_core::tree_ops::get_item(&app.files[fi].data, &app.rows[app.cursor].path)
        .unwrap()
        .id
        .clone();
    let file_id = app.files[fi].id;
    let cwd = std::env::current_dir().unwrap_or_default();
    let pty = crate::pty_tab::PtyTab::spawn("true", &[], 80, 24, &cwd, &[]).unwrap();
    app.active_kirons.insert(
        (file_id, node_id),
        app::ActiveKiron {
            pty,
            response_ready: false,
            had_output: false,
            mcp_snapshot: None,
            mutation_rx: None,
            socket_path: None,
        },
    );
    app
}
