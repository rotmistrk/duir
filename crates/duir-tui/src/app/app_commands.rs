use super::{App, FocusState, StatusLevel};

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
            "w!" => self.force_save_current(storage),
            "wa" => self.save_all(storage),
            "q" => self.close_current_file(),
            "qa" | "q!" => self.flags.set_should_quit(true),
            "e" | "o" | "open" | "write" | "saveas" => self.execute_file_cmd(&parts, storage),
            "export" => self.cmd_export(&parts),
            "yank" => self.cmd_yank_tree(),
            "import" => self.cmd_import(&parts),
            "collapse" => self.cmd_collapse(),
            "expand" => self.cmd_expand(),
            "autosave" => self.cmd_autosave(&parts),
            "init" | "config" => self.execute_config_cmd(&parts),
            "help" => {
                self.state = FocusState::Help {
                    scroll: 0,
                    search: String::new(),
                };
            }
            "encrypt" => self.cmd_encrypt(),
            "decrypt" => self.cmd_decrypt(),
            "files" => self.cmd_files(),
            "resolve" => self.cmd_resolve(storage),
            "kiron" => self.cmd_kiron(&parts),
            "kiro" => self.cmd_kiro(&parts),
            "kbd" => self.execute_kbd_cmd(&parts),
            "about" => self.state = FocusState::About,
            _ => self.status_message = format!("Unknown command: {cmd}"),
        }
    }

    fn execute_file_cmd(&mut self, parts: &[&str], storage: &dyn duir_core::TodoStorage) {
        match parts.first().copied().unwrap_or("") {
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
            "open" => self.cmd_open(parts, storage),
            "write" => self.cmd_write(parts, storage),
            "saveas" => self.cmd_saveas(parts, storage),
            _ => {}
        }
    }

    fn execute_config_cmd(&mut self, parts: &[&str]) {
        match parts.first().copied().unwrap_or("") {
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
            _ => {}
        }
    }

    fn execute_kbd_cmd(&mut self, parts: &[&str]) {
        match parts.get(1).copied() {
            Some("mac") => {
                self.flags.set_kbd_mac(true);
                self.set_status("Keyboard: macOS (⌥)", StatusLevel::Success);
            }
            Some("linux" | "pc") => {
                self.flags.set_kbd_mac(false);
                self.set_status("Keyboard: Linux/PC (Alt)", StatusLevel::Success);
            }
            _ => {
                let current = if self.flags.kbd_mac() {
                    "mac (⌥)"
                } else {
                    "linux (Alt)"
                };
                self.status_message = format!("Keyboard: {current}. Use :kbd mac | :kbd linux");
            }
        }
    }

    pub(crate) fn close_current_file(&mut self) {
        if let Some(row) = self.current_row().cloned() {
            let fi = row.file_index;
            let Some(file) = self.files.get(fi) else { return };
            if file.is_modified() {
                "File has unsaved changes. Use :q! to force".clone_into(&mut self.status_message);
                return;
            }
            let closed_id = file.id;
            self.active_kirons.retain(|k, _| k.0 != closed_id);
            self.pending_responses
                .retain(|pr| pr.kiron_file_id != closed_id && pr.prompt_file_id != closed_id);
            self.passwords.retain(|k, _| k.0 != closed_id);
            self.files.remove(fi);
            if self.files.is_empty() {
                self.flags.set_should_quit(true);
            } else {
                self.rebuild_rows();
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
}
