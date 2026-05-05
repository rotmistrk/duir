use super::{App, PendingResponse, StatusLevel};
use duir_core::NodeId;

impl App {
    pub fn active_kiron_for_cursor(&self) -> Option<(super::FileId, NodeId)> {
        let row = self.current_row()?;
        let fi = row.file_index;
        let file = self.files.get(fi)?;
        let file_id = file.id;
        let path = &row.path;

        let mut best: Option<(&(super::FileId, NodeId), usize)> = None;
        for len in (1..=path.len()).rev() {
            let Some(ancestor_path) = path.get(..len) else { continue };
            if let Some(item) = duir_core::tree_ops::get_item(&file.data, &ancestor_path.to_vec()) {
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
        let idle_threshold = std::time::Duration::from_secs(3);
        let now = std::time::Instant::now();
        for kiron in self.active_kirons.values_mut() {
            let before = kiron.pty.last_output_time;
            kiron.pty.poll();
            kiron.had_output = kiron.pty.last_output_time != before;
            if kiron.had_output {
                kiron.response_ready = false;
            }
        }
        for pr in &self.pending_responses {
            let key = (pr.kiron_file_id, pr.kiron_node_id.clone());
            if let Some(kiron) = self.active_kirons.get_mut(&key) {
                let idle = now.duration_since(kiron.pty.last_output_time);
                if idle >= idle_threshold && !kiron.response_ready {
                    kiron.response_ready = true;
                }
            }
        }
        let finished: Vec<_> = self
            .active_kirons
            .iter()
            .filter(|(_, k)| k.pty.finished)
            .map(|(key, _)| key.clone())
            .collect();
        for key in finished {
            if let Some(kiron) = self.active_kirons.remove(&key)
                && let Some(ref p) = kiron.socket_path
            {
                let _ = std::fs::remove_file(p);
            }
            self.set_status("Kiro process exited", StatusLevel::Warning);
        }
        self.process_mcp_mutations();
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
        self.finalize_capture_for_kiron(&kiron_key);
        let Some(file) = self.files.get(fi) else { return };
        let content = if path.is_empty() {
            duir_core::markdown_export::export_file(&file.data)
        } else {
            let Some(item) = duir_core::tree_ops::get_item(&file.data, &path) else {
                return;
            };
            duir_core::markdown_export::export_subtree_safe(item, 3)
        };

        let Some(kiron) = self.active_kirons.get_mut(&kiron_key) else {
            return;
        };
        let capture_start = kiron.pty.termbuf.total_lines();

        let mut payload = String::with_capacity(content.len() + 16);
        payload.push_str("\x1b[200~");
        payload.push_str(&content);
        payload.push_str("\x1b[201~");
        payload.push('\r');
        kiron.pty.write(payload.as_bytes());

        let prompt_node_id = if path.is_empty() {
            NodeId(String::new())
        } else if let Some(file) = self.files.get_mut(fi)
            && let Some(item) = duir_core::tree_ops::get_item_mut(&mut file.data, &path)
        {
            item.node_type = Some(duir_core::NodeType::Prompt);
            if !item.title.starts_with("❓ ") {
                item.title = format!("❓ {}", item.title);
            }
            item.id.clone()
        } else {
            NodeId(String::new())
        };
        if !path.is_empty() {
            self.mark_modified(fi, &path);
        }

        let file_id = self.files.get(fi).map_or(super::FileId(0), |f| f.id);
        self.pending_responses.push(PendingResponse {
            kiron_file_id: kiron_key.0,
            kiron_node_id: kiron_key.1,
            prompt_file_id: file_id,
            prompt_node_id,
            capture_start_line: capture_start,
        });

        self.rebuild_rows();
        self.set_status("Prompt sent to kiro", StatusLevel::Success);
    }

    fn finalize_capture_for_kiron(&mut self, kiron_key: &(super::FileId, NodeId)) {
        let indices: Vec<usize> = self
            .pending_responses
            .iter()
            .enumerate()
            .filter(|(_, pr)| pr.kiron_file_id == kiron_key.0 && pr.kiron_node_id == kiron_key.1)
            .map(|(i, _)| i)
            .collect();
        for &i in indices.iter().rev() {
            self.finalize_capture(i);
        }
    }

    /// Explicitly capture the kiro response for the most recent pending prompt.
    pub(crate) fn capture_kiro_response(&mut self) {
        if self.pending_responses.is_empty() {
            self.set_status("No pending capture", StatusLevel::Warning);
            return;
        }
        let last = self.pending_responses.len() - 1;
        self.finalize_capture(last);
        self.rebuild_rows();
    }

    /// Finalize a single pending capture at the given index.
    fn finalize_capture(&mut self, index: usize) {
        let Some(pr) = self.pending_responses.get(index) else {
            return;
        };
        let key = (pr.kiron_file_id, pr.kiron_node_id.clone());
        let Some(kiron) = self.active_kirons.get(&key) else {
            self.pending_responses.remove(index);
            return;
        };

        let output = crate::termbuf::extract_text_from_line(&kiron.pty.termbuf, pr.capture_start_line);
        let pr = self.pending_responses.remove(index);

        if let Some(k) = self
            .active_kirons
            .get_mut(&(pr.kiron_file_id, pr.kiron_node_id.clone()))
        {
            k.response_ready = false;
        }
        if output.trim().is_empty() {
            self.set_status("Captured empty response (skipped)", StatusLevel::Warning);
            return;
        }

        let first_line = output.lines().find(|l| !l.trim().is_empty()).unwrap_or("Response");
        let truncated: String = first_line.chars().take(80).collect();
        let title = format!("💡 {truncated}");

        let Some(kiron_fi) = self.file_index_for_id(pr.kiron_file_id) else {
            return;
        };
        let Some(file) = self.files.get(kiron_fi) else { return };
        let Some(kiron_path) = duir_core::tree_ops::find_node_path(&file.data, &pr.kiron_node_id) else {
            return;
        };

        let session_id = duir_core::tree_ops::get_item(&file.data, &kiron_path)
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
            return;
        };
        let Some(pfile) = self.files.get_mut(prompt_fi) else {
            return;
        };
        let insert_path =
            duir_core::tree_ops::find_node_path(&pfile.data, &pr.prompt_node_id).unwrap_or_else(|| kiron_path.clone());

        if let Err(e) = duir_core::tree_ops::add_sibling(&mut pfile.data, &insert_path, response_node) {
            self.set_status(&format!("Failed to insert response: {e}"), StatusLevel::Error);
            return;
        }
        self.mark_modified(prompt_fi, &insert_path);
        self.set_status("Response captured", StatusLevel::Success);
    }

    /// Clear `response_ready` on the kiron under the current cursor.
    pub fn clear_response_ready(&mut self) {
        if let Some(key) = self.active_kiron_for_cursor()
            && let Some(kiron) = self.active_kirons.get_mut(&key)
        {
            kiron.response_ready = false;
        }
    }

    /// Collect tree paths of kiron nodes that have `response_ready` set.
    pub fn response_ready_paths(&self) -> Vec<(usize, Vec<usize>)> {
        let mut result = Vec::new();
        for ((file_id, node_id), kiron) in &self.active_kirons {
            if !kiron.response_ready {
                continue;
            }
            let Some(fi) = self.file_index_for_id(*file_id) else {
                continue;
            };
            let Some(file) = self.files.get(fi) else { continue };
            if let Some(path) = duir_core::tree_ops::find_node_path(&file.data, node_id) {
                result.push((fi, path));
            }
        }
        result
    }
}
