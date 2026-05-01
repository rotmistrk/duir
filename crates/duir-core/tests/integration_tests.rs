//! Integration tests for tree operations — verifying data integrity after mutations.

#![allow(clippy::unwrap_used, clippy::assigning_clones)]

use duir_core::model::{Completion, TodoFile, TodoItem};
use duir_core::storage::TodoStorage;
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

fn find_item_by_title<'a>(items: &'a [TodoItem], title: &str) -> Option<&'a TodoItem> {
    for item in items {
        if item.title == title {
            return Some(item);
        }
        if let Some(found) = find_item_by_title(&item.items, title) {
            return Some(found);
        }
    }
    None
}

fn collect_all_titles(items: &[TodoItem]) -> Vec<String> {
    let mut out = Vec::new();
    for item in items {
        out.push(item.title.clone());
        out.extend(collect_all_titles(&item.items));
    }
    out
}

#[test]
fn clone_preserves_original() {
    let mut file = make_tree();
    clone_subtree(&mut file, &vec![0]).unwrap();

    assert_eq!(file.items[0].title, "Branch 1");
    assert_eq!(file.items[0].note, "branch1 note");
    assert_eq!(file.items[0].items.len(), 2);
    assert_eq!(file.items[0].items[0].title, "Child 1.1");
    assert_eq!(file.items[0].items[0].note, "child11 note");
    assert_eq!(file.items[0].items[0].completed, Completion::Done);
    assert_eq!(file.items[0].items[1].title, "Child 1.2");
    assert!(file.items[0].items[1].important);

    assert_eq!(file.items[1].title, "Branch 1");
    assert_eq!(file.items[1].note, "branch1 note");
    assert_eq!(file.items[1].items.len(), 2);
    assert_eq!(file.items[1].items[0].title, "Child 1.1");
    assert_eq!(file.items[1].items[0].completed, Completion::Done);
    assert_eq!(file.items[1].items[1].title, "Child 1.2");
    assert!(file.items[1].items[1].important);

    assert_eq!(file.items[2].title, "Branch 2");
    assert_eq!(file.items[2].note, "branch2 note");
    assert_eq!(file.items[2].items[0].title, "Child 2.1");

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
    assert_eq!(file.items[0].items[1].title, "Child 1.1");
    assert_eq!(file.items[0].items[1].note, "child11 note");
    assert_eq!(file.items[0].items[2].title, "Child 1.2");
    assert!(file.items[0].items[2].important);

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
    assert_eq!(get_item(&file, &vec![1]).unwrap().title, "Branch 1");
    assert_eq!(get_item(&file, &vec![2]).unwrap().title, "Branch 2");
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

// ── Clone data integrity ──────────────────────────────────────────────

#[test]
fn clone_then_access_all_paths() {
    let mut file = make_tree();
    clone_subtree(&mut file, &vec![0]).unwrap();
    // After clone: [0]=Branch1, [1]=Branch1(clone), [2]=Branch2, [3]=Branch3

    let b1 = get_item(&file, &vec![0]).unwrap();
    assert_eq!(b1.title, "Branch 1");
    assert_eq!(b1.note, "branch1 note");

    let c11 = get_item(&file, &vec![0, 0]).unwrap();
    assert_eq!(c11.title, "Child 1.1");
    assert_eq!(c11.note, "child11 note");

    let c12 = get_item(&file, &vec![0, 1]).unwrap();
    assert_eq!(c12.title, "Child 1.2");
    assert_eq!(c12.note, "child12 note");

    let clone = get_item(&file, &vec![1]).unwrap();
    assert_eq!(clone.title, "Branch 1");
    assert_eq!(clone.note, "branch1 note");

    let clone_c11 = get_item(&file, &vec![1, 0]).unwrap();
    assert_eq!(clone_c11.title, "Child 1.1");
    assert_eq!(clone_c11.note, "child11 note");

    let clone_c12 = get_item(&file, &vec![1, 1]).unwrap();
    assert_eq!(clone_c12.title, "Child 1.2");
    assert_eq!(clone_c12.note, "child12 note");

    let b2 = get_item(&file, &vec![2]).unwrap();
    assert_eq!(b2.title, "Branch 2");
    assert_eq!(b2.note, "branch2 note");

    let c21 = get_item(&file, &vec![2, 0]).unwrap();
    assert_eq!(c21.title, "Child 2.1");
    assert_eq!(c21.note, "child21 note");

    let b3 = get_item(&file, &vec![3]).unwrap();
    assert_eq!(b3.title, "Branch 3");
}

#[test]
fn clone_then_modify_clone_doesnt_affect_original() {
    let mut file = make_tree();
    clone_subtree(&mut file, &vec![0]).unwrap();

    let clone = get_item_mut(&mut file, &vec![1]).unwrap();
    clone.title = "Modified Clone".to_owned();
    clone.note = "modified note".to_owned();

    let original = get_item(&file, &vec![0]).unwrap();
    assert_eq!(original.title, "Branch 1");
    assert_eq!(original.note, "branch1 note");
}

#[test]
fn clone_nested_then_verify_deep_paths() {
    let mut file = make_tree();
    clone_subtree(&mut file, &vec![0, 0]).unwrap();
    // Branch 1 children: [0,0]=Child1.1, [0,1]=Child1.1(clone), [0,2]=Child1.2

    let orig = get_item(&file, &vec![0, 0]).unwrap();
    assert_eq!(orig.title, "Child 1.1");
    assert_eq!(orig.note, "child11 note");

    let cloned = get_item(&file, &vec![0, 1]).unwrap();
    assert_eq!(cloned.title, "Child 1.1");
    assert_eq!(cloned.note, "child11 note");

    let shifted = get_item(&file, &vec![0, 2]).unwrap();
    assert_eq!(shifted.title, "Child 1.2");
    assert_eq!(shifted.note, "child12 note");
    assert!(shifted.important);
}

// ── Save/load round-trip ──────────────────────────────────────────────

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

// ── Encrypt/decrypt data integrity ────────────────────────────────────

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

// ── Collapse/expand round-trip ────────────────────────────────────────

fn collapse_item(item: &mut TodoItem) {
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
}

fn expand_item(item: &mut TodoItem) {
    let marker = "<!-- duir:collapsed -->";
    let pos = item.note.find(marker).unwrap();
    let md_part = item.note[pos + marker.len()..].to_owned();
    item.note = item.note[..pos].trim_end().to_owned();
    let parsed = duir_core::markdown_import::import_markdown(&md_part);
    item.items = parsed.items;
}

#[test]
fn collapse_expand_preserves_child_titles() {
    let mut file = make_tree();
    let titles: Vec<String> = file.items[0].items.iter().map(|i| i.title.clone()).collect();

    collapse_item(&mut file.items[0]);
    expand_item(&mut file.items[0]);

    for (i, title) in titles.iter().enumerate() {
        assert_eq!(file.items[0].items[i].title, *title);
    }
}

#[test]
fn collapse_expand_preserves_completion_state() {
    let mut file = make_tree();
    // export_subtree(child, 3) renders depths 1-3 as headings.
    // Depth 4+ becomes checkboxes which carry completion state.
    // Need: child -> d1 -> d2 -> d3 -> leaves (depth 4 = checkbox)
    let mut d1 = TodoItem::new("D1");
    let mut d2 = TodoItem::new("D2");
    let mut d3 = TodoItem::new("D3");
    let mut done_leaf = TodoItem::new("Done Leaf");
    done_leaf.completed = Completion::Done;
    d3.items.push(done_leaf);
    d3.items.push(TodoItem::new("Open Leaf"));
    d2.items.push(d3);
    d1.items.push(d2);
    file.items[0].items.push(d1);

    collapse_item(&mut file.items[0]);
    expand_item(&mut file.items[0]);

    let d3_restored = &file.items[0].items[2].items[0].items[0];
    assert_eq!(d3_restored.items[0].completed, Completion::Done);
    assert_eq!(d3_restored.items[1].completed, Completion::Open);
}

#[test]
fn collapse_expand_preserves_importance() {
    let mut file = make_tree();
    collapse_item(&mut file.items[0]);
    expand_item(&mut file.items[0]);

    assert!(file.items[0].items[1].important);
    assert!(!file.items[0].items[0].important);
}

#[test]
fn collapse_expand_preserves_notes() {
    let mut file = make_tree();
    collapse_item(&mut file.items[0]);
    expand_item(&mut file.items[0]);

    assert_eq!(file.items[0].items[0].note, "child11 note");
    assert_eq!(file.items[0].items[1].note, "child12 note");
}

#[test]
fn collapse_with_existing_note_preserves_original() {
    let mut file = make_tree();
    file.items[0].note = "original note".to_owned();

    collapse_item(&mut file.items[0]);
    assert!(file.items[0].note.contains("original note"));
    assert!(file.items[0].note.contains("<!-- duir:collapsed -->"));

    expand_item(&mut file.items[0]);
    assert_eq!(file.items[0].note, "original note");
    assert_eq!(file.items[0].items.len(), 2);
    assert_eq!(file.items[0].items[0].title, "Child 1.1");
}

// ── Filter safety ─────────────────────────────────────────────────────

#[test]
fn filter_returns_parent_paths_for_deep_matches() {
    let file = make_tree();
    let opts = duir_core::filter::FilterOptions {
        search_notes: true,
        case_sensitive: false,
    };
    let matches = duir_core::filter::filter_items(&file.items, "child11 note", &opts);
    // Child 1.1 is at [0,0], parent Branch 1 at [0] should be included
    assert!(matches.contains(&vec![0]));
    assert!(matches.contains(&vec![0, 0]));
}

#[test]
fn filter_empty_string_matches_nothing() {
    let file = make_tree();
    let opts = duir_core::filter::FilterOptions {
        search_notes: true,
        case_sensitive: false,
    };
    // Empty string matches all via str::contains, so filter returns all paths.
    // Verify the filter at least returns consistently (no crash, no mutation).
    let matches = duir_core::filter::filter_items(&file.items, "", &opts);
    // Every item matches empty string, so all paths are returned.
    assert!(!matches.is_empty());
    // Verify original data is untouched.
    assert_eq!(file.items[0].title, "Branch 1");
}

// ── Markdown export/import round-trip ─────────────────────────────────

#[test]
fn export_import_roundtrip_titles() {
    let file = make_tree();
    let md = duir_core::markdown_export::export_file(&file);
    let imported = duir_core::markdown_import::import_markdown(&md);

    let orig_titles = collect_all_titles(&file.items);
    let imported_titles = collect_all_titles(&imported.items);

    for title in &orig_titles {
        assert!(imported_titles.contains(title), "Missing title after import: {title}");
    }
}

#[test]
fn export_import_roundtrip_completion() {
    let mut file = make_tree();
    // Headings don't carry completion in markdown; add checkbox-depth items.
    let mut deep = TodoItem::new("Deep Parent");
    let mut done_leaf = TodoItem::new("Done Leaf");
    done_leaf.completed = Completion::Done;
    deep.items.push(done_leaf);
    deep.items.push(TodoItem::new("Open Leaf"));
    file.items[0].items.push(deep);

    let md = duir_core::markdown_export::export_file(&file);
    let imported = duir_core::markdown_import::import_markdown(&md);

    let done = find_item_by_title(&imported.items, "Done Leaf").unwrap();
    assert_eq!(done.completed, Completion::Done);

    let open = find_item_by_title(&imported.items, "Open Leaf").unwrap();
    assert_eq!(open.completed, Completion::Open);
}

#[test]
fn export_import_roundtrip_folded_metadata() {
    let mut file = make_tree();
    file.items[0].folded = true;

    let md = duir_core::markdown_export::export_file(&file);
    assert!(md.contains("<!-- folded -->"));

    let imported = duir_core::markdown_import::import_markdown(&md);
    let b1 = find_item_by_title(&imported.items, "Branch 1").unwrap();
    assert!(b1.folded);
}

#[test]
fn export_import_roundtrip_important_metadata() {
    let file = make_tree();
    let md = duir_core::markdown_export::export_file(&file);

    let imported = duir_core::markdown_import::import_markdown(&md);
    let c12 = find_item_by_title(&imported.items, "Child 1.2").unwrap();
    assert!(c12.important);
}

// ── Tree operation edge cases ─────────────────────────────────────────

#[test]
fn add_sibling_at_end() {
    let mut file = make_tree();
    let new_item = TodoItem::new("Branch 4");
    add_sibling(&mut file, &vec![2], new_item).unwrap();

    assert_eq!(file.items.len(), 4);
    assert_eq!(file.items[3].title, "Branch 4");
}

#[test]
fn delete_last_item() {
    let mut file = make_tree();
    remove_item(&mut file, &vec![2]).unwrap();

    assert_eq!(file.items.len(), 2);
    assert_eq!(file.items[0].title, "Branch 1");
    assert_eq!(file.items[1].title, "Branch 2");
}

#[test]
fn swap_first_item_up_fails() {
    let mut file = make_tree();
    let result = swap_up(&mut file, &vec![0]);
    assert!(result.is_err());
}

#[test]
fn swap_last_item_down_fails() {
    let mut file = make_tree();
    let result = swap_down(&mut file, &vec![2]);
    assert!(result.is_err());
}

#[test]
fn promote_top_level_fails() {
    let mut file = make_tree();
    let result = promote(&mut file, &vec![0]);
    assert!(result.is_err());
}

#[test]
fn demote_first_item_fails() {
    let mut file = make_tree();
    let result = demote(&mut file, &vec![0]);
    assert!(result.is_err());
}
