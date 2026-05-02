use duir_core::NodeId;

use super::{App, StatusLevel};

impl App {
    pub fn collapse_current(&mut self) {
        if let Some(row) = self.rows.get(self.cursor) {
            if row.is_file_root {
                return;
            }
            let fi = row.file_index;
            let path = row.path.clone();
            let file_id = self.files[fi].id;
            if let Some(item) = duir_core::tree_ops::get_item_mut(&mut self.files[fi].data, &path)
                && !item.folded
                && !item.items.is_empty()
            {
                // If unlocked encrypted node, re-encrypt and forget password
                if item.unlocked {
                    let node_id = item.id.clone();
                    let key = (file_id, node_id);
                    if let Some(pw) = self.passwords.get(&key)
                        && let Err(e) = duir_core::crypto::encrypt_item(item, pw)
                    {
                        self.set_status(&format!("Encrypt error, node stays unlocked: {e}"), StatusLevel::Error);
                        return;
                    }
                    self.passwords.remove(&key);
                } else {
                    item.folded = true;
                }
                self.rebuild_rows();
                return;
            }
            // If already collapsed or leaf, move to parent
            if path.len() > 1 {
                let parent_path: duir_core::tree_ops::TreePath = path[..path.len() - 1].to_vec();
                if let Some(pos) = self
                    .rows
                    .iter()
                    .position(|r| r.file_index == fi && !r.is_file_root && r.path == parent_path)
                {
                    self.cursor = pos;
                    self.note_scroll = 0;
                }
            } else if let Some(pos) = self.rows.iter().position(|r| r.file_index == fi && r.is_file_root) {
                self.cursor = pos;
                self.note_scroll = 0;
            }
        }
    }

    pub fn expand_current(&mut self) {
        if let Some(row) = self.rows.get(self.cursor) {
            if row.is_file_root {
                return;
            }
            let fi = row.file_index;
            let path = row.path.clone();
            if duir_core::tree_ops::get_item(&self.files[fi].data, &path)
                .is_some_and(duir_core::model::TodoItem::is_locked)
            {
                self.try_expand_encrypted();
                return;
            }
            if let Some(item) = duir_core::tree_ops::get_item_mut(&mut self.files[fi].data, &path)
                && item.folded
                && !item.items.is_empty()
            {
                item.folded = false;
                self.rebuild_rows();
            }
        }
    }

    pub(crate) fn cmd_encrypt(&mut self) {
        self.save_editor();
        if let Some(row) = self.rows.get(self.cursor).cloned() {
            if row.is_file_root {
                "Cannot encrypt file root".clone_into(&mut self.status_message);
                return;
            }
            let fi = row.file_index;
            let item = duir_core::tree_ops::get_item(&self.files[fi].data, &row.path);
            let already_encrypted = item.is_some_and(duir_core::TodoItem::is_encrypted);
            let node_id = item.map_or_else(|| NodeId(String::new()), |it| it.id.clone());
            let file_id = self.files[fi].id;
            let title = if already_encrypted {
                "Change encryption password"
            } else {
                "Encrypt subtree"
            };
            let action = if already_encrypted {
                crate::password::PasswordAction::ChangePassword { file_id, node_id }
            } else {
                crate::password::PasswordAction::Encrypt { file_id, node_id }
            };
            self.password_prompt = Some(crate::password::PasswordPrompt::new(title, action));
        }
    }

    pub(crate) fn cmd_decrypt(&mut self) {
        self.save_editor();
        if let Some(row) = self.rows.get(self.cursor).cloned() {
            if row.is_file_root {
                return;
            }
            let fi = row.file_index;
            if let Some(item) = duir_core::tree_ops::get_item_mut(&mut self.files[fi].data, &row.path) {
                if !item.is_encrypted() {
                    self.set_status("Node is not encrypted", StatusLevel::Warning);
                    return;
                }
                if !item.unlocked {
                    self.set_status("Unlock first: press → and enter password", StatusLevel::Warning);
                    return;
                }
                let node_id = item.id.clone();
                duir_core::crypto::strip_encryption(item);
                self.passwords.remove(&(self.files[fi].id, node_id));
                self.mark_modified(fi, &row.path);
                self.rebuild_rows();
                self.set_status("Encryption removed", StatusLevel::Success);
            }
        }
    }

    /// Handle password prompt result.
    pub fn handle_password_result(&mut self, password: &str, action: crate::password::PasswordAction) {
        match action {
            crate::password::PasswordAction::Encrypt { file_id, node_id } => {
                let Some(fi) = self.file_index_for_id(file_id) else {
                    return;
                };
                let Some(path) = duir_core::tree_ops::find_node_path(&self.files[fi].data, &node_id) else {
                    return;
                };
                if let Some(item) = duir_core::tree_ops::get_item_mut(&mut self.files[fi].data, &path) {
                    match duir_core::crypto::encrypt_item(item, password) {
                        Ok(()) => {
                            self.passwords.insert((file_id, node_id), password.to_owned());
                            self.mark_modified(fi, &path);
                            self.rebuild_rows();
                            self.set_status("Subtree encrypted", StatusLevel::Success);
                        }
                        Err(e) => self.status_message = format!("Encrypt error: {e}"),
                    }
                }
            }
            crate::password::PasswordAction::Decrypt { file_id, node_id } => {
                let Some(fi) = self.file_index_for_id(file_id) else {
                    return;
                };
                let Some(path) = duir_core::tree_ops::find_node_path(&self.files[fi].data, &node_id) else {
                    return;
                };
                if let Some(item) = duir_core::tree_ops::get_item_mut(&mut self.files[fi].data, &path) {
                    match duir_core::crypto::decrypt_item(item, password) {
                        Ok(()) => {
                            self.passwords.insert((file_id, node_id), password.to_owned());
                            self.mark_file_modified(fi);
                            self.rebuild_rows();
                            self.set_status("Subtree unlocked", StatusLevel::Success);
                        }
                        Err(_) => self.set_status("Wrong password", StatusLevel::Error),
                    }
                }
            }
            crate::password::PasswordAction::ChangePassword { file_id, node_id } => {
                self.save_editor();
                let Some(fi) = self.file_index_for_id(file_id) else {
                    return;
                };
                let Some(path) = duir_core::tree_ops::find_node_path(&self.files[fi].data, &node_id) else {
                    return;
                };
                if let Some(item) = duir_core::tree_ops::get_item_mut(&mut self.files[fi].data, &path) {
                    match duir_core::crypto::encrypt_item(item, password) {
                        Ok(()) => {
                            self.passwords.insert((file_id, node_id), password.to_owned());
                            self.mark_modified(fi, &path);
                            self.rebuild_rows();
                            self.set_status("Password changed", StatusLevel::Success);
                        }
                        Err(e) => self.status_message = format!("Encrypt error: {e}"),
                    }
                }
            }
        }
    }

    /// Try to expand an encrypted node — prompts for password.
    pub fn try_expand_encrypted(&mut self) {
        if let Some(row) = self.rows.get(self.cursor).cloned() {
            if row.is_file_root {
                return;
            }
            let fi = row.file_index;
            let item = duir_core::tree_ops::get_item(&self.files[fi].data, &row.path);
            if item.is_some_and(duir_core::model::TodoItem::is_locked) {
                let node_id = item.map_or_else(|| NodeId(String::new()), |it| it.id.clone());
                self.password_prompt = Some(crate::password::PasswordPrompt::new(
                    "Unlock encrypted node",
                    crate::password::PasswordAction::Decrypt {
                        file_id: self.files[fi].id,
                        node_id,
                    },
                ));
            }
        }
    }
}
