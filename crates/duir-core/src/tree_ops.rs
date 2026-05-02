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
    let cloned = get_item(file, path)
        .ok_or_else(|| OmelaError::InvalidPath(path.clone()))?
        .clone();
    add_sibling(file, path, cloned)
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
    let parent_path: TreePath = path[..path.len() - 1].to_vec();
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
#[allow(clippy::expect_used)]
mod tests {
    use super::*;
    use crate::model::TodoFile;

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
}
