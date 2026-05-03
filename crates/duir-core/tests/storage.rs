//! Integration tests for save/load round-trips.

#![allow(clippy::unwrap_used, clippy::assigning_clones, clippy::indexing_slicing)]

mod common;

use duir_core::model::{Completion, TodoFile, TodoItem};
use duir_core::storage::TodoStorage;
use duir_core::tree_ops::*;

use common::make_tree;

#[test]
fn save_load_roundtrip_preserves_all_data() {
    let mut file = make_tree();
    file.items[0].folded = true;

    let dir = tempfile::tempdir().unwrap();
    let storage = duir_core::file_storage::FileStorage::new(dir.path()).unwrap();
    storage.save("rt", &file).unwrap();
    let loaded = storage.load("rt").unwrap();

    assert_eq!(loaded.items[0].title, "Branch 1");
    assert_eq!(loaded.items[0].note, "branch1 note");
    assert!(loaded.items[0].folded);
    assert_eq!(loaded.items[0].items[0].title, "Child 1.1");
    assert_eq!(loaded.items[0].items[0].note, "child11 note");
    assert_eq!(loaded.items[0].items[0].completed, Completion::Done);
    assert_eq!(loaded.items[0].items[1].title, "Child 1.2");
    assert_eq!(loaded.items[0].items[1].note, "child12 note");
    assert!(loaded.items[0].items[1].important);
    assert_eq!(loaded.items[1].title, "Branch 2");
    assert_eq!(loaded.items[1].note, "branch2 note");
    assert_eq!(loaded.items[1].items[0].title, "Child 2.1");
    assert_eq!(loaded.items[1].items[0].note, "child21 note");
    assert_eq!(loaded.items[2].title, "Branch 3");
}

#[test]
fn save_load_roundtrip_after_clone() {
    let mut file = make_tree();
    clone_subtree(&mut file, &vec![0]).unwrap();

    let dir = tempfile::tempdir().unwrap();
    let storage = duir_core::file_storage::FileStorage::new(dir.path()).unwrap();
    storage.save("rt", &file).unwrap();
    let loaded = storage.load("rt").unwrap();

    assert_eq!(loaded.items.len(), 4);
    assert_eq!(loaded.items[0].title, "Branch 1");
    assert_eq!(loaded.items[1].title, "Branch 1");
    assert_eq!(loaded.items[1].items[0].title, "Child 1.1");
    assert_eq!(loaded.items[2].title, "Branch 2");
    assert_eq!(loaded.items[3].title, "Branch 3");
}

#[test]
fn save_load_roundtrip_after_move_operations() {
    let mut file = make_tree();
    promote(&mut file, &vec![0, 1]).unwrap();
    demote(&mut file, &vec![1]).unwrap();
    swap_up(&mut file, &vec![1]).unwrap();

    let dir = tempfile::tempdir().unwrap();
    let storage = duir_core::file_storage::FileStorage::new(dir.path()).unwrap();
    storage.save("rt", &file).unwrap();
    let loaded = storage.load("rt").unwrap();

    assert_eq!(loaded.items.len(), file.items.len());
    for (i, item) in file.items.iter().enumerate() {
        assert_eq!(loaded.items[i].title, item.title);
        assert_eq!(loaded.items[i].note, item.note);
        assert_eq!(loaded.items[i].items.len(), item.items.len());
    }
}

#[test]
fn save_load_empty_notes() {
    let mut file = TodoFile::new("empty-notes");
    let mut a = TodoItem::new("Has note");
    "some note".clone_into(&mut a.note);
    let b = TodoItem::new("No note");
    file.items.push(a);
    file.items.push(b);

    let dir = tempfile::tempdir().unwrap();
    let storage = duir_core::file_storage::FileStorage::new(dir.path()).unwrap();
    storage.save("rt", &file).unwrap();
    let loaded = storage.load("rt").unwrap();

    assert_eq!(loaded.items[0].note, "some note");
    assert_eq!(loaded.items[1].note, "");
}
