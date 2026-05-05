use duir_core::stats::compute_stats;
use duir_core::{Completion, TodoItem};

use super::{App, TreeRow, TreeRowFlags};

impl App {
    pub(crate) fn rebuild_rows_raw(&mut self) {
        self.editor_cache.clear();

        self.rows.clear();
        for fi in 0..self.files.len() {
            let Some(file) = self.files.get(fi) else { continue };
            let mut flags = TreeRowFlags::default();
            flags.set_is_file_root(true);
            flags.set_expanded(!file.data.items.is_empty());
            flags.set_has_children(!file.data.items.is_empty());
            self.rows.push(TreeRow {
                path: vec![],
                depth: 0,
                title: file.name.clone(),
                completed: Completion::Open,
                flags,
                stats_text: String::new(),
                file_index: fi,
                file_id: file.id,
                file_source: Some(file.source),
            });
            let items: Vec<(usize, TodoItem)> = self
                .files
                .get(fi)
                .map(|f| {
                    f.data
                        .items
                        .iter()
                        .enumerate()
                        .map(|(i, item)| (i, item.clone()))
                        .collect()
                })
                .unwrap_or_default();
            for (i, item) in &items {
                self.flatten_item(item, &[*i], 1, fi);
            }
        }
        if self.cursor >= self.rows.len() && !self.rows.is_empty() {
            self.cursor = self.rows.len() - 1;
        }
    }

    fn flatten_item(&mut self, item: &TodoItem, path: &[usize], depth: usize, file_index: usize) {
        let stats = compute_stats(item);
        let stats_text = if stats.total_leaves > 0 {
            format!("{}%", stats.percentage)
        } else {
            String::new()
        };

        let expanded = !item.folded && !item.items.is_empty() && !item.is_locked();
        let has_enc_children = item.items.iter().any(duir_core::crypto::has_encrypted_in_subtree);

        let file_id = self.files.get(file_index).map_or(super::FileId(0), |f| f.id);
        let mut flags = TreeRowFlags::default();
        flags.set_important(item.important);
        flags.set_expanded(expanded);
        flags.set_has_children(!item.items.is_empty() || item.is_locked());
        flags.set_encrypted(item.is_encrypted());
        flags.set_locked(item.is_locked());
        flags.set_has_encrypted_children(has_enc_children);
        flags.set_is_kiron(item.is_kiron());
        flags.set_kiro_active(
            item.is_kiron() && {
                let node_id = item.id.clone();
                self.active_kirons.contains_key(&(file_id, node_id))
            },
        );

        self.rows.push(TreeRow {
            path: path.to_vec(),
            depth,
            title: item.title.clone(),
            completed: item.completed.clone(),
            flags,
            stats_text,
            file_index,
            file_id,
            file_source: None,
        });

        if expanded {
            for (i, child) in item.items.iter().enumerate() {
                let mut child_path = path.to_vec();
                child_path.push(i);
                self.flatten_item(child, &child_path, depth + 1, file_index);
            }
        }
    }

    pub(crate) fn navigate_to(&mut self, file_index: usize, path: &[usize]) {
        if let Some(pos) = self
            .rows
            .iter()
            .position(|r| r.file_index == file_index && !r.flags.is_file_root() && r.path == path)
        {
            self.cursor = pos;
            self.note_scroll = 0;
        }
    }

    pub const fn move_up(&mut self) {
        if self.cursor > 0 {
            self.cursor -= 1;
            self.note_scroll = 0;
        }
    }

    pub const fn move_down(&mut self) {
        if self.cursor + 1 < self.rows.len() {
            self.cursor += 1;
            self.note_scroll = 0;
        }
    }
}
