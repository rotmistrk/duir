//! Integration tests for encrypt/decrypt data integrity.

#![allow(clippy::unwrap_used, clippy::assigning_clones, clippy::indexing_slicing)]

mod common;

use duir_core::model::{Completion, TodoItem};
use duir_core::storage::TodoStorage;

use common::make_tree;

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
fn encrypt_preserves_sibling_data() {
    let mut file = make_tree();
    duir_core::crypto::encrypt_item(&mut file.items[0], "pw").unwrap();

    assert_eq!(file.items[1].title, "Branch 2");
    assert_eq!(file.items[1].note, "branch2 note");
    assert_eq!(file.items[1].items.len(), 1);
    assert_eq!(file.items[1].items[0].title, "Child 2.1");
    assert_eq!(file.items[1].items[0].note, "child21 note");
    assert_eq!(file.items[2].title, "Branch 3");
    assert!(file.items[2].items.is_empty());
}

#[test]
fn encrypt_decrypt_nested_children() {
    let mut file = make_tree();
    // Add a grandchild to Child 1.1
    let grandchild = TodoItem::new("Grandchild 1.1.1");
    file.items[0].items[0].items.push(grandchild);

    duir_core::crypto::encrypt_item(&mut file.items[0], "pw").unwrap();
    assert!(file.items[0].items.is_empty());

    duir_core::crypto::decrypt_item(&mut file.items[0], "pw").unwrap();
    assert_eq!(file.items[0].items.len(), 2);
    assert_eq!(file.items[0].items[0].items.len(), 1);
    assert_eq!(file.items[0].items[0].items[0].title, "Grandchild 1.1.1");
}

#[test]
fn encrypt_then_save_load_then_decrypt() {
    let mut file = make_tree();
    duir_core::crypto::encrypt_item(&mut file.items[0], "pw").unwrap();

    let dir = tempfile::tempdir().unwrap();
    let storage = duir_core::file_storage::FileStorage::new(dir.path()).unwrap();
    storage.save("enc", &file).unwrap();
    let mut loaded = storage.load("enc").unwrap();

    assert!(loaded.items[0].cipher.is_some());
    assert!(loaded.items[0].items.is_empty());

    duir_core::crypto::decrypt_item(&mut loaded.items[0], "pw").unwrap();
    assert_eq!(loaded.items[0].items.len(), 2);
    assert_eq!(loaded.items[0].items[0].title, "Child 1.1");
    assert_eq!(loaded.items[0].items[0].completed, Completion::Done);
    assert_eq!(loaded.items[0].items[1].title, "Child 1.2");
    assert!(loaded.items[0].items[1].important);
    assert_eq!(loaded.items[0].note, "branch1 note");
}

#[test]
fn encrypt_wrong_password_preserves_cipher() {
    let mut file = make_tree();
    duir_core::crypto::encrypt_item(&mut file.items[0], "correct").unwrap();
    let cipher_before = file.items[0].cipher.clone();

    let result = duir_core::crypto::decrypt_item(&mut file.items[0], "wrong");
    assert!(result.is_err());
    assert_eq!(file.items[0].cipher, cipher_before);
    assert!(file.items[0].items.is_empty());
}
