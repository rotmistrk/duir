use ratatui::style::{Color, Modifier, Style};
use tui_textarea::{CursorMove, TextArea};

use super::NoteEditor;

impl NoteEditor<'_> {
    pub(crate) fn ex_yank(&mut self, start: usize, end: usize) {
        let lines = self.textarea.lines();
        let end = end.min(lines.len().saturating_sub(1));
        let text: String = (start..=end)
            .filter_map(|i| lines.get(i))
            .cloned()
            .collect::<Vec<_>>()
            .join("\n");
        self.textarea.set_yank_text(&text);
        crate::clipboard::copy_to_clipboard(&text);
        let count = end - start + 1;
        self.status = format!("{count} line(s) yanked");
    }

    pub(crate) fn ex_delete(&mut self, start: usize, end: usize) {
        let mut lines = self.textarea.lines().to_vec();
        let total = lines.len();
        let end = end.min(total.saturating_sub(1));
        if start > end || start >= total {
            return;
        }
        let count = end - start + 1;
        lines.drain(start..=end);
        if lines.is_empty() {
            lines.push(String::new());
        }
        self.replace_all_lines(lines, start.min(total.saturating_sub(count + 1)));
        self.dirty = true;
        self.status = format!("{count} line(s) deleted");
    }

    pub(crate) fn ex_substitute(&mut self, start: usize, end: usize, pattern: &str, replacement: &str, flags: &str) {
        let global = flags.contains('g');
        let re = match regex::Regex::new(pattern) {
            Ok(r) => r,
            Err(e) => {
                self.status = format!("Bad regex: {e}");
                return;
            }
        };

        let lines = self.textarea.lines().to_vec();
        let total = lines.len();
        let end = end.min(total.saturating_sub(1));
        let mut count = 0usize;
        let mut new_lines = lines.clone();

        for i in start..=end {
            if let (Some(old), Some(slot)) = (lines.get(i), new_lines.get_mut(i)) {
                let new = if global {
                    re.replace_all(old, replacement).to_string()
                } else {
                    re.replace(old, replacement).to_string()
                };
                if *old != new {
                    count += 1;
                    *slot = new;
                }
            }
        }

        if count > 0 {
            let cursor = self.textarea.cursor();
            self.textarea = TextArea::new(new_lines);
            self.textarea.set_cursor_line_style(Style::default());
            self.textarea
                .set_cursor_style(Style::default().add_modifier(Modifier::REVERSED));
            self.textarea
                .set_selection_style(Style::default().bg(Color::DarkGray).fg(Color::Yellow));
            self.textarea
                .set_search_style(Style::default().bg(Color::Yellow).fg(Color::Black));
            self.textarea.set_tab_length(4);
            if self.line_numbers {
                self.textarea
                    .set_line_number_style(Style::default().fg(Color::DarkGray));
            }
            self.textarea.move_cursor(CursorMove::Top);
            for _ in 0..cursor.0.min(self.textarea.lines().len().saturating_sub(1)) {
                self.textarea.move_cursor(CursorMove::Down);
            }
            self.dirty = true;
        }
        self.status = format!("{count} substitution(s)");
    }
}
