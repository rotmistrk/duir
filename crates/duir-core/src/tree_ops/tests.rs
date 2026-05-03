#![allow(clippy::expect_used, clippy::indexing_slicing)]

use crate::model::TodoFile;
use crate::tree_ops::*;

fn sample_file() -> TodoFile {
    let mut file = TodoFile::new("Test");
    let mut a = TodoItem::new("A");
    a.items.push(TodoItem::new("A1"));
    a.items.push(TodoItem::new("A2"));
    file.items.push(a);
    file.items.push(TodoItem::new("B"));
    file.items.push(TodoItem::new("C"));
    file
}

// -- get_item --

#[test]
fn get_item_top_level() {
    let file = sample_file();
    let item = get_item(&file, &vec![1]).expect("should find B");
    assert_eq!(item.title, "B");
}

#[test]
fn get_item_nested() {
    let file = sample_file();
    let item = get_item(&file, &vec![0, 1]).expect("should find A2");
    assert_eq!(item.title, "A2");
}

#[test]
fn get_item_empty_path() {
    let file = sample_file();
    assert!(get_item(&file, &vec![]).is_none());
}

#[test]
fn get_item_out_of_bounds() {
    let file = sample_file();
    assert!(get_item(&file, &vec![99]).is_none());
}

// -- get_item_mut --

#[test]
fn get_item_mut_modifies() {
    let mut file = sample_file();
    let item = get_item_mut(&mut file, &vec![0, 0]).expect("should find A1");
    item.title = "A1-modified".to_owned();
    assert_eq!(file.items[0].items[0].title, "A1-modified");
}

// -- add_sibling --

#[test]
fn add_sibling_inserts_after() {
    let mut file = sample_file();
    add_sibling(&mut file, &vec![0], TodoItem::new("X")).expect("ok");
    assert_eq!(file.items[1].title, "X");
    assert_eq!(file.items.len(), 4);
}

#[test]
fn add_sibling_nested() {
    let mut file = sample_file();
    add_sibling(&mut file, &vec![0, 0], TodoItem::new("A1.5")).expect("ok");
    assert_eq!(file.items[0].items[1].title, "A1.5");
    assert_eq!(file.items[0].items.len(), 3);
}

#[test]
fn add_sibling_empty_path_errors() {
    let mut file = sample_file();
    assert!(add_sibling(&mut file, &vec![], TodoItem::new("X")).is_err());
}

// -- add_child --

#[test]
fn add_child_appends() {
    let mut file = sample_file();
    add_child(&mut file, &vec![0], TodoItem::new("A3")).expect("ok");
    assert_eq!(file.items[0].items.len(), 3);
    assert_eq!(file.items[0].items[2].title, "A3");
}

// -- remove_item --

#[test]
fn remove_item_returns_removed() {
    let mut file = sample_file();
    let removed = remove_item(&mut file, &vec![1]).expect("ok");
    assert_eq!(removed.title, "B");
    assert_eq!(file.items.len(), 2);
}

#[test]
fn remove_item_nested() {
    let mut file = sample_file();
    let removed = remove_item(&mut file, &vec![0, 0]).expect("ok");
    assert_eq!(removed.title, "A1");
    assert_eq!(file.items[0].items.len(), 1);
}

// -- clone_subtree --

#[test]
fn clone_subtree_duplicates() {
    let mut file = sample_file();
    clone_subtree(&mut file, &vec![0]).expect("ok");
    assert_eq!(file.items.len(), 4);
    assert_eq!(file.items[0].title, file.items[1].title);
    assert_eq!(file.items[1].items.len(), 2);
}

// -- swap_up --

#[test]
fn swap_up_moves_item() {
    let mut file = sample_file();
    let new_path = swap_up(&mut file, &vec![1]).expect("ok");
    assert_eq!(new_path, vec![0]);
    assert_eq!(file.items[0].title, "B");
    assert_eq!(file.items[1].title, "A");
}

#[test]
fn swap_up_first_item_errors() {
    let mut file = sample_file();
    assert!(swap_up(&mut file, &vec![0]).is_err());
}

// -- swap_down --

#[test]
fn swap_down_moves_item() {
    let mut file = sample_file();
    let new_path = swap_down(&mut file, &vec![0]).expect("ok");
    assert_eq!(new_path, vec![1]);
    assert_eq!(file.items[0].title, "B");
    assert_eq!(file.items[1].title, "A");
}

#[test]
fn swap_down_last_item_errors() {
    let mut file = sample_file();
    assert!(swap_down(&mut file, &vec![2]).is_err());
}

// -- promote --

#[test]
fn promote_moves_to_parent_level() {
    let mut file = sample_file();
    let new_path = promote(&mut file, &vec![0, 1]).expect("ok");
    assert_eq!(new_path, vec![1]);
    assert_eq!(file.items[1].title, "A2");
    assert_eq!(file.items[0].items.len(), 1);
    assert_eq!(file.items.len(), 4);
}

#[test]
fn promote_top_level_errors() {
    let mut file = sample_file();
    assert!(promote(&mut file, &vec![0]).is_err());
}

// -- demote --

#[test]
fn demote_makes_child_of_previous() {
    let mut file = sample_file();
    let new_path = demote(&mut file, &vec![1]).expect("ok");
    assert_eq!(new_path, vec![0, 2]);
    assert_eq!(file.items.len(), 2);
    assert_eq!(file.items[0].items.len(), 3);
    assert_eq!(file.items[0].items[2].title, "B");
}

#[test]
fn demote_first_item_errors() {
    let mut file = sample_file();
    assert!(demote(&mut file, &vec![0]).is_err());
}

// -- sort_children --

#[test]
fn sort_children_alphabetical() {
    let mut file = sample_file();
    // A has children A1, A2. Add Z and M to test sorting.
    file.items[0].items.push(TodoItem::new("M"));
    file.items[0].items.push(TodoItem::new("Z"));
    sort_children(&mut file, &vec![0]).expect("ok");
    let titles: Vec<&str> = file.items[0].items.iter().map(|i| i.title.as_str()).collect();
    assert_eq!(titles, vec!["A1", "A2", "M", "Z"]);
}

#[test]
fn sort_children_invalid_path_errors() {
    let mut file = sample_file();
    assert!(sort_children(&mut file, &vec![99]).is_err());
}
