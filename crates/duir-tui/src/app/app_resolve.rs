use duir_core::TodoStorage;
use duir_core::conflict::{ConflictKind, NodeConflict, Resolution, find_conflicts};

use super::{App, FocusState, StatusLevel};

/// State for the conflict resolution overlay.
#[derive(Debug, Clone)]
pub struct ConflictState {
    pub conflicts: Vec<NodeConflict>,
    pub resolutions: Vec<Option<Resolution>>,
    pub cursor: usize,
    pub file_index: usize,
}

impl App {
    pub fn cmd_resolve(&mut self, storage: &dyn TodoStorage) {
        let Some(row) = self.current_row().cloned() else { return };
        let fi = row.file_index;
        let Some(file) = self.files.get(fi) else { return };

        if !file.conflicted {
            "No conflict on current file".clone_into(&mut self.status_message);
            return;
        }

        // Load disk version
        let disk_data = match storage.load(&file.name) {
            Ok(d) => d,
            Err(e) => {
                self.set_status(&format!("Cannot load disk version: {e}"), StatusLevel::Error);
                return;
            }
        };

        let conflicts = find_conflicts(&file.data.items, &disk_data.items);
        if conflicts.is_empty() {
            if let Some(f) = self.files.get_mut(fi) {
                f.conflicted = false;
                f.disk_mtime = storage.mtime(&f.name);
            }
            self.set_status("Conflict resolved (no node differences)", StatusLevel::Success);
            return;
        }

        let resolutions = vec![None; conflicts.len()];
        self.state = FocusState::Resolve(ConflictState {
            conflicts,
            resolutions,
            cursor: 0,
            file_index: fi,
        });
    }

    pub fn resolve_apply(&mut self, storage: &dyn TodoStorage) {
        let FocusState::Resolve(state) = &self.state else {
            return;
        };
        let state = state.clone();

        if state.resolutions.iter().any(Option::is_none) {
            self.set_status("Resolve all conflicts first (m/t/b)", StatusLevel::Warning);
            return;
        }

        let fi = state.file_index;
        let Some(file_name) = self.files.get(fi).map(|f| f.name.clone()) else {
            return;
        };

        let disk_data = match storage.load(&file_name) {
            Ok(d) => d,
            Err(e) => {
                self.set_status(&format!("Cannot load disk version: {e}"), StatusLevel::Error);
                return;
            }
        };

        let theirs_map = duir_core::conflict::collect_by_id(&disk_data.items);

        // Apply resolutions
        if let Some(file) = self.files.get_mut(fi) {
            for (conflict, resolution) in state.conflicts.iter().zip(state.resolutions.iter()) {
                let Some(res) = resolution else { continue };
                match res {
                    Resolution::KeepMine => {}
                    Resolution::KeepTheirs => {
                        replace_node_by_id(&mut file.data.items, &conflict.id, &theirs_map);
                    }
                    Resolution::KeepBoth => {
                        if let Some(their_node) = theirs_map.get(&conflict.id) {
                            insert_copy_after(&mut file.data.items, &conflict.id, their_node);
                        }
                    }
                }
            }

            // Add back nodes deleted locally if user chose theirs/both
            for (conflict, resolution) in state.conflicts.iter().zip(state.resolutions.iter()) {
                let Some(res) = resolution else { continue };
                if conflict.kind == ConflictKind::DeletedLocally
                    && *res != Resolution::KeepMine
                    && let Some(their_node) = theirs_map.get(&conflict.id)
                {
                    file.data.items.push((*their_node).clone());
                }
            }

            file.conflicted = false;
            file.disk_mtime = storage.mtime(&file.name);
        }

        self.state = FocusState::Tree;
        self.rebuild_rows();
        self.set_status("Conflicts resolved", StatusLevel::Success);
    }
}

fn replace_node_by_id(
    items: &mut [duir_core::TodoItem],
    id: &duir_core::NodeId,
    theirs: &std::collections::HashMap<duir_core::NodeId, &duir_core::TodoItem>,
) {
    for item in items.iter_mut() {
        if item.id == *id {
            if let Some(their) = theirs.get(id) {
                their.title.clone_into(&mut item.title);
                their.note.clone_into(&mut item.note);
                item.completed.clone_from(&their.completed);
                item.important = their.important;
            }
            return;
        }
        replace_node_by_id(&mut item.items, id, theirs);
    }
}

fn insert_copy_after(items: &mut Vec<duir_core::TodoItem>, id: &duir_core::NodeId, node: &duir_core::TodoItem) {
    if let Some(pos) = items.iter().position(|i| i.id == *id) {
        let mut copy = node.clone();
        copy.id = duir_core::NodeId::new();
        copy.title = format!("{} (conflict)", copy.title);
        items.insert(pos + 1, copy);
        return;
    }
    for item in items.iter_mut() {
        insert_copy_after(&mut item.items, id, node);
    }
}
