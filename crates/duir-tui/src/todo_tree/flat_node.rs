//! Flattened node for display in the todo tree.

use super::model::TreePath;

/// Flattened node for display in the tree.
pub(super) struct FlatNode {
    pub(super) depth: usize,
    pub(super) path: TreePath,
    pub(super) expandable: bool,
    pub(super) expanded: bool,
    /// Whether this node is the last child of its parent (used by connector drawing).
    #[allow(dead_code)]
    pub(super) is_last_child: bool,
}
