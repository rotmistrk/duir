//! Integration tests for collapse/expand round-trips.

#![allow(clippy::unwrap_used, clippy::assigning_clones, clippy::indexing_slicing)]

mod common;

use duir_core::model::{Completion, TodoItem};

use common::{collapse_item, expand_item, make_tree};

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
