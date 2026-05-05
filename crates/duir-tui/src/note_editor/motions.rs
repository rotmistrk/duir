use tui_textarea::CursorMove;

use super::NoteEditor;

impl NoteEditor<'_> {
    pub(crate) fn execute_pending_char(&mut self, pending: char, ch: char) -> bool {
        match pending {
            'r' => {
                self.textarea.delete_next_char();
                self.textarea.insert_char(ch);
                self.textarea.move_cursor(CursorMove::Back);
                self.dirty = true;
                true
            }
            'f' | 'F' | 't' | 'T' => {
                self.last_find = Some((pending, ch));
                self.execute_find(pending, ch);
                true
            }
            'g' => {
                if ch == 'g' {
                    if let Some(n) = self.pending_count.take() {
                        self.goto_line(n);
                    } else {
                        self.textarea.move_cursor(CursorMove::Top);
                    }
                    true
                } else {
                    self.pending_count = None;
                    false
                }
            }
            _ => false,
        }
    }

    pub(crate) fn execute_find(&mut self, cmd: char, target: char) {
        let (row, col) = self.textarea.cursor();
        let line = self.textarea.lines().get(row).cloned().unwrap_or_default();
        let chars: Vec<char> = line.chars().collect();

        let found = match cmd {
            'f' => chars
                .iter()
                .enumerate()
                .skip(col + 1)
                .find(|(_, c)| **c == target)
                .map(|(i, _)| i),
            'F' => chars
                .iter()
                .enumerate()
                .take(col)
                .rev()
                .find(|(_, c)| **c == target)
                .map(|(i, _)| i),
            't' => chars
                .iter()
                .enumerate()
                .skip(col + 1)
                .find(|(_, c)| **c == target)
                .map(|(i, _)| i.saturating_sub(1).max(col + 1)),
            'T' => chars
                .iter()
                .enumerate()
                .take(col)
                .rev()
                .find(|(_, c)| **c == target)
                .map(|(i, _)| (i + 1).min(col.saturating_sub(1))),
            _ => None,
        };

        if let Some(target_col) = found {
            self.textarea.move_cursor(CursorMove::Head);
            for _ in 0..target_col {
                self.textarea.move_cursor(CursorMove::Forward);
            }
        }
    }

    pub(crate) fn join_lines(&mut self) {
        let n = self.count();
        for _ in 0..n {
            self.textarea.move_cursor(CursorMove::End);
            self.textarea.delete_next_char();
            let (row, col) = self.textarea.cursor();
            let line = self.textarea.lines().get(row).cloned().unwrap_or_default();
            let after = line.get(col..).unwrap_or_default();
            let ws = after.len() - after.trim_start().len();
            for _ in 0..ws {
                self.textarea.delete_next_char();
            }
            self.textarea.insert_char(' ');
        }
        self.dirty = true;
    }

    pub(crate) fn toggle_case(&mut self) {
        let n = self.count();
        for _ in 0..n {
            let (row, col) = self.textarea.cursor();
            let line = self.textarea.lines().get(row).cloned().unwrap_or_default();
            if let Some(ch) = line.chars().nth(col) {
                let toggled: char = if ch.is_uppercase() {
                    ch.to_lowercase().next().unwrap_or(ch)
                } else {
                    ch.to_uppercase().next().unwrap_or(ch)
                };
                self.textarea.delete_next_char();
                self.textarea.insert_char(toggled);
            } else {
                self.textarea.move_cursor(CursorMove::Forward);
            }
        }
        self.dirty = true;
    }

    pub(crate) fn move_to_first_nonblank(&mut self) {
        self.textarea.move_cursor(CursorMove::Head);
        let (row, _) = self.textarea.cursor();
        let line = self.textarea.lines().get(row).cloned().unwrap_or_default();
        let indent = line.len() - line.trim_start().len();
        for _ in 0..indent {
            self.textarea.move_cursor(CursorMove::Forward);
        }
    }

    pub(crate) fn move_to_word_end(&mut self) {
        self.textarea.move_cursor(CursorMove::Forward);
        self.textarea.move_cursor(CursorMove::WordForward);
        self.textarea.move_cursor(CursorMove::Back);
    }

    pub(crate) fn goto_line(&mut self, n: usize) {
        self.textarea.move_cursor(CursorMove::Top);
        let target = n.saturating_sub(1);
        for _ in 0..target {
            self.textarea.move_cursor(CursorMove::Down);
        }
    }

    pub(crate) fn search_word_under_cursor(&mut self, forward: bool) {
        let (row, col) = self.textarea.cursor();
        let line = self.textarea.lines().get(row).cloned().unwrap_or_default();
        if let Some(word) = word_at(&line, col) {
            let pattern = format!("\\b{word}\\b");
            self.textarea.set_search_pattern(&pattern).ok();
            if forward {
                self.textarea.search_forward(false);
            } else {
                self.textarea.search_back(false);
            }
            self.status = format!("/{pattern}");
        }
    }

    pub(crate) fn delete_lines(&mut self, n: usize) {
        for _ in 0..n {
            self.textarea.move_cursor(CursorMove::Head);
            self.textarea.start_selection();
            self.textarea.move_cursor(CursorMove::Down);
            self.textarea.cut();
        }
        self.dirty = true;
    }

    pub(crate) fn yank_lines(&mut self, n: usize) {
        self.textarea.move_cursor(CursorMove::Head);
        self.textarea.start_selection();
        for _ in 0..n {
            self.textarea.move_cursor(CursorMove::Down);
        }
        self.textarea.copy();
        self.textarea.cancel_selection();
        self.sync_system_clipboard();
        self.status = format!("{n} line(s) yanked");
    }

    pub(crate) const fn page_full(&self) -> usize {
        self.viewport_height.saturating_sub(2) as usize
    }
}

/// Extract the word at the given column position.
fn word_at(line: &str, col: usize) -> Option<String> {
    let chars: Vec<char> = line.chars().collect();
    if col >= chars.len() {
        return None;
    }
    if !chars.get(col).is_some_and(|c| c.is_alphanumeric() || *c == '_') {
        return None;
    }
    let start = chars
        .get(..col)
        .unwrap_or_default()
        .iter()
        .rposition(|c| !c.is_alphanumeric() && *c != '_')
        .map_or(0, |p| p + 1);
    let end = chars
        .get(col..)
        .unwrap_or_default()
        .iter()
        .position(|c| !c.is_alphanumeric() && *c != '_')
        .map_or(chars.len(), |p| col + p);
    Some(chars.get(start..end).unwrap_or_default().iter().collect())
}
