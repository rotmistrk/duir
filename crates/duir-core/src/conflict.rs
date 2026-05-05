use crate::model::{NodeId, TodoItem};
use std::collections::HashMap;

/// A single conflict between local and disk versions of a node.
#[derive(Debug, Clone)]
pub struct NodeConflict {
    pub id: NodeId,
    pub title_mine: String,
    pub title_theirs: String,
    pub note_mine: String,
    pub note_theirs: String,
    pub kind: ConflictKind,
}

/// What kind of conflict this is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConflictKind {
    Modified,
    DeletedLocally,
    DeletedOnDisk,
}

/// Resolution choice for a conflict.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Resolution {
    KeepMine,
    KeepTheirs,
    KeepBoth,
}

/// Find conflicts between local (mine) and disk (theirs) trees.
#[must_use]
pub fn find_conflicts(mine: &[TodoItem], theirs: &[TodoItem]) -> Vec<NodeConflict> {
    let mine_map = collect_nodes(mine);
    let theirs_map = collect_nodes(theirs);
    let mut conflicts = Vec::new();

    for (id, my_node) in &mine_map {
        if let Some(their_node) = theirs_map.get(id) {
            if node_differs(my_node, their_node) {
                conflicts.push(NodeConflict {
                    id: id.clone(),
                    title_mine: my_node.title.clone(),
                    title_theirs: their_node.title.clone(),
                    note_mine: my_node.note.clone(),
                    note_theirs: their_node.note.clone(),
                    kind: ConflictKind::Modified,
                });
            }
        } else {
            conflicts.push(NodeConflict {
                id: id.clone(),
                title_mine: my_node.title.clone(),
                title_theirs: String::new(),
                note_mine: my_node.note.clone(),
                note_theirs: String::new(),
                kind: ConflictKind::DeletedOnDisk,
            });
        }
    }

    for (id, their_node) in &theirs_map {
        if !mine_map.contains_key(id) {
            conflicts.push(NodeConflict {
                id: id.clone(),
                title_mine: String::new(),
                title_theirs: their_node.title.clone(),
                note_mine: String::new(),
                note_theirs: their_node.note.clone(),
                kind: ConflictKind::DeletedLocally,
            });
        }
    }

    conflicts
}

fn node_differs(a: &TodoItem, b: &TodoItem) -> bool {
    a.title != b.title || a.note != b.note || a.completed != b.completed || a.important != b.important
}

fn collect_nodes(items: &[TodoItem]) -> HashMap<NodeId, &TodoItem> {
    let mut map = HashMap::new();
    collect_recursive(items, &mut map);
    map
}

/// Collect all nodes by ID (public for resolution).
#[must_use]
pub fn collect_by_id(items: &[TodoItem]) -> HashMap<NodeId, &TodoItem> {
    let mut map = HashMap::new();
    collect_recursive(items, &mut map);
    map
}

fn collect_recursive<'a>(items: &'a [TodoItem], map: &mut HashMap<NodeId, &'a TodoItem>) {
    for item in items {
        map.insert(item.id.clone(), item);
        collect_recursive(&item.items, map);
    }
}
