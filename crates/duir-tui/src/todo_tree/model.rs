//! Re-export duir-core model + `tree_ops` for the todo tree view.

use std::fs;
use std::path::Path;

pub use duir_core::model::{Completion, TodoFile, TodoItem};
use duir_core::stats::update_completion;
pub use duir_core::tree_ops;
pub use duir_core::tree_ops::TreePath;

#[must_use]
pub fn get_item<'a>(file: &'a TodoFile, path: &TreePath) -> Option<&'a TodoItem> {
    tree_ops::get_item(file, path)
}

pub fn get_item_mut<'a>(file: &'a mut TodoFile, path: &TreePath) -> Option<&'a mut TodoItem> {
    tree_ops::get_item_mut(file, path)
}

pub fn add_sibling(file: &mut TodoFile, path: &TreePath, item: TodoItem) -> bool {
    tree_ops::add_sibling(file, path, item).is_ok()
}

pub fn add_child(file: &mut TodoFile, path: &TreePath, item: TodoItem) -> bool {
    tree_ops::add_child(file, path, item).is_ok()
}

pub fn remove_item(file: &mut TodoFile, path: &TreePath) -> Option<TodoItem> {
    tree_ops::remove_item(file, path).ok()
}

pub fn swap_up(file: &mut TodoFile, path: &TreePath) -> Option<TreePath> {
    tree_ops::swap_up(file, path).ok()
}

pub fn swap_down(file: &mut TodoFile, path: &TreePath) -> Option<TreePath> {
    tree_ops::swap_down(file, path).ok()
}

pub fn promote(file: &mut TodoFile, path: &TreePath) -> Option<TreePath> {
    tree_ops::promote(file, path).ok()
}

pub fn demote(file: &mut TodoFile, path: &TreePath) -> Option<TreePath> {
    tree_ops::demote(file, path).ok()
}

pub fn propagate_completion(file: &mut TodoFile, _path: &TreePath) {
    for item in &mut file.items {
        update_completion(item);
    }
}

pub fn sort_children(file: &mut TodoFile, path: &TreePath) -> bool {
    tree_ops::sort_children(file, path).is_ok()
}

/// Load a `TodoFile` from path, creating empty if absent.
#[must_use]
pub fn load_todo_file(path: &Path) -> TodoFile {
    match fs::read_to_string(path) {
        Ok(content) if !content.trim().is_empty() => {
            serde_json::from_str(&content).unwrap_or_else(|_| TodoFile::new("Todo"))
        }
        _ => TodoFile::new("Todo"),
    }
}

/// Save a `TodoFile` to path.
#[must_use]
pub fn save_todo_file(path: &Path, file: &TodoFile) -> bool {
    let Ok(content) = serde_json::to_string_pretty(file) else {
        return false;
    };
    fs::write(path, content).is_ok()
}
