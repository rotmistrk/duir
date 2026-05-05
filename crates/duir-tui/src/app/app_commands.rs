use super::{App, FocusState, StatusLevel, find_available_path, read_file, write_file};

#[allow(clippy::indexing_slicing)] // fi from rebuild_rows, parts guarded by len checks
impl App {
    /// Execute a `:` command. Returns an optional path for file operations.
    pub fn execute_command(&mut self, storage: &dyn duir_core::TodoStorage) {
        let cmd = if let FocusState::Command { ref buffer, .. } = self.state {
            buffer.trim().to_owned()
        } else {
            return;
        };
        self.state = FocusState::Tree;

        let parts: Vec<&str> = cmd.splitn(3, ' ').collect();
        match parts.first().copied().unwrap_or("") {
            "w" => self.save_current(storage),
            "wa" => self.save_all(storage),
            "q" => self.close_current_file(),
            "qa" | "q!" => {
                self.should_quit = true;
            }
            "e" => {
                if let Some(&name) = parts.get(1) {
                    self.add_empty_file(name);
                    self.status_message = format!("New file: {name}");
                } else {
                    "Usage: :e <name>".clone_into(&mut self.status_message);
                }
            }
            "o" => {
                if let Some(&path_str) = parts.get(1) {
                    self.open_file_path(path_str, storage);
                } else {
                    "Usage: :o <path>".clone_into(&mut self.status_message);
                }
            }
            "export" => self.cmd_export(&parts),
            "yank" => self.cmd_yank_tree(),
            "import" => self.cmd_import(&parts),
            "open" => self.cmd_open(&parts, storage),
            "write" => self.cmd_write(&parts, storage),
            "saveas" => self.cmd_saveas(&parts, storage),
            "collapse" => self.cmd_collapse(),
            "expand" => self.cmd_expand(),
            "autosave" => self.cmd_autosave(&parts),
            "init" => {
                let config = duir_core::config::Config::load();
                match config.init_local() {
                    Ok(()) => "Initialized .duir/ in current directory".clone_into(&mut self.status_message),
                    Err(e) => self.status_message = format!("Init error: {e}"),
                }
            }
            "config" => {
                if parts.get(1).copied() == Some("write") {
                    let config = duir_core::config::Config::default();
                    let path = if duir_core::config::Config::has_local() {
                        std::path::PathBuf::from(".duir/config.toml")
                    } else if let Some(d) = dirs::config_dir() {
                        d.join("duir").join("config.toml")
                    } else {
                        std::path::PathBuf::from(".duir/config.toml")
                    };
                    match config.write_to(&path) {
                        Ok(()) => self.status_message = format!("Config written to {}", path.display()),
                        Err(e) => self.status_message = format!("Config error: {e}"),
                    }
                } else {
                    let config = duir_core::config::Config::load();
                    self.status_message = format!(
                        "central={} local={} autosave={}",
                        config.storage.central.display(),
                        config.storage.local.display(),
                        config.editor.autosave,
                    );
                }
            }
            "help" => {
                self.state = FocusState::Help {
                    scroll: 0,
                    search: String::new(),
                };
            }
            "encrypt" => self.cmd_encrypt(),
            "decrypt" => self.cmd_decrypt(),
            "kiron" => self.cmd_kiron(&parts),
            "kiro" => self.cmd_kiro(&parts),
            "kbd" => match parts.get(1).copied() {
                Some("mac") => {
                    self.kbd_mac = true;
                    self.set_status("Keyboard: macOS (⌥)", StatusLevel::Success);
                }
                Some("linux" | "pc") => {
                    self.kbd_mac = false;
                    self.set_status("Keyboard: Linux/PC (Alt)", StatusLevel::Success);
                }
                _ => {
                    let current = if self.kbd_mac { "mac (⌥)" } else { "linux (Alt)" };
                    self.status_message = format!("Keyboard: {current}. Use :kbd mac | :kbd linux");
                }
            },
            "about" => {
                self.state = FocusState::About;
            }
            _ => {
                self.status_message = format!("Unknown command: {cmd}");
            }
        }
    }

    pub(crate) fn close_current_file(&mut self) {
        if let Some(row) = self.current_row().cloned() {
            let fi = row.file_index;
            if self.files[fi].is_modified() {
                "File has unsaved changes. Use :q! to force".clone_into(&mut self.status_message);
                return;
            }
            let closed_id = self.files[fi].id;
            // Clean up active kirons and pending responses for the closed file
            self.active_kirons.retain(|k, _| k.0 != closed_id);
            self.pending_responses
                .retain(|pr| pr.kiron_file_id != closed_id && pr.prompt_file_id != closed_id);
            self.passwords.retain(|k, _| k.0 != closed_id);
            self.files.remove(fi);
            if self.files.is_empty() {
                self.should_quit = true;
            } else {
                self.rebuild_rows();
            }
        }
    }

    pub(crate) fn save_current(&mut self, storage: &dyn duir_core::TodoStorage) {
        if let Some(row) = self.current_row().cloned() {
            let fi = row.file_index;
            let file_id = self.files[fi].id;
            let pw_map: std::collections::HashMap<Vec<usize>, String> = self
                .passwords
                .iter()
                .filter(|((fid, _), _)| *fid == file_id)
                .filter_map(|((_, nid), pw)| {
                    duir_core::tree_ops::find_node_path(&self.files[fi].data, nid).map(|path| (path, pw.clone()))
                })
                .collect();

            let file = &mut self.files[fi];
            let saved = duir_core::crypto::lock_for_save(&mut file.data.items, &pw_map, &[]);
            match saved {
                Ok(saved_state) => {
                    let save_result = storage.save(&file.name, &file.data);
                    duir_core::crypto::restore_after_save(&mut file.data.items, &saved_state);
                    match save_result {
                        Ok(()) => {
                            let name = self.files[fi].name.clone();
                            self.mark_saved(fi);
                            self.status_message = format!("Saved {name}");
                        }
                        Err(e) => self.status_message = format!("Save error: {e}"),
                    }
                }
                Err(e) => self.status_message = format!("Encrypt error on save: {e}"),
            }
        }
    }

    pub fn save_all(&mut self, storage: &dyn duir_core::TodoStorage) {
        let mut errors: Vec<String> = Vec::new();
        for file in &mut self.files {
            if !file.is_modified() {
                continue;
            }
            let pw_map: std::collections::HashMap<Vec<usize>, String> = self
                .passwords
                .iter()
                .filter(|((fid, _), _)| *fid == file.id)
                .filter_map(|((_, nid), pw)| {
                    duir_core::tree_ops::find_node_path(&file.data, nid).map(|path| (path, pw.clone()))
                })
                .collect();

            let saved = duir_core::crypto::lock_for_save(&mut file.data.items, &pw_map, &[]);

            match saved {
                Ok(saved_state) => {
                    match storage.save(&file.name, &file.data) {
                        Ok(()) => file.modified = false,
                        Err(e) => errors.push(format!("{}: {e}", file.name)),
                    }
                    duir_core::crypto::restore_after_save(&mut file.data.items, &saved_state);
                }
                Err(e) => {
                    errors.push(format!("{}: encrypt error: {e}", file.name));
                }
            }
        }
        if errors.is_empty() {
            self.set_status("Saved", StatusLevel::Success);
        } else {
            self.set_status(&format!("Save errors: {}", errors.join("; ")), StatusLevel::Error);
        }
    }

    pub(crate) fn open_file_path(&mut self, path_str: &str, storage: &dyn duir_core::TodoStorage) {
        let path = std::path::Path::new(path_str);
        if path.exists() {
            match duir_core::file_storage::load_path(path) {
                Ok(data) => {
                    let name = path
                        .file_stem()
                        .and_then(|s| s.to_str())
                        .unwrap_or("untitled")
                        .to_owned();
                    self.add_file(name.clone(), data);
                    self.status_message = format!("Opened {name}");
                }
                Err(e) => self.status_message = format!("Error: {e}"),
            }
        } else {
            match storage.load(path_str) {
                Ok(data) => {
                    self.add_file(path_str.to_owned(), data);
                    self.status_message = format!("Opened {path_str}");
                }
                Err(e) => self.status_message = format!("Error: {e}"),
            }
        }
    }

    fn cmd_yank_tree(&mut self) {
        if let Some(item) = self.current_item() {
            let md = duir_core::markdown_export::export_subtree_safe(item, 3);
            let lines = md.lines().count();
            crate::clipboard::copy_to_clipboard(&md);
            self.status_message = format!("Yanked {lines} lines to clipboard (encrypted nodes redacted)");
        } else {
            "No item selected".clone_into(&mut self.status_message);
        }
    }

    pub(crate) fn cmd_export(&mut self, parts: &[&str]) {
        let Some(item) = self.current_item() else {
            "No item selected".clone_into(&mut self.status_message);
            return;
        };

        let path = if let Some(&fname) = parts.get(1) {
            std::path::PathBuf::from(fname)
        } else {
            let slug: String = item
                .title
                .chars()
                .map(|c| {
                    if c.is_alphanumeric() {
                        c.to_ascii_lowercase()
                    } else {
                        '-'
                    }
                })
                .collect();
            let slug = slug.trim_matches('-').to_owned();
            let base = if slug.is_empty() { "export".to_owned() } else { slug };
            find_available_path(&format!("{base}.md"))
        };

        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("md");
        let path_s = path.to_string_lossy().to_string();
        match ext {
            "md" => {
                let md = duir_core::markdown_export::export_subtree(item, 3);
                match write_file(&path_s, md.as_bytes()) {
                    Ok(()) => self.set_status(&format!("Exported to {path_s}"), StatusLevel::Success),
                    Err(e) => self.set_status(&format!("Export error: {e}"), StatusLevel::Error),
                }
            }
            "docx" => match duir_core::docx_export::export_subtree_docx(item) {
                Ok(bytes) => match write_file(&path_s, &bytes) {
                    Ok(()) => self.set_status(&format!("Exported to {path_s}"), StatusLevel::Success),
                    Err(e) => self.set_status(&format!("Write error: {e}"), StatusLevel::Error),
                },
                Err(e) => self.set_status(&format!("DOCX error: {e}"), StatusLevel::Error),
            },
            "pdf" => match duir_core::pdf_export::export_subtree_pdf(item) {
                Ok(bytes) => match write_file(&path_s, &bytes) {
                    Ok(()) => self.set_status(&format!("Exported to {path_s}"), StatusLevel::Success),
                    Err(e) => self.set_status(&format!("Write error: {e}"), StatusLevel::Error),
                },
                Err(e) => self.set_status(&format!("PDF error: {e}"), StatusLevel::Error),
            },
            _ => {
                self.status_message = format!("Unknown format: .{ext} (supported: .md, .docx, .pdf)");
            }
        }
    }

    pub(crate) fn cmd_import(&mut self, parts: &[&str]) {
        let path_str = if parts.len() >= 3 && parts[1] == "md" {
            parts[2]
        } else if parts.len() >= 2 {
            parts[1]
        } else {
            "Usage: :import <file.md|file.docx>".clone_into(&mut self.status_message);
            return;
        };
        let path = std::path::Path::new(path_str);
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("md");

        let parsed = if ext == "docx" {
            match std::fs::File::open(path) {
                Ok(f) => match duir_core::docx_import::import_docx(std::io::BufReader::new(f)) {
                    Ok(p) => p,
                    Err(e) => {
                        self.set_status(&format!("Import error: {e}"), StatusLevel::Error);
                        return;
                    }
                },
                Err(e) => {
                    self.set_status(&format!("Import error: {e}"), StatusLevel::Error);
                    return;
                }
            }
        } else {
            match read_file(path_str) {
                Ok(content) => duir_core::markdown_import::import_markdown(&content),
                Err(e) => {
                    self.set_status(&format!("Import error: {e}"), StatusLevel::Error);
                    return;
                }
            }
        };

        if let Some(row) = self.rows.get(self.cursor).cloned() {
            let fi = row.file_index;
            if row.is_file_root {
                self.files[fi].data.items.extend(parsed.items);
            } else if let Some(item) = duir_core::tree_ops::get_item_mut(&mut self.files[fi].data, &row.path) {
                item.items.extend(parsed.items);
                item.folded = false;
            }
            self.mark_modified(fi, &row.path);
            self.rebuild_rows();
            self.set_status(&format!("Imported {}", path.display()), StatusLevel::Success);
        }
    }

    pub(crate) fn cmd_autosave(&mut self, parts: &[&str]) {
        if parts.get(1).copied() == Some("all") {
            self.autosave_global = !self.autosave_global;
            for file in &mut self.files {
                file.autosave = self.autosave_global;
            }
            let state = if self.autosave_global { "ON" } else { "OFF" };
            self.status_message = format!("Autosave (all): {state}");
        } else if let Some(row) = self.current_row().cloned() {
            let fi = row.file_index;
            self.files[fi].autosave = !self.files[fi].autosave;
            let state = if self.files[fi].autosave { "ON" } else { "OFF" };
            let name = &self.files[fi].name;
            self.status_message = format!("Autosave {name}: {state}");
        }
    }
}
