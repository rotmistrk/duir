//! Integration tests for tree operations — verifying data integrity after mutations.

#![allow(clippy::unwrap_used, clippy::assigning_clones, clippy::indexing_slicing)]

mod common;

use duir_core::model::Completion;
use duir_core::tree_ops::*;

use common::make_tree;

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
    let new_item = duir_core::model::TodoItem::new("New Sibling");
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

// ── Tree operation edge cases ─────────────────────────────────────────

#[test]
fn add_sibling_at_end() {
    let mut file = make_tree();
    let new_item = duir_core::model::TodoItem::new("Branch 4");
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
