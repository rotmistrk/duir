use duir_core::NodeId;

use super::{App, FocusState};

// fi (file_index) is always set by rebuild_rows from 0..self.files.len(), so self.files[fi] is safe.
#[allow(clippy::indexing_slicing)]
impl App {
    /// Write editor content back to the model.
    pub fn save_editor(&mut self) {
        if let FocusState::Note {
            ref editor,
            file_id,
            ref node_id,
        } = self.state
        {
            let content = editor.content();
            let Some(fi) = self.file_index_for_id(file_id) else {
                return;
            };
            if node_id.0.is_empty() {
                if self.files[fi].data.note != content {
                    self.files[fi].data.note = content;
                    self.mark_modified(fi, &[]);
                }
            } else if let Some(path) = duir_core::tree_ops::find_node_path(&self.files[fi].data, node_id)
                && let Some(item) = duir_core::tree_ops::get_item_mut(&mut self.files[fi].data, &path)
                && item.note != content
            {
                item.note = content;
                self.mark_modified(fi, &path);
            }
        }
    }

    /// Reload the editor from the model.
    pub fn reload_editor(&mut self) {
        let note = if let FocusState::Note {
            file_id, ref node_id, ..
        } = self.state
        {
            let fi = self.file_index_for_id(file_id);
            if node_id.0.is_empty() {
                fi.and_then(|i| self.files.get(i))
                    .map_or(String::new(), |f| f.data.note.clone())
            } else {
                fi.and_then(|i| {
                    let file = self.files.get(i)?;
                    let path = duir_core::tree_ops::find_node_path(&file.data, node_id)?;
                    duir_core::tree_ops::get_item(&file.data, &path).map(|item| item.note.clone())
                })
                .unwrap_or_default()
            }
        } else {
            return;
        };
        if let FocusState::Note { ref mut editor, .. } = self.state {
            **editor = crate::note_editor::NoteEditor::new(&note);
        }
    }

    /// Switch focus to note pane.
    pub fn focus_note(&mut self) {
        self.editor_cache.clear();
        if let Some(row) = self.current_row().cloned() {
            let note = self.current_note();
            let node_id = if row.is_file_root || row.path.is_empty() {
                NodeId(String::new())
            } else {
                duir_core::tree_ops::get_item(&self.files[row.file_index].data, &row.path)
                    .map_or_else(|| NodeId(String::new()), |item| item.id.clone())
            };
            self.state = FocusState::Note {
                editor: Box::new(crate::note_editor::NoteEditor::new(&note)),
                file_id: row.file_id,
                node_id,
            };
        }
    }

    /// Switch focus to tree pane.
    pub fn focus_tree(&mut self) {
        self.save_editor();
        self.state = FocusState::Tree;
    }

    pub fn finish_editing(&mut self) {
        if let FocusState::EditingTitle { ref buffer, .. } = self.state {
            let new_title = buffer.clone();
            if let Some(row) = self.rows.get(self.cursor).cloned()
                && !row.is_file_root
            {
                let fi = row.file_index;
                if let Some(item) = duir_core::tree_ops::get_item_mut(&mut self.files[fi].data, &row.path)
                    && item.title != new_title
                {
                    item.title.clone_from(&new_title);
                    self.mark_modified(fi, &row.path);
                }
            }
            self.state = FocusState::Tree;
            self.rebuild_rows();
        }
    }

    /// Apply the current committed filter text.
    pub fn apply_filter(&mut self) {
        self.rebuild_rows_raw();
        self.reapply_filter();
    }

    pub(crate) fn reapply_filter(&mut self) {
        if self.filter_committed_text.is_empty() {
            return;
        }

        let opts = duir_core::filter::FilterOptions {
            search_notes: true,
            case_sensitive: false,
        };

        let mut match_set: std::collections::HashSet<(usize, Vec<usize>)> = std::collections::HashSet::new();
        for (fi, file) in self.files.iter().enumerate() {
            let matches = duir_core::filter::filter_items(&file.data.items, &self.filter_committed_text, &opts);
            for path in matches {
                match_set.insert((fi, path));
            }
        }

        if self.filter_committed_exclude {
            self.rows
                .retain(|row| row.is_file_root || !match_set.contains(&(row.file_index, row.path.clone())));
        } else {
            self.rows
                .retain(|row| row.is_file_root || match_set.contains(&(row.file_index, row.path.clone())));
        }

        let visible = self.rows.iter().filter(|r| !r.is_file_root).count();
        let mode = if self.filter_committed_exclude {
            "exclude"
        } else {
            "include"
        };
        self.status_message = format!(
            "Filter '{}' ({}): {} visible",
            self.filter_committed_text, mode, visible
        );

        if self.cursor >= self.rows.len() && !self.rows.is_empty() {
            self.cursor = self.rows.len() - 1;
        }
    }

    /// Live filter — called on each keystroke while typing the filter.
    pub fn apply_filter_live(&mut self) {
        let filter_text = if let FocusState::Filter { ref text, .. } = self.state {
            text.clone()
        } else {
            return;
        };

        if filter_text.is_empty() {
            self.rebuild_rows_raw();
            self.status_message.clear();
            return;
        }
        let (text, exclude) = filter_text
            .strip_prefix('!')
            .map_or_else(|| (filter_text.clone(), false), |rest| (rest.to_owned(), true));
        if text.is_empty() {
            self.rebuild_rows_raw();
            return;
        }

        self.rebuild_rows_raw();
        let opts = duir_core::filter::FilterOptions {
            search_notes: true,
            case_sensitive: false,
        };
        let mut match_set: std::collections::HashSet<(usize, Vec<usize>)> = std::collections::HashSet::new();
        for (fi, file) in self.files.iter().enumerate() {
            let matches = duir_core::filter::filter_items(&file.data.items, &text, &opts);
            for path in matches {
                match_set.insert((fi, path));
            }
        }
        if exclude {
            self.rows
                .retain(|row| row.is_file_root || !match_set.contains(&(row.file_index, row.path.clone())));
        } else {
            self.rows
                .retain(|row| row.is_file_root || match_set.contains(&(row.file_index, row.path.clone())));
        }
        let visible = self.rows.iter().filter(|r| !r.is_file_root).count();
        self.status_message = format!("/{filter_text}: {visible} matches");
        if self.cursor >= self.rows.len() && !self.rows.is_empty() {
            self.cursor = self.rows.len() - 1;
        }
    }

    pub(crate) fn cmd_collapse(&mut self) {
        if let Some(row) = self.rows.get(self.cursor).cloned() {
            if row.is_file_root {
                return;
            }
            let fi = row.file_index;
            if let Some(item) = duir_core::tree_ops::get_item_mut(&mut self.files[fi].data, &row.path) {
                if item.items.is_empty() {
                    "No children to collapse".clone_into(&mut self.status_message);
                    return;
                }
                let mut md = String::new();
                for child in &item.items {
                    md.push_str(&duir_core::markdown_export::export_subtree(child, 3));
                }
                if !item.note.is_empty() {
                    item.note.push_str("\n\n");
                }
                item.note.push_str("<!-- duir:collapsed -->\n");
                item.note.push_str(&md);
                item.items.clear();
                self.mark_modified(fi, &row.path);
                self.rebuild_rows();
                self.reload_editor();
                "Children collapsed to note".clone_into(&mut self.status_message);
            }
        }
    }

    pub(crate) fn cmd_expand(&mut self) {
        if let Some(row) = self.rows.get(self.cursor).cloned() {
            if row.is_file_root {
                return;
            }
            let fi = row.file_index;
            if let Some(item) = duir_core::tree_ops::get_item_mut(&mut self.files[fi].data, &row.path) {
                if item.note.trim().is_empty() {
                    "No note to expand".clone_into(&mut self.status_message);
                    return;
                }
                let marker = "<!-- duir:collapsed -->";
                let (keep_note, md_part) = if let Some(pos) = item.note.find(marker) {
                    (
                        item.note[..pos].trim_end().to_owned(),
                        item.note[pos + marker.len()..].to_owned(),
                    )
                } else {
                    (String::new(), item.note.clone())
                };
                let parsed = duir_core::markdown_import::import_markdown(&md_part);
                if parsed.items.is_empty() {
                    "No tree structure found in note".clone_into(&mut self.status_message);
                    return;
                }
                item.items.extend(parsed.items);
                item.note = keep_note;
                item.folded = false;
                self.mark_modified(fi, &row.path);
                self.rebuild_rows();
                self.reload_editor();
                "Note expanded to children".clone_into(&mut self.status_message);
            }
        }
    }
}
