//! Integration tests for item lifecycle (archive/trash).
#![allow(clippy::indexing_slicing)]

use duir_core::lifecycle::Lifecycle;
use duir_core::model::{Completion, TodoFile, TodoItem};

fn make_item(title: &str) -> TodoItem {
    TodoItem::new(title)
}

fn make_done_item(title: &str, hours_ago: u64) -> TodoItem {
    let mut item = TodoItem::new(title);
    item.completed = Completion::Done;
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    item.updated_at = Some(now.saturating_sub(hours_ago * 3600));
    item
}

#[test]
fn archive_moves_done_items_older_than_threshold() {
    let mut main = TodoFile::new("Test");
    main.items.push(make_item("active"));
    main.items.push(make_done_item("old done", 48));
    main.items.push(make_done_item("recent done", 1));

    let mut archive = TodoFile::new("Archive");
    let mut trash = TodoFile::new("Trash");

    let lc = Lifecycle::new(24, 60); // archive after 24h, delete after 60 days
    lc.run(&mut main, &mut archive, &mut trash);

    assert_eq!(main.items.len(), 2, "active + recent done remain");
    assert_eq!(archive.items.len(), 1, "old done moved to archive");
    assert_eq!(archive.items[0].title, "old done");
}

#[test]
fn trash_deletes_archived_items_older_than_threshold() {
    let mut main = TodoFile::new("Test");
    let mut archive = TodoFile::new("Archive");
    let mut trash = TodoFile::new("Trash");

    // Simulate item archived 70 days ago
    let mut old_archived = make_done_item("ancient", 70 * 24);
    old_archived.completed = Completion::Done;
    archive.items.push(old_archived);

    // Recent archived item (5 days)
    archive.items.push(make_done_item("recent archive", 5 * 24));

    let lc = Lifecycle::new(24, 60);
    lc.run(&mut main, &mut archive, &mut trash);

    assert_eq!(archive.items.len(), 1, "recent stays in archive");
    assert_eq!(trash.items.len(), 1, "ancient moved to trash");
    assert_eq!(trash.items[0].title, "ancient");
}

#[test]
fn restore_from_archive_moves_back_to_main() {
    let mut main = TodoFile::new("Test");
    main.items.push(make_item("existing"));

    let mut archive = TodoFile::new("Archive");
    archive.items.push(make_done_item("archived item", 48));

    Lifecycle::restore(&mut archive, &[0], &mut main);

    assert_eq!(archive.items.len(), 0);
    assert_eq!(main.items.len(), 2);
    assert_eq!(main.items[1].title, "archived item");
}

#[test]
fn restore_from_trash_moves_back_to_main() {
    let mut main = TodoFile::new("Test");
    let mut trash = TodoFile::new("Trash");
    trash.items.push(make_done_item("trashed item", 100));

    Lifecycle::restore(&mut trash, &[0], &mut main);

    assert_eq!(trash.items.len(), 0);
    assert_eq!(main.items.len(), 1);
    assert_eq!(main.items[0].title, "trashed item");
}

#[test]
fn disabled_lifecycle_does_nothing() {
    let mut main = TodoFile::new("Test");
    main.items.push(make_done_item("old done", 1000));

    let mut archive = TodoFile::new("Archive");
    let mut trash = TodoFile::new("Trash");

    let lc = Lifecycle::new(0, 0); // 0 = disabled
    lc.run(&mut main, &mut archive, &mut trash);

    assert_eq!(main.items.len(), 1, "nothing moved when disabled");
    assert_eq!(archive.items.len(), 0);
}
