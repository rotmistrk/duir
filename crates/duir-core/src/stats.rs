use crate::model::{Completion, TodoFile, TodoItem};

/// Completion statistics for a todo tree.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Stats {
    /// Number of leaf items (items with no children).
    pub total_leaves: usize,
    /// Number of leaf items whose completion is `Done`.
    pub checked_leaves: usize,
    /// `checked_leaves * 100 / total_leaves`, or 0 when there are no leaves.
    pub percentage: u8,
}

/// Recursively compute completion statistics for a single item.
///
/// Leaf items (no children) count as one toward `total_leaves`.
/// A leaf counts toward `checked_leaves` when its `completed` field is
/// [`Completion::Done`].
#[must_use]
pub fn compute_stats(item: &TodoItem) -> Stats {
    let (total, checked) = count_leaves(item);
    Stats {
        total_leaves: total,
        checked_leaves: checked,
        percentage: pct(checked, total),
    }
}

/// Recursively update the `completed` field of parent items based on their
/// children.
///
/// * All leaves done → `Done`
/// * Some leaves done → `Partial`
/// * No leaves done  → `Open`
///
/// Leaf items keep their own state unchanged.
pub fn update_completion(item: &mut TodoItem) {
    if item.items.is_empty() {
        return;
    }
    for child in &mut item.items {
        update_completion(child);
    }
    let (total, checked) = count_leaves(item);
    item.completed = if checked == 0 {
        Completion::Open
    } else if checked == total {
        Completion::Done
    } else {
        Completion::Partial
    };
}

/// Aggregate completion statistics across all top-level items in a file.
#[must_use]
pub fn compute_file_stats(file: &TodoFile) -> Stats {
    let (total, checked) = file
        .items
        .iter()
        .map(count_leaves)
        .fold((0, 0), |(t, c), (lt, lc)| (t + lt, c + lc));
    Stats {
        total_leaves: total,
        checked_leaves: checked,
        percentage: pct(checked, total),
    }
}

fn count_leaves(item: &TodoItem) -> (usize, usize) {
    if item.items.is_empty() {
        let done = usize::from(item.completed == Completion::Done);
        return (1, done);
    }
    item.items
        .iter()
        .map(count_leaves)
        .fold((0, 0), |(t, c), (lt, lc)| (t + lt, c + lc))
}

fn pct(checked: usize, total: usize) -> u8 {
    if total == 0 {
        0
    } else {
        u8::try_from(checked * 100 / total).unwrap_or(u8::MAX)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn leaf(done: bool) -> TodoItem {
        let mut item = TodoItem::new("leaf");
        if done {
            item.completed = Completion::Done;
        }
        item
    }

    fn parent(children: Vec<TodoItem>) -> TodoItem {
        let mut item = TodoItem::new("parent");
        item.items = children;
        item
    }

    // --- compute_stats ---

    #[test]
    fn single_leaf_open() {
        let s = compute_stats(&leaf(false));
        assert_eq!(
            s,
            Stats {
                total_leaves: 1,
                checked_leaves: 0,
                percentage: 0
            }
        );
    }

    #[test]
    fn single_leaf_done() {
        let s = compute_stats(&leaf(true));
        assert_eq!(
            s,
            Stats {
                total_leaves: 1,
                checked_leaves: 1,
                percentage: 100
            }
        );
    }

    #[test]
    fn flat_list_all_done() {
        let item = parent(vec![leaf(true), leaf(true), leaf(true)]);
        let s = compute_stats(&item);
        assert_eq!(
            s,
            Stats {
                total_leaves: 3,
                checked_leaves: 3,
                percentage: 100
            }
        );
    }

    #[test]
    fn flat_list_none_done() {
        let item = parent(vec![leaf(false), leaf(false)]);
        let s = compute_stats(&item);
        assert_eq!(
            s,
            Stats {
                total_leaves: 2,
                checked_leaves: 0,
                percentage: 0
            }
        );
    }

    #[test]
    fn flat_list_mixed() {
        let item = parent(vec![leaf(true), leaf(false), leaf(true)]);
        let s = compute_stats(&item);
        assert_eq!(
            s,
            Stats {
                total_leaves: 3,
                checked_leaves: 2,
                percentage: 66
            }
        );
    }

    #[test]
    fn nested_tree() {
        let inner = parent(vec![leaf(true), leaf(false)]);
        let root = parent(vec![inner, leaf(true)]);
        let s = compute_stats(&root);
        assert_eq!(
            s,
            Stats {
                total_leaves: 3,
                checked_leaves: 2,
                percentage: 66
            }
        );
    }

    // --- update_completion ---

    #[test]
    fn update_leaf_unchanged() {
        let mut item = leaf(false);
        update_completion(&mut item);
        assert_eq!(item.completed, Completion::Open);
    }

    #[test]
    fn update_all_done() {
        let mut item = parent(vec![leaf(true), leaf(true)]);
        update_completion(&mut item);
        assert_eq!(item.completed, Completion::Done);
    }

    #[test]
    fn update_none_done() {
        let mut item = parent(vec![leaf(false), leaf(false)]);
        update_completion(&mut item);
        assert_eq!(item.completed, Completion::Open);
    }

    #[test]
    fn update_mixed() {
        let mut item = parent(vec![leaf(true), leaf(false)]);
        update_completion(&mut item);
        assert_eq!(item.completed, Completion::Partial);
    }

    #[test]
    fn update_nested() -> Result<(), String> {
        let inner = parent(vec![leaf(true), leaf(true)]);
        let mut root = parent(vec![inner, leaf(false)]);
        update_completion(&mut root);
        assert_eq!(root.completed, Completion::Partial);
        let first = root.items.first().ok_or("missing first child")?;
        assert_eq!(first.completed, Completion::Done);
        Ok(())
    }

    // --- compute_file_stats ---

    #[test]
    fn file_stats_empty() {
        let file = TodoFile::new("empty");
        let s = compute_file_stats(&file);
        assert_eq!(
            s,
            Stats {
                total_leaves: 0,
                checked_leaves: 0,
                percentage: 0
            }
        );
    }

    #[test]
    fn file_stats_mixed() {
        let mut file = TodoFile::new("mix");
        file.items.push(parent(vec![leaf(true), leaf(false)]));
        file.items.push(leaf(true));
        let s = compute_file_stats(&file);
        assert_eq!(
            s,
            Stats {
                total_leaves: 3,
                checked_leaves: 2,
                percentage: 66
            }
        );
    }
}
