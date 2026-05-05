use duir_core::NodeId;
use std::sync::{Arc, Mutex};

use super::App;

impl App {
    /// Drain mutation channels and apply MCP mutations to the real tree.
    pub(crate) fn process_mcp_mutations(&mut self) {
        let mut all_mutations: Vec<((super::FileId, NodeId), Vec<duir_core::mcp_server::McpMutation>)> = Vec::new();
        for (key, kiron) in &mut self.active_kirons {
            if let Some(ref rx) = kiron.mutation_rx {
                let mutations: Vec<_> = rx.try_iter().collect();
                if !mutations.is_empty() {
                    all_mutations.push((key.clone(), mutations));
                }
            }
        }

        for (key, mutations) in all_mutations {
            let Some(fi) = self.file_index_for_id(key.0) else {
                continue;
            };
            let Some(file) = self.files.get_mut(fi) else { continue };
            let Some(kiron_path) = duir_core::tree_ops::find_node_path(&file.data, &key.1) else {
                continue;
            };
            let mut modified = false;
            for mutation in &mutations {
                if apply_mcp_mutation(&mut file.data, &kiron_path, mutation) {
                    modified = true;
                }
            }
            if modified {
                self.mark_modified(fi, &kiron_path);
                self.rebuild_rows();
            }
        }
    }
}

/// Apply a single MCP mutation to the tree. Returns true if modified.
fn apply_mcp_mutation(
    file: &mut duir_core::TodoFile,
    kiron_path: &[usize],
    mutation: &duir_core::mcp_server::McpMutation,
) -> bool {
    use duir_core::mcp_server::McpMutation;
    match mutation {
        McpMutation::AddChild {
            parent_path,
            title,
            note,
        } => {
            let mut abs = kiron_path.to_vec();
            abs.extend(parent_path);
            let mut item = duir_core::TodoItem::new(title);
            item.note.clone_from(note);
            duir_core::tree_ops::add_child(file, &abs, item).is_ok()
        }
        McpMutation::AddSibling { path, title, note } => {
            let mut abs = kiron_path.to_vec();
            abs.extend(path);
            let mut item = duir_core::TodoItem::new(title);
            item.note.clone_from(note);
            duir_core::tree_ops::add_sibling(file, &abs, item).is_ok()
        }
        McpMutation::MarkDone { path } => {
            let mut abs = kiron_path.to_vec();
            abs.extend(path);
            if let Some(item) = duir_core::tree_ops::get_item_mut(file, &abs) {
                item.completed = duir_core::Completion::Done;
                true
            } else {
                false
            }
        }
        McpMutation::MarkImportant { path } => {
            let mut abs = kiron_path.to_vec();
            abs.extend(path);
            if let Some(item) = duir_core::tree_ops::get_item_mut(file, &abs) {
                item.important = !item.important;
                true
            } else {
                false
            }
        }
        McpMutation::Reorder { path, direction } => {
            let mut abs = kiron_path.to_vec();
            abs.extend(path);
            match direction {
                duir_core::mcp_server::ReorderDirection::Up => duir_core::tree_ops::swap_up(file, &abs).is_ok(),
                duir_core::mcp_server::ReorderDirection::Down => duir_core::tree_ops::swap_down(file, &abs).is_ok(),
            }
        }
    }
}

/// Update the MCP snapshot from the current kiron subtree state.
pub fn sync_mcp_snapshot(snapshot: &Arc<Mutex<duir_core::TodoFile>>, item: &duir_core::TodoItem) {
    if let Ok(mut guard) = snapshot.lock() {
        guard.items.clone_from(&item.items);
        guard.note.clone_from(&item.note);
        guard.title.clone_from(&item.title);
    }
}

#[cfg(test)]
pub fn apply_mcp_mutation_for_test(
    file: &mut duir_core::TodoFile,
    kiron_path: &[usize],
    mutation: &duir_core::mcp_server::McpMutation,
) -> bool {
    apply_mcp_mutation(file, kiron_path, mutation)
}
