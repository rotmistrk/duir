use duir_core::NodeId;

use super::{App, FocusState};

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
            let Some(file) = self.files.get_mut(fi) else { return };
            if node_id.0.is_empty() {
                if file.data.note != content {
                    file.data.note = content;
                    self.mark_modified(fi, &[]);
                }
            } else if let Some(path) = duir_core::tree_ops::find_node_path(&file.data, node_id)
                && let Some(item) = duir_core::tree_ops::get_item_mut(&mut file.data, &path)
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
            let node_id = if row.flags.is_file_root() || row.path.is_empty() {
                NodeId(String::new())
            } else {
                self.files
                    .get(row.file_index)
                    .and_then(|f| duir_core::tree_ops::get_item(&f.data, &row.path))
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
                && !row.flags.is_file_root()
            {
                let fi = row.file_index;
                if let Some(file) = self.files.get_mut(fi)
                    && let Some(item) = duir_core::tree_ops::get_item_mut(&mut file.data, &row.path)
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
}
