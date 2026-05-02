use duir_core::NodeId;

use super::{ActiveKiron, App, PendingResponse, StatusLevel};

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
            duir_core::tree_ops::get_item_mut(&mut self.files[fi].data, path)
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
            duir_core::tree_ops::get_item(&self.files[fi].data, &path)
        };
        let node_id = item.map_or_else(|| NodeId(String::new()), |it| it.id.clone());
        if self.active_kirons.contains_key(&(self.files[fi].id, node_id)) {
            self.set_status("Stop kiro first (:kiro stop)", StatusLevel::Error);
            return;
        }
        let item = if path.is_empty() {
            None
        } else {
            duir_core::tree_ops::get_item_mut(&mut self.files[fi].data, &path)
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

    /// Start or stop a kiro session on the current kiron node.
    pub(crate) fn cmd_kiro(&mut self, parts: &[&str]) {
        let subcmd = parts.get(1).copied().unwrap_or("");
        match subcmd {
            "start" => self.kiro_start(),
            "stop" => self.kiro_stop(),
            _ => {
                "Usage: :kiro start | :kiro stop".clone_into(&mut self.status_message);
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
            duir_core::tree_ops::get_item(&self.files[fi].data, &path)
        };
        let Some(item) = item else {
            "Node not found".clone_into(&mut self.status_message);
            return;
        };
        if !item.is_kiron() {
            self.set_status("Not a kiron node. Use :kiron first", StatusLevel::Error);
            return;
        }
        let file_id = self.files[fi].id;
        let node_id = item.id.clone();
        let key = (file_id, node_id);
        if self.active_kirons.contains_key(&key) {
            self.set_status("Kiron already active", StatusLevel::Warning);
            return;
        }
        let config = duir_core::config::Config::load();
        let (cmd, args) = config.kiro.build_command(std::path::Path::new("."));
        let arg_refs: Vec<&str> = args.iter().map(String::as_str).collect();
        let cwd = std::env::current_dir().unwrap_or_default();
        match crate::pty_tab::PtyTab::spawn(&cmd, &arg_refs, 80, 24, &cwd) {
            Ok(pty) => {
                self.active_kirons.insert(key, ActiveKiron { pty });
                self.kiro_tab_focused = false;
                self.set_status("Kiro session started", StatusLevel::Success);
            }
            Err(e) => {
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
            duir_core::tree_ops::get_item(&self.files[fi].data, &row.path)
                .map_or_else(|| NodeId(String::new()), |it| it.id.clone())
        };
        let key = (self.files[fi].id, node_id);
        if self.active_kirons.remove(&key).is_some() {
            self.kiro_tab_focused = false;
            self.set_status("Kiro session stopped", StatusLevel::Success);
        } else {
            self.set_status("No active kiro session on this node", StatusLevel::Warning);
        }
    }

    /// Find the active kiron for the current cursor position.
    pub fn active_kiron_for_cursor(&self) -> Option<(super::FileId, NodeId)> {
        let row = self.current_row()?;
        let fi = row.file_index;
        let file_id = self.files[fi].id;
        let path = &row.path;

        let mut best: Option<(&(super::FileId, NodeId), usize)> = None;
        for len in (1..=path.len()).rev() {
            let ancestor_path = &path[..len];
            if let Some(item) = duir_core::tree_ops::get_item(&self.files[fi].data, &ancestor_path.to_vec()) {
                let key_candidate = (file_id, item.id.clone());
                if self.active_kirons.contains_key(&key_candidate) {
                    for key in self.active_kirons.keys() {
                        if *key == key_candidate && best.as_ref().is_none_or(|(_, d)| len > *d) {
                            best = Some((key, len));
                            break;
                        }
                    }
                }
            }
        }
        best.map(|(k, _)| k.clone())
    }

    /// Poll all active kiron PTYs for new output and clean up finished ones.
    pub fn poll_kirons(&mut self) {
        for kiron in self.active_kirons.values_mut() {
            kiron.pty.poll();
        }
        let finished: Vec<_> = self
            .active_kirons
            .iter()
            .filter(|(_, k)| k.pty.finished)
            .map(|(key, _)| key.clone())
            .collect();
        for key in finished {
            self.active_kirons.remove(&key);
            self.set_status("Kiro process exited", StatusLevel::Warning);
        }
    }

    /// Send the current node's content as a prompt to the active kiron's PTY.
    pub(crate) fn send_to_kiro(&mut self) {
        let Some(row) = self.current_row().cloned() else {
            return;
        };
        let Some(kiron_key) = self.active_kiron_for_cursor() else {
            return;
        };
        let fi = row.file_index;
        let path = row.path;

        let content = if path.is_empty() {
            duir_core::markdown_export::export_file(&self.files[fi].data)
        } else {
            let Some(item) = duir_core::tree_ops::get_item(&self.files[fi].data, &path) else {
                return;
            };
            duir_core::markdown_export::export_subtree_safe(item, 3)
        };

        let Some(kiron) = self.active_kirons.get_mut(&kiron_key) else {
            return;
        };
        let mut payload = String::with_capacity(content.len() + 16);
        payload.push_str("\x1b[200~");
        payload.push_str(&content);
        payload.push_str("\x1b[201~");
        payload.push('\n');
        kiron.pty.write(payload.as_bytes());

        let prompt_node_id = if path.is_empty() {
            NodeId(String::new())
        } else if let Some(item) = duir_core::tree_ops::get_item_mut(&mut self.files[fi].data, &path) {
            item.node_type = Some(duir_core::NodeType::Prompt);
            if !item.title.starts_with("📤 ") {
                item.title = format!("📤 {}", item.title);
            }
            item.id.clone()
        } else {
            NodeId(String::new())
        };
        if !path.is_empty() {
            self.mark_modified(fi, &path);
        }

        self.pending_responses.push(PendingResponse {
            kiron_file_id: kiron_key.0,
            kiron_node_id: kiron_key.1,
            prompt_file_id: self.files[fi].id,
            prompt_node_id,
            start_time: std::time::Instant::now(),
        });

        self.rebuild_rows();
        self.set_status("Prompt sent to kiro", StatusLevel::Success);
    }

    /// Check pending responses for idle PTYs and capture output.
    pub(crate) fn check_response_capture(&mut self) {
        let idle_threshold = std::time::Duration::from_secs(5);
        let min_wait = std::time::Duration::from_secs(2);
        let now = std::time::Instant::now();

        let mut completed = Vec::new();
        for (i, pr) in self.pending_responses.iter().enumerate() {
            if now.duration_since(pr.start_time) < min_wait {
                continue;
            }
            let key = (pr.kiron_file_id, pr.kiron_node_id.clone());
            let Some(kiron) = self.active_kirons.get(&key) else {
                completed.push(i);
                continue;
            };
            let idle_time = now.duration_since(kiron.pty.last_output_time);
            if idle_time < idle_threshold {
                continue;
            }
            let output = crate::termbuf::extract_last_output(&kiron.pty.termbuf);
            if output.trim().is_empty() {
                continue;
            }
            completed.push(i);
        }

        for &i in completed.iter().rev() {
            let pr = self.pending_responses.remove(i);
            let key = (pr.kiron_file_id, pr.kiron_node_id.clone());
            let Some(kiron) = self.active_kirons.get(&key) else {
                continue;
            };
            let output = crate::termbuf::extract_last_output(&kiron.pty.termbuf);
            if output.trim().is_empty() {
                continue;
            }

            let first_line = output.lines().find(|l| !l.trim().is_empty()).unwrap_or("Response");
            let truncated: String = first_line.chars().take(80).collect();
            let title = format!("📥 {truncated}");

            let Some(kiron_fi) = self.file_index_for_id(pr.kiron_file_id) else {
                continue;
            };
            let Some(kiron_path) = duir_core::tree_ops::find_node_path(&self.files[kiron_fi].data, &pr.kiron_node_id)
            else {
                continue;
            };

            let session_id = duir_core::tree_ops::get_item(&self.files[kiron_fi].data, &kiron_path)
                .and_then(|item| item.kiron.as_ref())
                .map_or_else(|| "unknown".to_owned(), |k| k.session_id.clone());

            let timestamp = chrono::Utc::now().to_rfc3339();
            let note = format!(
                "<!-- duir:response kiron={session_id} timestamp={timestamp} -->\n\
                 {output}"
            );

            let mut response_node = duir_core::TodoItem::new(&title);
            response_node.note = note;
            response_node.node_type = Some(duir_core::NodeType::Response);

            let Some(prompt_fi) = self.file_index_for_id(pr.prompt_file_id) else {
                continue;
            };
            let insert_path = duir_core::tree_ops::find_node_path(&self.files[prompt_fi].data, &pr.prompt_node_id)
                .unwrap_or_else(|| kiron_path.clone());

            if let Err(e) =
                duir_core::tree_ops::add_sibling(&mut self.files[prompt_fi].data, &insert_path, response_node)
            {
                self.set_status(&format!("Failed to insert response: {e}"), StatusLevel::Error);
                continue;
            }
            self.mark_modified(prompt_fi, &insert_path);
        }

        if !completed.is_empty() {
            self.rebuild_rows();
        }
    }
}
