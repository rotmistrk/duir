use super::{App, StatusLevel};

impl App {
    pub(crate) fn save_current(&mut self, storage: &dyn duir_core::TodoStorage) {
        if let Some(row) = self.current_row().cloned() {
            let fi = row.file_index;
            let Some(file) = self.files.get(fi) else { return };

            if let Some(our_mtime) = file.disk_mtime
                && let Some(disk_mtime) = storage.mtime(&file.name)
                && disk_mtime > our_mtime
            {
                let Some(file) = self.files.get_mut(fi) else { return };
                file.conflicted = true;
                "⚠ File changed on disk. :w! to force, :e to reload, :resolve for conflicts"
                    .clone_into(&mut self.status_message);
                return;
            }

            self.do_save_file(fi, storage);
        }
    }

    pub(crate) fn force_save_current(&mut self, storage: &dyn duir_core::TodoStorage) {
        if let Some(row) = self.current_row().cloned() {
            let fi = row.file_index;
            if let Some(file) = self.files.get_mut(fi) {
                file.conflicted = false;
            }
            self.do_save_file(fi, storage);
        }
    }

    pub(crate) fn do_save_file(&mut self, fi: usize, storage: &dyn duir_core::TodoStorage) {
        let Some(file) = self.files.get(fi) else { return };
        let file_id = file.id;
        let pw_map: std::collections::HashMap<Vec<usize>, String> = self
            .passwords
            .iter()
            .filter(|((fid, _), _)| *fid == file_id)
            .filter_map(|((_, nid), pw)| {
                self.files
                    .get(fi)
                    .and_then(|f| duir_core::tree_ops::find_node_path(&f.data, nid).map(|path| (path, pw.clone())))
            })
            .collect();

        let Some(file) = self.files.get_mut(fi) else { return };
        let saved = duir_core::crypto::lock_for_save(&mut file.data.items, &pw_map, &[]);
        match saved {
            Ok(saved_state) => {
                let save_result = storage.save(&file.name, &file.data);
                duir_core::crypto::restore_after_save(&mut file.data.items, &saved_state);
                match save_result {
                    Ok(()) => {
                        let Some(file) = self.files.get_mut(fi) else { return };
                        let name = file.name.clone();
                        file.disk_mtime = storage.mtime(&name);
                        self.mark_saved(fi);
                        self.status_message = format!("Saved {name}");
                    }
                    Err(e) => self.status_message = format!("Save error: {e}"),
                }
            }
            Err(e) => self.status_message = format!("Encrypt error on save: {e}"),
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
}
