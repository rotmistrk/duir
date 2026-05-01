//! Integration tests for tree operations — verifying data integrity after mutations.

#![allow(clippy::unwrap_used, clippy::assigning_clones)]

use duir_core::model::{Completion, TodoFile, TodoItem};
use duir_core::tree_ops::*;

fn make_tree() -> TodoFile {
    let mut file = TodoFile::new("test");

    let mut branch1 = TodoItem::new("Branch 1");
    "branch1 note".clone_into(&mut branch1.note);
    let mut child11 = TodoItem::new("Child 1.1");
    "child11 note".clone_into(&mut child11.note);
    child11.completed = Completion::Done;
    let mut child12 = TodoItem::new("Child 1.2");
    "child12 note".clone_into(&mut child12.note);
    child12.important = true;
    branch1.items.push(child11);
    branch1.items.push(child12);

    let mut branch2 = TodoItem::new("Branch 2");
    "branch2 note".clone_into(&mut branch2.note);
    let mut child21 = TodoItem::new("Child 2.1");
    "child21 note".clone_into(&mut child21.note);
    branch2.items.push(child21);

    let branch3 = TodoItem::new("Branch 3");

    file.items.push(branch1);
    file.items.push(branch2);
    file.items.push(branch3);
    file
}

#[test]
fn clone_preserves_original() {
    let mut file = make_tree();
    clone_subtree(&mut file, &vec![0]).unwrap();

    // Original at [0] unchanged
    assert_eq!(file.items[0].title, "Branch 1");
    assert_eq!(file.items[0].note, "branch1 note");
    assert_eq!(file.items[0].items.len(), 2);
    assert_eq!(file.items[0].items[0].title, "Child 1.1");
    assert_eq!(file.items[0].items[0].note, "child11 note");
    assert_eq!(file.items[0].items[0].completed, Completion::Done);
    assert_eq!(file.items[0].items[1].title, "Child 1.2");
    assert!(file.items[0].items[1].important);

    // Clone at [1] is identical
    assert_eq!(file.items[1].title, "Branch 1");
    assert_eq!(file.items[1].note, "branch1 note");
    assert_eq!(file.items[1].items.len(), 2);
    assert_eq!(file.items[1].items[0].title, "Child 1.1");
    assert_eq!(file.items[1].items[0].completed, Completion::Done);
    assert_eq!(file.items[1].items[1].title, "Child 1.2");
    assert!(file.items[1].items[1].important);

    // Former branch2 shifted to [2]
    assert_eq!(file.items[2].title, "Branch 2");
    assert_eq!(file.items[2].note, "branch2 note");
    assert_eq!(file.items[2].items[0].title, "Child 2.1");

    // Former branch3 shifted to [3]
    assert_eq!(file.items[3].title, "Branch 3");

    assert_eq!(file.items.len(), 4);
}

#[test]
fn clone_deep_child_preserves_siblings() {
    let mut file = make_tree();
    clone_subtree(&mut file, &vec![0, 0]).unwrap();

    assert_eq!(file.items[0].items.len(), 3);
    assert_eq!(file.items[0].items[0].title, "Child 1.1");
    assert_eq!(file.items[0].items[0].note, "child11 note");
    assert_eq!(file.items[0].items[1].title, "Child 1.1"); // clone
    assert_eq!(file.items[0].items[1].note, "child11 note");
    assert_eq!(file.items[0].items[2].title, "Child 1.2"); // shifted
    assert!(file.items[0].items[2].important);

    // Branch 2 unaffected
    assert_eq!(file.items[1].title, "Branch 2");
    assert_eq!(file.items[1].items[0].note, "child21 note");
}

#[test]
fn add_sibling_preserves_data() {
    let mut file = make_tree();
    let new_item = TodoItem::new("New Sibling");
    add_sibling(&mut file, &vec![1], new_item).unwrap();

    assert_eq!(file.items[0].title, "Branch 1");
    assert_eq!(file.items[1].title, "Branch 2");
    assert_eq!(file.items[2].title, "New Sibling");
    assert_eq!(file.items[3].title, "Branch 3");
    assert_eq!(file.items[1].items[0].title, "Child 2.1");
}

#[test]
fn delete_preserves_remaining() {
    let mut file = make_tree();
    let removed = remove_item(&mut file, &vec![1]).unwrap();

    assert_eq!(removed.title, "Branch 2");
    assert_eq!(file.items.len(), 2);
    assert_eq!(file.items[0].title, "Branch 1");
    assert_eq!(file.items[0].items[0].note, "child11 note");
    assert_eq!(file.items[1].title, "Branch 3");
}

#[test]
fn swap_up_preserves_content() {
    let mut file = make_tree();
    let new_path = swap_up(&mut file, &vec![1]).unwrap();

    assert_eq!(new_path, vec![0]);
    assert_eq!(file.items[0].title, "Branch 2");
    assert_eq!(file.items[0].note, "branch2 note");
    assert_eq!(file.items[0].items[0].title, "Child 2.1");
    assert_eq!(file.items[1].title, "Branch 1");
    assert_eq!(file.items[1].items.len(), 2);
}

#[test]
fn promote_preserves_content() {
    let mut file = make_tree();
    let new_path = promote(&mut file, &vec![0, 1]).unwrap();

    assert_eq!(new_path, vec![1]);
    assert_eq!(file.items[0].title, "Branch 1");
    assert_eq!(file.items[0].items.len(), 1);
    assert_eq!(file.items[0].items[0].title, "Child 1.1");
    assert_eq!(file.items[1].title, "Child 1.2");
    assert!(file.items[1].important);
    assert_eq!(file.items[1].note, "child12 note");
    assert_eq!(file.items[2].title, "Branch 2");
}

#[test]
fn demote_preserves_content() {
    let mut file = make_tree();
    let new_path = demote(&mut file, &vec![1]).unwrap();

    assert_eq!(new_path, vec![0, 2]);
    assert_eq!(file.items[0].title, "Branch 1");
    assert_eq!(file.items[0].items.len(), 3);
    assert_eq!(file.items[0].items[2].title, "Branch 2");
    assert_eq!(file.items[0].items[2].note, "branch2 note");
    assert_eq!(file.items[0].items[2].items[0].title, "Child 2.1");
    assert_eq!(file.items[1].title, "Branch 3");
}

#[test]
fn sort_preserves_content() {
    let mut file = make_tree();
    sort_children(&mut file, &vec![0]).unwrap();

    assert_eq!(file.items[0].items[0].title, "Child 1.1");
    assert_eq!(file.items[0].items[0].completed, Completion::Done);
    assert_eq!(file.items[0].items[1].title, "Child 1.2");
    assert!(file.items[0].items[1].important);
}

#[test]
fn encrypt_decrypt_roundtrip() {
    let mut file = make_tree();
    let item = &mut file.items[0];

    duir_core::crypto::encrypt_item(item, "secret123").unwrap();

    assert!(item.cipher.is_some());
    assert!(item.items.is_empty());
    assert!(item.note.is_empty());
    assert!(!item.unlocked);

    duir_core::crypto::decrypt_item(item, "secret123").unwrap();

    assert!(item.unlocked);
    assert_eq!(item.items.len(), 2);
    assert_eq!(item.items[0].title, "Child 1.1");
    assert_eq!(item.items[0].note, "child11 note");
    assert_eq!(item.items[0].completed, Completion::Done);
    assert_eq!(item.items[1].title, "Child 1.2");
    assert!(item.items[1].important);
    assert_eq!(item.note, "branch1 note");
}

#[test]
fn encrypt_wrong_password_fails() {
    let mut file = make_tree();
    let item = &mut file.items[0];

    duir_core::crypto::encrypt_item(item, "correct").unwrap();
    let result = duir_core::crypto::decrypt_item(item, "wrong");

    assert!(result.is_err());
    assert!(item.cipher.is_some());
    assert!(item.items.is_empty());
}

#[test]
fn collapse_expand_roundtrip() {
    let mut file = make_tree();
    let item = &mut file.items[0];

    let original_note = item.note.clone();
    let original_child_count = item.items.len();
    let original_titles: Vec<String> = item.items.iter().map(|i| i.title.clone()).collect();

    // Collapse: children -> markdown in note
    let mut md = String::new();
    md.push_str("<!-- duir:collapsed -->\n");
    for child in &item.items {
        md.push_str(&duir_core::markdown_export::export_subtree(child, 3));
    }
    if item.note.is_empty() {
        item.note = md;
    } else {
        item.note.push_str("\n\n");
        item.note.push_str(&md);
    }
    item.items.clear();

    assert!(item.items.is_empty());
    assert!(item.note.contains("<!-- duir:collapsed -->"));

    // Expand: markdown in note -> children
    let marker = "<!-- duir:collapsed -->";
    let pos = item.note.find(marker).unwrap();
    let md_part = item.note[pos + marker.len()..].to_owned();
    item.note = item.note[..pos].trim_end().to_owned();

    let parsed = duir_core::markdown_import::import_markdown(&md_part);
    item.items = parsed.items;

    assert_eq!(item.items.len(), original_child_count);
    for (i, title) in original_titles.iter().enumerate() {
        assert_eq!(item.items[i].title, *title);
    }
    assert_eq!(item.note, original_note);
}

#[test]
fn get_item_after_mutations() {
    let mut file = make_tree();

    assert_eq!(get_item(&file, &vec![0]).unwrap().title, "Branch 1");
    assert_eq!(get_item(&file, &vec![1]).unwrap().title, "Branch 2");
    assert_eq!(get_item(&file, &vec![0, 0]).unwrap().title, "Child 1.1");
    assert_eq!(get_item(&file, &vec![0, 1]).unwrap().title, "Child 1.2");
    assert_eq!(get_item(&file, &vec![1, 0]).unwrap().title, "Child 2.1");

    clone_subtree(&mut file, &vec![0]).unwrap();

    assert_eq!(get_item(&file, &vec![0]).unwrap().title, "Branch 1");
    assert_eq!(get_item(&file, &vec![1]).unwrap().title, "Branch 1"); // clone
    assert_eq!(get_item(&file, &vec![2]).unwrap().title, "Branch 2"); // shifted
    assert_eq!(get_item(&file, &vec![2, 0]).unwrap().title, "Child 2.1");
    assert_eq!(get_item(&file, &vec![3]).unwrap().title, "Branch 3");

    remove_item(&mut file, &vec![1]).unwrap();

    assert_eq!(get_item(&file, &vec![0]).unwrap().title, "Branch 1");
    assert_eq!(get_item(&file, &vec![1]).unwrap().title, "Branch 2");
    assert_eq!(get_item(&file, &vec![1, 0]).unwrap().title, "Child 2.1");
}

#[test]
fn filter_does_not_mutate() {
    let file = make_tree();
    let opts = duir_core::filter::FilterOptions {
        search_notes: true,
        case_sensitive: false,
    };

    let matches = duir_core::filter::filter_items(&file.items, "child", &opts);
    assert!(!matches.is_empty());

    assert_eq!(file.items[0].title, "Branch 1");
    assert_eq!(file.items[0].items[0].title, "Child 1.1");
    assert_eq!(file.items[1].items[0].note, "child21 note");
}
