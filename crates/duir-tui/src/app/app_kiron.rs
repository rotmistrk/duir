#[cfg(test)]
pub use super::app_kiron_capture::apply_mcp_mutation_for_test;
pub use super::app_kiron_capture::sync_mcp_snapshot;

use super::app_kiron_mcp;
use super::{ActiveKiron, App, FocusState, StatusLevel};
use duir_core::NodeId;
use std::sync::{Arc, Mutex};

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

    fn kiro_start(&mut self) {
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

        let Some(item) = item else {
            "Node not found".clone_into(&mut self.status_message);
            return;
        };

        if !item.is_kiron() {
            self.set_status("Not a kiron node. Use :kiron first", StatusLevel::Error);
            return;
        }

        let Some(file) = self.files.get(fi) else { return };
        let file_id = file.id;
        let node_id = item.id.clone();
        let key = (file_id, node_id);

        if self.active_kirons.contains_key(&key) {
            self.set_status("Kiron already active", StatusLevel::Warning);
            return;
        }

        // Build MCP snapshot of the kiron subtree
        let mut snapshot_file = duir_core::TodoFile::new(&item.title);
        snapshot_file.items.clone_from(&item.items);
        snapshot_file.note.clone_from(&item.note);

        let snapshot = Arc::new(Mutex::new(snapshot_file));
        let (mutation_tx, mutation_rx) = std::sync::mpsc::channel();

        let session_id = item.kiron.as_ref().map_or("unknown", |k| k.session_id.as_str());

        let socket_path = match app_kiron_mcp::start_mcp_listener(Arc::clone(&snapshot), mutation_tx, session_id) {
            Ok(p) => p,
            Err(e) => {
                self.set_status(&e, StatusLevel::Error);
                return;
            }
        };

        // Spawn kiro-cli with --agent duir and DUIR_MCP_SOCKET env var
        let config = duir_core::config::Config::load();

        app_kiron_mcp::ensure_agent_file(&config.kiro.sop);
        let (cmd, mut args) = config.kiro.build_command(std::path::Path::new("."));
        args.push("--agent".to_owned());
        args.push("duir".to_owned());

        let arg_refs: Vec<&str> = args.iter().map(String::as_str).collect();
        let socket_str = socket_path.to_string_lossy().into_owned();
        let envs = [("DUIR_MCP_SOCKET", socket_str.as_str())];
        let cwd = std::env::current_dir().unwrap_or_default();

        match crate::pty_tab::PtyTab::spawn(&cmd, &arg_refs, 80, 24, &cwd, &envs) {
            Ok(pty) => {
                self.active_kirons.insert(
                    key,
                    ActiveKiron {
                        pty,
                        response_ready: false,
                        had_output: false,
                        mcp_snapshot: Some(snapshot),
                        mutation_rx: Some(mutation_rx),
                        socket_path: Some(socket_path),
                    },
                );

                self.flags.set_kiro_tab_focused(false);
                self.set_status("Kiro started (MCP available)", StatusLevel::Success);
            }
            Err(e) => {
                let _ = std::fs::remove_file(&socket_path);
                self.set_status(&format!("Failed to start kiro: {e}"), StatusLevel::Error);
            }
        }
    }

    fn kiro_stop(&mut self) {
        let Some(row) = self.current_row().cloned() else {
            "No node selected".clone_into(&mut self.status_message);
            return;
        };

        let fi = row.file_index;

        let node_id = if row.path.is_empty() {
            NodeId(String::new())
        } else {
            self.files
                .get(fi)
                .and_then(|f| duir_core::tree_ops::get_item(&f.data, &row.path))
                .map_or_else(|| NodeId(String::new()), |it| it.id.clone())
        };

        let file_id = self.files.get(fi).map_or(super::FileId(0), |f| f.id);
        let key = (file_id, node_id);

        if let Some(kiron) = self.active_kirons.remove(&key) {
            if let Some(ref p) = kiron.socket_path {
                let _ = std::fs::remove_file(p);
            }

            if self.is_kiro_focused() {
                self.state = FocusState::Tree;
            }

            self.flags.set_kiro_tab_focused(false);
            self.set_status("Kiro session stopped", StatusLevel::Success);
        } else {
            self.set_status("No active kiro session on this node", StatusLevel::Warning);
        }
    }

    /// Stop current session, generate new `session_id`, start fresh.
    fn kiro_new_session(&mut self) {
        let Some(row) = self.current_row().cloned() else {
            "No node selected".clone_into(&mut self.status_message);
            return;
        };

        let fi = row.file_index;
        let path = row.path;

        let (node_id, is_kiron) = {
            let item = if path.is_empty() {
                None
            } else {
                self.files
                    .get(fi)
                    .and_then(|f| duir_core::tree_ops::get_item(&f.data, &path))
            };

            if let Some(it) = item {
                (it.id.clone(), it.is_kiron())
            } else {
                "Node not found".clone_into(&mut self.status_message);
                return;
            }
        };

        if !is_kiron {
            self.set_status("Not a kiron node", StatusLevel::Warning);
            return;
        }

        // Stop existing session if running
        let file_id = self.files.get(fi).map_or(super::FileId(0), |f| f.id);
        let key = (file_id, node_id);

        if let Some(kiron) = self.active_kirons.remove(&key) {
            if let Some(ref p) = kiron.socket_path {
                let _ = std::fs::remove_file(p);
            }

            if self.is_kiro_focused() {
                self.state = FocusState::Tree;
            }

            self.flags.set_kiro_tab_focused(false);
        }

        // Generate new session_id
        let new_session = uuid::Uuid::new_v4().to_string();

        if let Some(file) = self.files.get_mut(fi)
            && let Some(item) = duir_core::tree_ops::get_item_mut(&mut file.data, &path)
        {
            item.kiron = Some(duir_core::KironMeta {
                session_id: new_session.clone(),
            });
        }

        self.mark_modified(fi, &path);
        self.rebuild_rows();

        // Start fresh
        self.kiro_start();

        if self.status_level == StatusLevel::Success {
            self.set_status(
                &format!("New kiro session ({})", &new_session[..8]),
                StatusLevel::Success,
            );
        }
    }
}
