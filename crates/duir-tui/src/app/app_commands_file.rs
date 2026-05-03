use super::{App, StatusLevel, read_file};

// fi (file_index) is always set by rebuild_rows from 0..self.files.len(), so self.files[fi] is safe.
#[allow(clippy::indexing_slicing)]
impl App {
    pub(super) fn cmd_open(&mut self, parts: &[&str], storage: &dyn duir_core::TodoStorage) {
        let path_str = if parts.len() >= 3 && parts[1] == "md" {
            parts[2]
        } else if let Some(&p) = parts.get(1) {
            p
        } else {
            "Usage: :open <file>".clone_into(&mut self.status_message);
            return;
        };
        let path = std::path::Path::new(path_str);
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
        match ext {
            "md" => match read_file(path_str) {
                Ok(content) => {
                    let parsed = duir_core::markdown_import::import_markdown(&content);
                    let name = path.file_stem().and_then(|s| s.to_str()).unwrap_or("imported");
                    self.add_file(name.to_owned(), parsed);
                    self.set_status(&format!("Opened {name}"), StatusLevel::Success);
                }
                Err(e) => self.set_status(&format!("Open error: {e}"), StatusLevel::Error),
            },
            "docx" => match std::fs::File::open(path) {
                Ok(f) => match duir_core::docx_import::import_docx(std::io::BufReader::new(f)) {
                    Ok(parsed) => {
                        let name = path.file_stem().and_then(|s| s.to_str()).unwrap_or("imported");
                        self.add_file(name.to_owned(), parsed);
                        self.set_status(&format!("Imported {name}.docx"), StatusLevel::Success);
                    }
                    Err(e) => self.set_status(&format!("Import error: {e}"), StatusLevel::Error),
                },
                Err(e) => self.set_status(&format!("Open error: {e}"), StatusLevel::Error),
            },
            "todo" => match read_file(path_str) {
                Ok(content) => match duir_core::legacy_import::import_legacy_todo(&content) {
                    Ok(parsed) => {
                        let name = path.file_stem().and_then(|s| s.to_str()).unwrap_or("imported");
                        self.add_file(name.to_owned(), parsed);
                        self.set_status(&format!("Imported legacy {name}"), StatusLevel::Success);
                    }
                    Err(e) => self.set_status(&format!("Import error: {e}"), StatusLevel::Error),
                },
                Err(e) => self.set_status(&format!("Open error: {e}"), StatusLevel::Error),
            },
            _ => {
                self.open_file_path(path_str, storage);
            }
        }
    }

    pub(super) fn cmd_write(&mut self, parts: &[&str], storage: &dyn duir_core::TodoStorage) {
        if let Some(&name) = parts.get(1) {
            if let Some(row) = self.current_row().cloned() {
                let fi = row.file_index;
                match storage.save(name, &self.files[fi].data) {
                    Ok(()) => self.set_status(&format!("Written to {name}"), StatusLevel::Success),
                    Err(e) => self.set_status(&format!("Write error: {e}"), StatusLevel::Error),
                }
            }
        } else {
            "Usage: :write <name>".clone_into(&mut self.status_message);
        }
    }

    pub(super) fn cmd_saveas(&mut self, parts: &[&str], storage: &dyn duir_core::TodoStorage) {
        if let Some(&name) = parts.get(1) {
            if let Some(row) = self.current_row().cloned() {
                let fi = row.file_index;
                match storage.save(name, &self.files[fi].data) {
                    Ok(()) => {
                        name.clone_into(&mut self.files[fi].name);
                        self.mark_saved(fi);
                        self.rebuild_rows();
                        self.set_status(&format!("Saved as {name}"), StatusLevel::Success);
                    }
                    Err(e) => self.set_status(&format!("Save error: {e}"), StatusLevel::Error),
                }
            }
        } else {
            "Usage: :saveas <name>".clone_into(&mut self.status_message);
        }
    }
}
