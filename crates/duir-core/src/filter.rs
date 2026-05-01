use crate::model::TodoItem;
use crate::tree_ops::TreePath;

/// Options controlling filter behavior.
#[derive(Debug, Default, Clone)]
pub struct FilterOptions {
    /// Include note content in search.
    pub search_notes: bool,
    /// Use case-sensitive matching.
    pub case_sensitive: bool,
}

/// Filter items in a tree, returning paths of all matching items.
///
/// An item matches if its title (and optionally note) contains the query.
/// Parent items of matches are included to preserve tree structure.
#[must_use]
pub fn filter_items(items: &[TodoItem], query: &str, options: &FilterOptions) -> Vec<TreePath> {
    let mut matches = Vec::new();
    let mut path = Vec::new();
    for (i, item) in items.iter().enumerate() {
        path.push(i);
        collect_matches(item, query, options, &mut path, &mut matches);
        path.pop();
    }
    matches
}

fn matches_query(item: &TodoItem, query: &str, options: &FilterOptions) -> bool {
    if options.case_sensitive {
        if item.title.contains(query) {
            return true;
        }
        if options.search_notes && item.note.contains(query) {
            return true;
        }
    } else {
        let q = query.to_lowercase();
        if item.title.to_lowercase().contains(&q) {
            return true;
        }
        if options.search_notes && item.note.to_lowercase().contains(&q) {
            return true;
        }
    }
    false
}

fn collect_matches(
    item: &TodoItem,
    query: &str,
    options: &FilterOptions,
    path: &mut TreePath,
    results: &mut Vec<TreePath>,
) -> bool {
    let self_matches = matches_query(item, query, options);
    let mut child_matches = false;

    for (i, child) in item.items.iter().enumerate() {
        path.push(i);
        if collect_matches(child, query, options, path, results) {
            child_matches = true;
        }
        path.pop();
    }

    if self_matches || child_matches {
        results.push(path.clone());
        true
    } else {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::TodoItem;

    fn make_tree() -> Vec<TodoItem> {
        let mut parent = TodoItem::new("Project Alpha");
        parent.note = "Main project notes".to_owned();

        let mut child1 = TodoItem::new("Design API");
        child1.note = "REST endpoints".to_owned();
        child1.items.push(TodoItem::new("Define schemas"));

        let child2 = TodoItem::new("Write tests");

        parent.items.push(child1);
        parent.items.push(child2);

        let other = TodoItem::new("Beta release");
        vec![parent, other]
    }

    #[test]
    fn filter_by_title() {
        let items = make_tree();
        let opts = FilterOptions::default();
        let matches = filter_items(&items, "design", &opts);
        assert!(matches.contains(&vec![0, 0]));
        assert!(matches.contains(&vec![0])); // parent included
    }

    #[test]
    fn filter_case_sensitive() {
        let items = make_tree();
        let opts = FilterOptions {
            case_sensitive: true,
            search_notes: false,
        };
        let matches = filter_items(&items, "design", &opts);
        assert!(matches.is_empty());

        let matches = filter_items(&items, "Design", &opts);
        assert!(matches.contains(&vec![0, 0]));
    }

    #[test]
    fn filter_includes_notes() {
        let items = make_tree();
        let opts = FilterOptions {
            search_notes: true,
            case_sensitive: false,
        };
        let matches = filter_items(&items, "rest endpoints", &opts);
        assert!(matches.contains(&vec![0, 0]));
    }

    #[test]
    fn filter_no_match() {
        let items = make_tree();
        let opts = FilterOptions::default();
        let matches = filter_items(&items, "nonexistent", &opts);
        assert!(matches.is_empty());
    }

    #[test]
    fn filter_parent_included_for_deep_match() {
        let items = make_tree();
        let opts = FilterOptions::default();
        let matches = filter_items(&items, "schemas", &opts);
        // "Define schemas" is at [0, 0, 0], parent [0, 0] and grandparent [0] included
        assert!(matches.contains(&vec![0, 0, 0]));
        assert!(matches.contains(&vec![0, 0]));
        assert!(matches.contains(&vec![0]));
    }

    #[test]
    fn filter_top_level_match() {
        let items = make_tree();
        let opts = FilterOptions::default();
        let matches = filter_items(&items, "beta", &opts);
        assert!(matches.contains(&vec![1]));
        assert!(!matches.contains(&vec![0]));
    }
}
