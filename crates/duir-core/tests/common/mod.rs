#![allow(dead_code)]

use duir_core::model::{Completion, TodoFile, TodoItem};

pub fn make_tree() -> TodoFile {
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

pub fn find_item_by_title<'a>(items: &'a [TodoItem], title: &str) -> Option<&'a TodoItem> {
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

pub fn collect_all_titles(items: &[TodoItem]) -> Vec<String> {
    let mut out = Vec::new();
    for item in items {
        out.push(item.title.clone());
        out.extend(collect_all_titles(&item.items));
    }
    out
}

pub fn collapse_item(item: &mut TodoItem) {
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

pub fn expand_item(item: &mut TodoItem) {
    let marker = "<!-- duir:collapsed -->";
    let pos = item.note.find(marker).unwrap();
    let md_part = item.note[pos + marker.len()..].to_owned();
    item.note = item.note[..pos].trim_end().to_owned();
    let parsed = duir_core::markdown_import::import_markdown(&md_part);
    item.items = parsed.items;
}
