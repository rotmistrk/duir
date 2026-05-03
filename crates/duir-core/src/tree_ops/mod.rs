//! Tree operations for navigating and mutating the hierarchical todo structure.

use crate::error::{OmelaError, Result};
use crate::model::{NodeId, TodoFile, TodoItem};

/// A path addressing a `TodoItem` by index at each nesting level.
///
/// For example, `vec![1, 0]` refers to the first child of the second top-level item.
pub type TreePath = Vec<usize>;

/// Navigate to the item at `path`.
///
/// # Errors
///
/// Returns `None` if any index in `path` is out of bounds.
#[must_use]
pub fn get_item<'a>(file: &'a TodoFile, path: &TreePath) -> Option<&'a TodoItem> {
    let mut items = &file.items;
    let (last, parents) = path.split_last()?;
    for &idx in parents {
        items = &items.get(idx)?.items;
    }
    items.get(*last)
}

/// Navigate to the item at `path` mutably.
///
/// # Errors
///
/// Returns `None` if any index in `path` is out of bounds.
#[must_use]
pub fn get_item_mut<'a>(file: &'a mut TodoFile, path: &TreePath) -> Option<&'a mut TodoItem> {
    let mut items = &mut file.items;
    let (last, parents) = path.split_last()?;
    for &idx in parents {
        items = &mut items.get_mut(idx)?.items;
    }
    items.get_mut(*last)
}

/// Resolve `path` to the parent's children vec and the final index.
fn parent_items_and_index<'a>(file: &'a mut TodoFile, path: &TreePath) -> Result<(&'a mut Vec<TodoItem>, usize)> {
    let (last, parents) = path.split_last().ok_or_else(|| OmelaError::InvalidPath(path.clone()))?;
    let mut items = &mut file.items;
    for &idx in parents {
        items = &mut items
            .get_mut(idx)
            .ok_or_else(|| OmelaError::InvalidPath(path.clone()))?
            .items;
    }
    if *last > items.len() {
        return Err(OmelaError::InvalidPath(path.clone()));
    }
    Ok((items, *last))
}

/// Insert `item` as the next sibling after the item at `path`.
///
/// # Errors
///
/// Returns `OmelaError::InvalidPath` if `path` is empty or out of bounds.
pub fn add_sibling(file: &mut TodoFile, path: &TreePath, item: TodoItem) -> Result<()> {
    let (items, idx) = parent_items_and_index(file, path)?;
    if idx >= items.len() {
        return Err(OmelaError::InvalidPath(path.clone()));
    }
    items.insert(idx + 1, item);
    Ok(())
}

/// Append `item` as the last child of the item at `path`.
///
/// # Errors
///
/// Returns `OmelaError::InvalidPath` if `path` is empty or out of bounds.
pub fn add_child(file: &mut TodoFile, path: &TreePath, item: TodoItem) -> Result<()> {
    let parent = get_item_mut(file, path).ok_or_else(|| OmelaError::InvalidPath(path.clone()))?;
    parent.items.push(item);
    Ok(())
}

/// Remove and return the item at `path`.
///
/// # Errors
///
/// Returns `OmelaError::InvalidPath` if `path` is empty or out of bounds.
pub fn remove_item(file: &mut TodoFile, path: &TreePath) -> Result<TodoItem> {
    let (items, idx) = parent_items_and_index(file, path)?;
    if idx >= items.len() {
        return Err(OmelaError::InvalidPath(path.clone()));
    }
    Ok(items.remove(idx))
}

/// Clone the item at `path` and insert the clone as its next sibling.
///
/// # Errors
///
/// Returns `OmelaError::InvalidPath` if `path` is empty or out of bounds.
pub fn clone_subtree(file: &mut TodoFile, path: &TreePath) -> Result<()> {
    let mut cloned = get_item(file, path)
        .ok_or_else(|| OmelaError::InvalidPath(path.clone()))?
        .clone();
    strip_kiron_state(&mut cloned);
    add_sibling(file, path, cloned)
}

/// Recursively strip kiron/session metadata from a subtree (used after clone).
fn strip_kiron_state(item: &mut TodoItem) {
    if item.node_type == Some(crate::NodeType::Kiron) {
        item.node_type = None;
    }
    item.kiron = None;
    for child in &mut item.items {
        strip_kiron_state(child);
    }
}

/// Swap the item at `path` with its previous sibling. Returns the new path.
///
/// # Errors
///
/// Returns `OmelaError::InvalidPath` if `path` is empty, out of bounds, or the item is already first.
pub fn swap_up(file: &mut TodoFile, path: &TreePath) -> Result<TreePath> {
    let (items, idx) = parent_items_and_index(file, path)?;
    if idx == 0 || idx >= items.len() {
        return Err(OmelaError::InvalidPath(path.clone()));
    }
    items.swap(idx, idx - 1);
    let mut new_path = path.clone();
    if let Some(last) = new_path.last_mut() {
        *last = idx - 1;
    }
    Ok(new_path)
}

/// Swap the item at `path` with its next sibling. Returns the new path.
///
/// # Errors
///
/// Returns `OmelaError::InvalidPath` if `path` is empty, out of bounds, or the item is already last.
pub fn swap_down(file: &mut TodoFile, path: &TreePath) -> Result<TreePath> {
    let (items, idx) = parent_items_and_index(file, path)?;
    if idx + 1 >= items.len() {
        return Err(OmelaError::InvalidPath(path.clone()));
    }
    items.swap(idx, idx + 1);
    let mut new_path = path.clone();
    if let Some(last) = new_path.last_mut() {
        *last = idx + 1;
    }
    Ok(new_path)
}

/// Move the item at `path` to its parent's level, inserted right after the parent.
///
/// # Errors
///
/// Returns `OmelaError::InvalidPath` if `path` has fewer than 2 elements or is out of bounds.
pub fn promote(file: &mut TodoFile, path: &TreePath) -> Result<TreePath> {
    if path.len() < 2 {
        return Err(OmelaError::InvalidPath(path.clone()));
    }
    let item = remove_item(file, path)?;
    let (_, parent_slice) = path.split_last().ok_or_else(|| OmelaError::InvalidPath(path.clone()))?;
    let parent_path: TreePath = parent_slice.to_vec();
    add_sibling(file, &parent_path, item)?;
    let mut new_path = parent_path.clone();
    if let Some(last) = new_path.last_mut() {
        *last += 1;
    }
    Ok(new_path)
}

/// Make the item at `path` the last child of its preceding sibling.
///
/// # Errors
///
/// Returns `OmelaError::InvalidPath` if `path` is empty, out of bounds, or the item has no preceding sibling.
pub fn demote(file: &mut TodoFile, path: &TreePath) -> Result<TreePath> {
    let &idx = path.last().ok_or_else(|| OmelaError::InvalidPath(path.clone()))?;
    if idx == 0 {
        return Err(OmelaError::InvalidPath(path.clone()));
    }
    let item = remove_item(file, path)?;
    let mut sibling_path = path.clone();
    if let Some(last) = sibling_path.last_mut() {
        *last = idx - 1;
    }
    let sibling = get_item_mut(file, &sibling_path).ok_or_else(|| OmelaError::InvalidPath(path.clone()))?;
    let child_idx = sibling.items.len();
    sibling.items.push(item);
    let mut new_path = sibling_path;
    new_path.push(child_idx);
    Ok(new_path)
}

/// Sort the children of the item at `path` alphabetically by title.
///
/// # Errors
///
/// Returns `OmelaError::InvalidPath` if `path` is empty or out of bounds.
pub fn sort_children(file: &mut TodoFile, path: &TreePath) -> Result<()> {
    let parent = get_item_mut(file, path).ok_or_else(|| OmelaError::InvalidPath(path.clone()))?;
    parent.items.sort_by(|a, b| a.title.cmp(&b.title));
    Ok(())
}

/// Search the tree for a node with the given `NodeId` and return its path.
#[must_use]
pub fn find_node_path(file: &TodoFile, node_id: &NodeId) -> Option<TreePath> {
    fn search(items: &[TodoItem], node_id: &NodeId, prefix: &mut Vec<usize>) -> Option<TreePath> {
        for (i, item) in items.iter().enumerate() {
            prefix.push(i);
            if item.id == *node_id {
                return Some(prefix.clone());
            }
            if let Some(found) = search(&item.items, node_id, prefix) {
                return Some(found);
            }
            prefix.pop();
        }
        None
    }
    search(&file.items, node_id, &mut Vec::new())
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
