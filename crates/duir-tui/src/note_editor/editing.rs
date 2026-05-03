use ratatui::style::{Color, Modifier, Style};
use tui_textarea::{CursorMove, TextArea};

use super::NoteEditor;

impl NoteEditor<'_> {
    pub(crate) fn insert_shell_output(&mut self, command: &str) {
        match std::process::Command::new("sh").arg("-c").arg(command).output() {
            Ok(out) => {
                let text = String::from_utf8_lossy(&out.stdout);
                for line in text.lines() {
                    self.textarea.insert_str(line);
                    self.textarea.insert_newline();
                }
                self.dirty = true;
                self.status = format!("!{command}: {} line(s)", text.lines().count());
            }
            Err(e) => self.status = format!("Shell error: {e}"),
        }
    }

    pub(crate) fn ex_shell(&mut self, start: usize, end: usize, command: &str) {
        let lines = self.textarea.lines().to_vec();
        let total = lines.len();
        let end = end.min(total.saturating_sub(1));

        let input: String = (start..=end)
            .filter_map(|i| lines.get(i))
            .cloned()
            .collect::<Vec<_>>()
            .join("\n");

        let output = match std::process::Command::new("sh")
            .arg("-c")
            .arg(command)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
        {
            Ok(mut child) => {
                use std::io::Write;
                if let Some(stdin) = child.stdin.as_mut() {
                    stdin.write_all(input.as_bytes()).ok();
                }
                drop(child.stdin.take());
                match child.wait_with_output() {
                    Ok(out) => String::from_utf8_lossy(&out.stdout).to_string(),
                    Err(e) => {
                        self.status = format!("Shell error: {e}");
                        return;
                    }
                }
            }
            Err(e) => {
                self.status = format!("Shell error: {e}");
                return;
            }
        };

        let mut new_lines: Vec<String> = Vec::new();
        for (i, line) in lines.iter().enumerate() {
            if i == start {
                for out_line in output.lines() {
                    new_lines.push(out_line.to_owned());
                }
            } else if i > end || i < start {
                new_lines.push(line.clone());
            }
        }
        if new_lines.is_empty() {
            new_lines.push(String::new());
        }

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
        for _ in 0..start.min(self.textarea.lines().len().saturating_sub(1)) {
            self.textarea.move_cursor(CursorMove::Down);
        }
        self.dirty = true;
        let out_count = output.lines().count();
        self.status = format!("!{command}: {out_count} line(s)");
    }

    pub(crate) fn indent_lines(&mut self, count: usize) {
        let (row, _) = self.textarea.cursor();
        let total = self.textarea.lines().len();
        let end = (row + count).min(total);
        let mut lines = self.textarea.lines().to_vec();
        for line in lines.iter_mut().take(end).skip(row) {
            *line = format!("    {line}");
        }
        self.replace_all_lines(lines, row);
        self.dirty = true;
    }

    pub(crate) fn unindent_lines(&mut self, count: usize) {
        let (row, _) = self.textarea.cursor();
        let total = self.textarea.lines().len();
        let end = (row + count).min(total);
        let mut lines = self.textarea.lines().to_vec();
        for line in lines.iter_mut().take(end).skip(row) {
            if line.starts_with("    ") {
                *line = line[4..].to_owned();
            } else if line.starts_with('\t') {
                *line = line[1..].to_owned();
            } else {
                let trimmed = line.trim_start();
                let ws = line.len() - trimmed.len();
                if ws > 0 {
                    *line = trimmed.to_owned();
                }
            }
        }
        self.replace_all_lines(lines, row);
        self.dirty = true;
    }

    pub(crate) fn indent_selection(&mut self) {
        if let Some(((start_row, _), (end_row, _))) = self.textarea.selection_range() {
            let mut lines = self.textarea.lines().to_vec();
            for line in lines.iter_mut().take(end_row + 1).skip(start_row) {
                *line = format!("    {line}");
            }
            self.replace_all_lines(lines, start_row);
            self.dirty = true;
        }
    }

    pub(crate) fn unindent_selection(&mut self) {
        if let Some(((start_row, _), (end_row, _))) = self.textarea.selection_range() {
            let mut lines = self.textarea.lines().to_vec();
            for line in lines.iter_mut().take(end_row + 1).skip(start_row) {
                if line.starts_with("    ") {
                    *line = line[4..].to_owned();
                } else if line.starts_with('\t') {
                    *line = line[1..].to_owned();
                } else {
                    let trimmed = line.trim_start();
                    *line = trimmed.to_owned();
                }
            }
            self.replace_all_lines(lines, start_row);
            self.dirty = true;
        }
    }
}
