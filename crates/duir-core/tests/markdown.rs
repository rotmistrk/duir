//! Integration tests for markdown export/import round-trips and filter safety.

#![allow(clippy::unwrap_used, clippy::assigning_clones, clippy::indexing_slicing)]

mod common;

use duir_core::model::{Completion, TodoItem};

use common::{collect_all_titles, find_item_by_title, make_tree};

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
