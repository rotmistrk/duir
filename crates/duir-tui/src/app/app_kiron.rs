#[cfg(test)]
pub use super::app_kiron_mutation::apply_mcp_mutation_for_test;
pub use super::app_kiron_mutation::sync_mcp_snapshot;

use super::{App, StatusLevel};
use duir_core::NodeId;

impl App {
    /// Mark or disable a kiron on the current node.
    pub(crate) fn cmd_kiron(&mut self, parts: &[&str]) {
        if parts.get(1).copied() == Some("disable") {
            self.kiron_disable();
            return;
        }

        let Some(row) = self.current_row().cloned() else {
            "No node selected".clone_into(&mut self.status_message);
            return;
        };

        let fi = row.file_index;
        let path = &row.path;

        let item = if path.is_empty() {
            "Cannot mark file root as kiron".clone_into(&mut self.status_message);
            return;
        } else {
            let Some(file) = self.files.get_mut(fi) else { return };
            duir_core::tree_ops::get_item_mut(&mut file.data, path)
        };

        let Some(item) = item else {
            "Node not found".clone_into(&mut self.status_message);
            return;
        };

        if item.is_kiron() {
            self.set_status("Already a kiron", StatusLevel::Warning);
            return;
        }

        let session_id = uuid::Uuid::new_v4().to_string();

        item.node_type = Some(duir_core::NodeType::Kiron);
        item.kiron = Some(duir_core::KironMeta {
            session_id: session_id.clone(),
        });

        self.mark_modified(fi, path);
        self.rebuild_rows();
        self.set_status(
            &format!("Marked as kiron (session {})", &session_id[..8]),
            StatusLevel::Success,
        );
    }

    pub(crate) fn kiron_disable(&mut self) {
        let Some(row) = self.current_row().cloned() else {
            "No node selected".clone_into(&mut self.status_message);
            return;
        };

        let fi = row.file_index;
        let path = row.path;

        let item = if path.is_empty() {
            None
        } else {
            self.files
                .get(fi)
                .and_then(|f| duir_core::tree_ops::get_item(&f.data, &path))
        };

        let node_id = item.map_or_else(|| NodeId(String::new()), |it| it.id.clone());

        let file_id = self.files.get(fi).map_or(super::FileId(0), |f| f.id);
        if self.active_kirons.contains_key(&(file_id, node_id)) {
            self.set_status("Stop kiro first (:kiro stop)", StatusLevel::Error);
            return;
        }

        let item = if path.is_empty() {
            None
        } else {
            self.files
                .get_mut(fi)
                .and_then(|f| duir_core::tree_ops::get_item_mut(&mut f.data, &path))
        };

        let Some(item) = item else {
            "Node not found".clone_into(&mut self.status_message);
            return;
        };

        if !item.is_kiron() {
            self.set_status("Not a kiron node", StatusLevel::Warning);
            return;
        }

        item.node_type = None;
        item.kiron = None;

        self.mark_modified(fi, &path);
        self.rebuild_rows();
        self.set_status("Kiron disabled", StatusLevel::Success);
    }

    /// Start, stop, or reset a kiro session on the current kiron node.
    pub(crate) fn cmd_kiro(&mut self, parts: &[&str]) {
        let subcmd = parts.get(1).copied().unwrap_or("");

        match subcmd {
            "start" => self.kiro_start(),
            "stop" => self.kiro_stop(),
            "new" => self.kiro_new_session(),
            "capture" => self.capture_kiro_response(),
            _ => {
                "Usage: :kiro start | stop | new | capture".clone_into(&mut self.status_message);
            }
        }
    }
}
