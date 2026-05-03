use crossterm::event::{KeyCode, KeyEvent};
use ratatui::style::{Color, Modifier, Style};
use tui_textarea::{CursorMove, TextArea};

use super::util::parse_ex_command;
use super::{EditorMode, NoteEditor};

impl NoteEditor<'_> {
    pub(crate) fn handle_command_input(&mut self, key: KeyEvent) -> bool {
        match key.code {
            KeyCode::Esc => {
                self.history_index = None;
                self.completer.matches.clear();
                self.enter_normal();
                true
            }
            KeyCode::Enter => {
                let cmd = self.command_buf.clone();
                let is_search = self.mode == EditorMode::Search;
                if !cmd.trim().is_empty() {
                    let entry = if is_search { format!("/{cmd}") } else { cmd.clone() };
                    self.command_history.push(entry);
                }
                self.history_index = None;
                self.completer.matches.clear();
                self.enter_normal();
                if is_search {
                    self.execute_search(&cmd);
                } else {
                    self.execute_editor_command(&cmd);
                }
                true
            }
            KeyCode::Tab => {
                if self.mode == EditorMode::Command {
                    self.completer.update(&self.command_buf);
                    if let Some(completion) = self.completer.next() {
                        self.command_buf = completion.to_owned();
                    }
                }
                true
            }
            KeyCode::BackTab => {
                if self.mode == EditorMode::Command {
                    self.completer.update(&self.command_buf);
                    if let Some(completion) = self.completer.prev() {
                        self.command_buf = completion.to_owned();
                    }
                }
                true
            }
            KeyCode::Up => {
                if self.command_history.is_empty() {
                    return true;
                }
                let idx = self
                    .history_index
                    .map_or(self.command_history.len() - 1, |i| i.saturating_sub(1));
                self.history_index = Some(idx);
                let entry = self.command_history.get(idx).cloned().unwrap_or_default();
                entry
                    .strip_prefix('/')
                    .unwrap_or(&entry)
                    .clone_into(&mut self.command_buf);
                true
            }
            KeyCode::Down => {
                if let Some(idx) = self.history_index {
                    if idx + 1 < self.command_history.len() {
                        self.history_index = Some(idx + 1);
                        let entry = self.command_history.get(idx + 1).cloned().unwrap_or_default();
                        entry
                            .strip_prefix('/')
                            .unwrap_or(&entry)
                            .clone_into(&mut self.command_buf);
                    } else {
                        self.history_index = None;
                        self.command_buf.clear();
                    }
                }
                true
            }
            KeyCode::Backspace => {
                if self.command_buf.is_empty() {
                    self.history_index = None;
                    self.completer.matches.clear();
                    self.enter_normal();
                } else {
                    self.command_buf.pop();
                    if self.mode == EditorMode::Command {
                        self.completer.update(&self.command_buf);
                        self.completer.reset_selection();
                    }
                }
                true
            }
            KeyCode::Char(c) => {
                self.history_index = None;
                self.command_buf.push(c);
                if self.mode == EditorMode::Search {
                    self.textarea.set_search_pattern(&self.command_buf).ok();
                } else {
                    self.completer.update(&self.command_buf);
                    self.completer.reset_selection();
                }
                true
            }
            _ => false,
        }
    }

    pub(crate) fn execute_search(&mut self, pattern: &str) {
        if pattern.is_empty() {
            self.textarea.set_search_pattern("").ok();
            return;
        }
        self.textarea.set_search_pattern(pattern).ok();
        self.textarea.search_forward(false);
        self.status = format!("/{pattern}");
    }

    pub(crate) fn execute_editor_command(&mut self, cmd: &str) {
        let cmd = cmd.trim();

        if let Some(rest) = cmd.strip_prefix("set ") {
            self.execute_set(rest.trim());
            return;
        }

        if let Some(shell_cmd) = cmd.strip_prefix('!')
            && !shell_cmd.is_empty()
        {
            self.insert_shell_output(shell_cmd);
            return;
        }

        let total_lines = self.textarea.lines().len();
        let (row, _) = self.textarea.cursor();
        match parse_ex_command(cmd, row, total_lines) {
            Some(super::util::ExCommand::Yank { start, end }) => self.ex_yank(start, end),
            Some(super::util::ExCommand::Delete { start, end }) => self.ex_delete(start, end),
            Some(super::util::ExCommand::Substitute {
                start,
                end,
                pattern,
                replacement,
                flags,
            }) => {
                self.ex_substitute(start, end, &pattern, &replacement, &flags);
            }
            Some(super::util::ExCommand::Shell { start, end, command }) => {
                self.ex_shell(start, end, &command);
            }
            None => {
                self.status = format!("Unknown: {cmd}");
            }
        }
    }

    fn execute_set(&mut self, arg: &str) {
        match arg {
            "nu" | "num" | "number" => {
                self.line_numbers = true;
                self.textarea
                    .set_line_number_style(Style::default().fg(Color::DarkGray));
                "line numbers on".clone_into(&mut self.status);
            }
            "nonu" | "nonum" | "nonumber" => {
                self.line_numbers = false;
                self.textarea.remove_line_number();
                "line numbers off".clone_into(&mut self.status);
            }
            "li" | "list" | "noli" | "nolist" => {
                "list mode not yet supported".clone_into(&mut self.status);
            }
            _ => self.status = format!("Unknown set: {arg}"),
        }
    }

    fn ex_yank(&mut self, start: usize, end: usize) {
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

    fn ex_delete(&mut self, start: usize, end: usize) {
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

    fn ex_substitute(&mut self, start: usize, end: usize, pattern: &str, replacement: &str, flags: &str) {
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
