#![allow(clippy::expect_used, clippy::indexing_slicing)]

use crate::markdown_import::*;
use crate::model::Completion;
use pretty_assertions::assert_eq;

#[test]
fn headings_only() {
    let md = "# Project\n## Phase 1\n## Phase 2\n### Sub-phase";
    let file = import_markdown(md);

    assert_eq!(file.title, "Project");
    assert_eq!(file.version, "2.0");
    assert_eq!(file.items.len(), 1);
    assert_eq!(file.items[0].title, "Project");
    assert_eq!(file.items[0].items.len(), 2);
    assert_eq!(file.items[0].items[0].title, "Phase 1");
    assert_eq!(file.items[0].items[1].title, "Phase 2");
    assert_eq!(file.items[0].items[1].items.len(), 1);
    assert_eq!(file.items[0].items[1].items[0].title, "Sub-phase");
}

#[test]
fn checkboxes_only() {
    let md = "- [ ] Open task\n- [x] Done task\n- [-] Partial task";
    let file = import_markdown(md);

    assert_eq!(file.title, "Imported");
    assert_eq!(file.items.len(), 3);
    assert_eq!(file.items[0].completed, Completion::Open);
    assert_eq!(file.items[1].completed, Completion::Done);
    assert_eq!(file.items[2].completed, Completion::Partial);
}

#[test]
fn mixed_headings_and_checkboxes() {
    let md = "# Tasks\n- [ ] Task A\n- [x] Task B\n## Section\n- [ ] Task C";
    let file = import_markdown(md);

    assert_eq!(file.items.len(), 1);
    let root = &file.items[0];
    assert_eq!(root.title, "Tasks");
    // Task A, Task B, then Section heading
    assert_eq!(root.items.len(), 3);
    assert_eq!(root.items[0].title, "Task A");
    assert_eq!(root.items[1].title, "Task B");
    assert_eq!(root.items[2].title, "Section");
    assert_eq!(root.items[2].items[0].title, "Task C");
}

#[test]
fn nested_lists() {
    let md = "- [ ] Parent\n  - [ ] Child 1\n  - [x] Child 2\n    - [ ] Grandchild";
    let file = import_markdown(md);

    assert_eq!(file.items.len(), 1);
    assert_eq!(file.items[0].title, "Parent");
    assert_eq!(file.items[0].items.len(), 2);
    assert_eq!(file.items[0].items[0].title, "Child 1");
    assert_eq!(file.items[0].items[1].title, "Child 2");
    assert_eq!(file.items[0].items[1].items.len(), 1);
    assert_eq!(file.items[0].items[1].items[0].title, "Grandchild");
}

#[test]
fn notes_attached_to_items() {
    let md = "# Heading\nSome note text\nMore notes\n- [ ] Task\nTask note here";
    let file = import_markdown(md);

    assert_eq!(file.items[0].note, "Some note text\nMore notes");
    assert_eq!(file.items[0].items[0].note, "Task note here");
}

#[test]
fn importance_bold() {
    let md = "- **Important item**\n- [ ] **Bold open**\n- [x] **Bold done**";
    let file = import_markdown(md);

    assert!(file.items[0].important);
    assert_eq!(file.items[0].title, "Important item");
    assert!(file.items[1].important);
    assert_eq!(file.items[1].title, "Bold open");
    assert!(file.items[2].important);
    assert_eq!(file.items[2].title, "Bold done");
}

#[test]
fn no_heading_uses_default_title() {
    let md = "- [ ] Just a task";
    let file = import_markdown(md);

    assert_eq!(file.title, "Imported");
}

#[test]
fn empty_input() {
    let file = import_markdown("");

    assert_eq!(file.title, "Imported");
    assert!(file.items.is_empty());
}
