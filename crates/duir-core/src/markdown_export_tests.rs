#![allow(clippy::expect_used)]

use pretty_assertions::assert_eq;

use super::*;

fn item(title: &str) -> TodoItem {
    TodoItem::new(title)
}

fn done_item(title: &str) -> TodoItem {
    TodoItem {
        completed: Completion::Done,
        ..TodoItem::new(title)
    }
}

fn important_item(title: &str) -> TodoItem {
    TodoItem {
        important: true,
        ..TodoItem::new(title)
    }
}

#[test]
fn single_item_heading() {
    let i = item("Hello");
    let md = export_subtree(&i, 3);
    assert_eq!(md, "# Hello\n");
}

#[test]
fn important_item_bold() {
    let i = important_item("Urgent");
    let md = export_subtree(&i, 3);
    assert_eq!(md, "# **Urgent** <!-- important -->\n");
}

#[test]
fn nested_headings() {
    let mut root = item("Root");
    root.items.push(item("Child"));
    let md = export_subtree(&root, 3);
    assert_eq!(md, "# Root\n\n## Child\n");
}

#[test]
fn heading_to_checkbox_transition() {
    let mut l2 = item("Level 2");
    let mut l3 = item("Level 3");
    l3.items.push(done_item("Level 4"));
    l2.items.push(l3);
    let mut root = item("Level 1");
    root.items.push(l2);

    let md = export_subtree(&root, 3);
    let expected = "\
# Level 1

## Level 2

### Level 3

- [x] Level 4
";
    assert_eq!(md, expected);
}

#[test]
fn nested_checkboxes() {
    let mut l3 = item("L3");
    let mut deep = item("L4");
    deep.items.push(done_item("L5"));
    l3.items.push(deep);
    let mut l2 = item("L2");
    l2.items.push(l3);
    let mut root = item("L1");
    root.items.push(l2);

    let md = export_subtree(&root, 2);
    let expected = "\
# L1

## L2

- [ ] L3
  - [ ] L4
    - [x] L5
";
    assert_eq!(md, expected);
}

#[test]
fn completion_states() {
    let mut root = item("Root");
    root.items.push(item("Open"));
    root.items.push(done_item("Done"));
    root.items.push(TodoItem {
        completed: Completion::Partial,
        ..TodoItem::new("Partial")
    });

    let md = export_subtree(&root, 1);
    let expected = "\
# Root

- [ ] Open
- [x] Done
- [-] Partial
";
    assert_eq!(md, expected);
}

#[test]
fn item_with_note() {
    let mut i = item("Task");
    i.note = "Some details here.".to_owned();
    let md = export_subtree(&i, 3);
    let expected = "\
# Task

Some details here.
";
    assert_eq!(md, expected);
}

#[test]
fn checkbox_with_note() {
    let mut child = done_item("Sub");
    child.note = "Note line.".to_owned();
    let mut root = item("Root");
    root.items.push(child);

    let md = export_subtree(&root, 1);
    let expected = "\
# Root

- [x] Sub

  Note line.

";
    assert_eq!(md, expected);
}

#[test]
fn important_checkbox() {
    let mut root = item("Root");
    root.items.push(important_item("Bold"));
    let md = export_subtree(&root, 1);
    assert!(md.contains("- [ ] **Bold**"));
}

#[test]
fn export_file_basic() {
    let mut file = TodoFile::new("My List");
    file.items.push(item("First"));
    file.items.push(done_item("Second"));

    let md = export_file(&file);
    let expected = "\
# My List

## First

## Second
";
    assert_eq!(md, expected);
}

#[test]
fn export_file_with_note() {
    let mut file = TodoFile::new("Project");
    file.note = "Project description.".to_owned();
    file.items.push(item("Task"));

    let md = export_file(&file);
    assert!(md.starts_with("# Project\n\nProject description.\n"));
    assert!(md.contains("## Task\n"));
}

#[test]
fn export_file_deep_tree() {
    let mut file = TodoFile::new("Deep");
    let mut l1 = item("A");
    let mut l2 = item("B");
    l2.items.push(done_item("C"));
    l1.items.push(l2);
    file.items.push(l1);

    let md = export_file(&file);
    // file title = #, top-level = ##, child = ###, grandchild = checkbox
    assert!(md.contains("## A\n"));
    assert!(md.contains("### B\n"));
    assert!(md.contains("- [x] C\n"));
}
