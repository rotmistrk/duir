use super::{App, StatusLevel, find_available_path, read_file, write_file};

impl App {
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
        let path_str = if parts.len() >= 3 && parts.get(1).copied() == Some("md") {
            if let Some(&p) = parts.get(2) {
                p
            } else {
                return;
            }
        } else if let Some(&p) = parts.get(1) {
            p
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
            let Some(file) = self.files.get_mut(fi) else { return };
            if row.flags.is_file_root() {
                file.data.items.extend(parsed.items);
            } else if let Some(item) = duir_core::tree_ops::get_item_mut(&mut file.data, &row.path) {
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
            let new_val = !self.flags.autosave_global();
            self.flags.set_autosave_global(new_val);
            for file in &mut self.files {
                file.autosave = new_val;
            }
            let state = if new_val { "ON" } else { "OFF" };
            self.status_message = format!("Autosave (all): {state}");
        } else if let Some(row) = self.current_row().cloned() {
            let fi = row.file_index;
            if let Some(file) = self.files.get_mut(fi) {
                file.autosave = !file.autosave;
                let state = if file.autosave { "ON" } else { "OFF" };
                self.status_message = format!("Autosave {}: {state}", file.name);
            }
        }
    }
}
