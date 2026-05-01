use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders};
use tui_textarea::{CursorMove, TextArea};

/// Vim-like mode for the note editor.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EditorMode {
    Normal,
    Insert,
    Visual,
    Command,
    Search,
}

/// Wraps `tui_textarea::TextArea` with vim-like keybindings.
pub struct NoteEditor<'a> {
    pub textarea: TextArea<'a>,
    pub mode: EditorMode,
    pub command_buf: String,
    pub line_numbers: bool,
    pub dirty: bool,
    pub status: String,
    pending_count: Option<usize>,
    pending_op: Option<char>,
    command_history: Vec<String>,
    history_index: Option<usize>,
    completer: crate::completer::Completer,
    pub viewport_height: u16,
}

impl NoteEditor<'_> {
    #[must_use]
    pub fn new(content: &str) -> Self {
        let lines: Vec<String> = if content.is_empty() {
            vec![String::new()]
        } else {
            content.lines().map(String::from).collect()
        };
        let mut textarea = TextArea::new(lines);
        textarea.set_cursor_line_style(Style::default());
        textarea.set_cursor_style(Style::default().add_modifier(Modifier::REVERSED));
        textarea.set_selection_style(Style::default().bg(Color::DarkGray).fg(Color::Yellow));
        textarea.set_search_style(Style::default().bg(Color::Yellow).fg(Color::Black));
        textarea.set_tab_length(4);

        Self {
            textarea,
            mode: EditorMode::Normal,
            command_buf: String::new(),
            line_numbers: false,
            dirty: false,
            status: String::new(),
            pending_count: None,
            pending_op: None,
            command_history: Vec::new(),
            history_index: None,
            completer: crate::completer::Completer::new(crate::completer::EDITOR_COMMANDS),
            viewport_height: 24,
        }
    }

    #[must_use]
    pub fn content(&self) -> String {
        self.textarea.lines().join("\n")
    }

    pub fn set_block(&mut self, title: &str, focused: bool) {
        let border_style = if focused {
            Style::default().add_modifier(Modifier::BOLD)
        } else {
            Style::default()
        };
        let mode_str = match self.mode {
            EditorMode::Normal => "NORMAL",
            EditorMode::Insert => "INSERT",
            EditorMode::Visual => "VISUAL",
            EditorMode::Command | EditorMode::Search => "COMMAND",
        };
        self.textarea.set_block(
            Block::default()
                .title(format!("{title} [{mode_str}]"))
                .borders(Borders::ALL)
                .border_style(border_style),
        );
    }

    #[must_use]
    pub fn status_line(&self) -> Line<'_> {
        match self.mode {
            EditorMode::Command => {
                let mut spans = vec![
                    Span::raw(":"),
                    Span::styled(&self.command_buf, Style::default().add_modifier(Modifier::BOLD)),
                    Span::raw("▏"),
                ];
                if !self.completer.matches.is_empty() {
                    spans.push(Span::raw("  "));
                    for (i, m) in self.completer.matches.iter().enumerate() {
                        let style = if self.completer.selected == Some(i) {
                            Style::default().bg(Color::DarkGray).fg(Color::Yellow)
                        } else {
                            Style::default().fg(Color::DarkGray)
                        };
                        spans.push(Span::styled(format!(" {m} "), style));
                    }
                }
                Line::from(spans)
            }
            EditorMode::Search => Line::from(vec![
                Span::raw("/"),
                Span::styled(&self.command_buf, Style::default().add_modifier(Modifier::BOLD)),
                Span::raw("▏"),
            ]),
            _ if !self.status.is_empty() => Line::styled(&self.status, Style::default().fg(Color::DarkGray)),
            _ => Line::raw(""),
        }
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> bool {
        match self.mode {
            EditorMode::Normal => self.handle_normal(key),
            EditorMode::Insert => self.handle_insert(key),
            EditorMode::Visual => self.handle_visual(key),
            EditorMode::Command | EditorMode::Search => self.handle_command_input(key),
        }
    }

    fn count(&mut self) -> usize {
        self.pending_count.take().unwrap_or(1)
    }

    const fn page_half(&self) -> usize {
        (self.viewport_height as usize) / 2
    }

    fn enter_insert(&mut self) {
        self.mode = EditorMode::Insert;
        self.pending_count = None;
        self.pending_op = None;
        self.textarea
            .set_cursor_style(Style::default().add_modifier(Modifier::REVERSED).fg(Color::Green));
    }

    fn enter_normal(&mut self) {
        self.mode = EditorMode::Normal;
        self.pending_count = None;
        self.pending_op = None;
        self.textarea.cancel_selection();
        self.textarea
            .set_cursor_style(Style::default().add_modifier(Modifier::REVERSED));
    }

    #[allow(clippy::too_many_lines)]
    fn handle_normal(&mut self, key: KeyEvent) -> bool {
        // Ctrl+U / Ctrl+D for half-page scroll (before count/operator handling)
        if key.modifiers.contains(KeyModifiers::CONTROL) {
            return match key.code {
                KeyCode::Char('u') => {
                    let n = self.page_half();
                    for _ in 0..n {
                        self.textarea.move_cursor(CursorMove::Up);
                    }
                    true
                }
                KeyCode::Char('d') => {
                    let n = self.page_half();
                    for _ in 0..n {
                        self.textarea.move_cursor(CursorMove::Down);
                    }
                    true
                }
                KeyCode::Char('r') => {
                    self.textarea.redo();
                    self.dirty = true;
                    true
                }
                _ => false,
            };
        }

        // Accumulate count prefix (digits)
        if let KeyCode::Char(c @ '1'..='9') = key.code
            && self.pending_op.is_none()
        {
            let digit = (c as usize) - ('0' as usize);
            self.pending_count = Some(self.pending_count.unwrap_or(0) * 10 + digit);
            return true;
        }
        if key.code == KeyCode::Char('0') && self.pending_count.is_some() && self.pending_op.is_none() {
            let val = self.pending_count.unwrap_or(0) * 10;
            self.pending_count = Some(val);
            return true;
        }

        // Handle pending operator (d, y)
        if let Some(op) = self.pending_op {
            self.pending_op = None;
            return match (op, key.code) {
                ('d', KeyCode::Char('d')) => {
                    let n = self.count();
                    self.delete_lines(n);
                    true
                }
                ('y', KeyCode::Char('y')) => {
                    let n = self.count();
                    self.yank_lines(n);
                    true
                }
                _ => {
                    self.pending_count = None;
                    false
                }
            };
        }

        match key.code {
            // Operators that wait for second key
            KeyCode::Char('d') => {
                self.pending_op = Some('d');
                true
            }
            KeyCode::Char('y') => {
                self.pending_op = Some('y');
                true
            }

            // Mode switches
            KeyCode::Char('i') => {
                self.enter_insert();
                true
            }
            KeyCode::Char('a') => {
                self.textarea.move_cursor(CursorMove::Forward);
                self.enter_insert();
                true
            }
            KeyCode::Char('A') => {
                self.textarea.move_cursor(CursorMove::End);
                self.enter_insert();
                true
            }
            KeyCode::Char('I') => {
                self.textarea.move_cursor(CursorMove::Head);
                self.enter_insert();
                true
            }
            KeyCode::Char('o') => {
                self.textarea.move_cursor(CursorMove::End);
                self.textarea.insert_newline();
                self.auto_indent();
                self.enter_insert();
                self.dirty = true;
                true
            }
            KeyCode::Char('O') => {
                self.textarea.move_cursor(CursorMove::Head);
                self.textarea.insert_newline();
                self.textarea.move_cursor(CursorMove::Up);
                self.enter_insert();
                self.dirty = true;
                true
            }
            KeyCode::Char('v') => {
                self.mode = EditorMode::Visual;
                self.pending_count = None;
                self.textarea.start_selection();
                true
            }
            KeyCode::Char('V') => {
                // Line-wise visual: select from head of current line
                self.mode = EditorMode::Visual;
                self.pending_count = None;
                self.textarea.move_cursor(CursorMove::Head);
                self.textarea.start_selection();
                self.textarea.move_cursor(CursorMove::End);
                true
            }
            KeyCode::Char(':') => {
                self.mode = EditorMode::Command;
                self.command_buf.clear();
                true
            }
            KeyCode::Char('/') => {
                self.mode = EditorMode::Search;
                self.command_buf.clear();
                true
            }

            // Navigation (with count)
            KeyCode::Char('h') | KeyCode::Left => {
                let n = self.count();
                for _ in 0..n {
                    self.textarea.move_cursor(CursorMove::Back);
                }
                true
            }
            KeyCode::Char('j') | KeyCode::Down => {
                let n = self.count();
                for _ in 0..n {
                    self.textarea.move_cursor(CursorMove::Down);
                }
                true
            }
            KeyCode::Char('k') | KeyCode::Up => {
                let n = self.count();
                for _ in 0..n {
                    self.textarea.move_cursor(CursorMove::Up);
                }
                true
            }
            KeyCode::Char('l') | KeyCode::Right => {
                let n = self.count();
                for _ in 0..n {
                    self.textarea.move_cursor(CursorMove::Forward);
                }
                true
            }
            KeyCode::Char('0') => {
                self.textarea.move_cursor(CursorMove::Head);
                true
            }
            KeyCode::Char('$') => {
                self.textarea.move_cursor(CursorMove::End);
                true
            }
            KeyCode::Char('w') => {
                let n = self.count();
                for _ in 0..n {
                    self.textarea.move_cursor(CursorMove::WordForward);
                }
                true
            }
            KeyCode::Char('b') => {
                let n = self.count();
                for _ in 0..n {
                    self.textarea.move_cursor(CursorMove::WordBack);
                }
                true
            }
            KeyCode::Char('g') => {
                self.textarea.move_cursor(CursorMove::Top);
                true
            }
            KeyCode::Char('G') => {
                self.textarea.move_cursor(CursorMove::Bottom);
                true
            }
            KeyCode::PageUp => {
                let n = self.page_half();
                for _ in 0..n {
                    self.textarea.move_cursor(CursorMove::Up);
                }
                true
            }
            KeyCode::PageDown => {
                let n = self.page_half();
                for _ in 0..n {
                    self.textarea.move_cursor(CursorMove::Down);
                }
                true
            }

            // Single-key editing (with count)
            KeyCode::Char('x') => {
                let n = self.count();
                for _ in 0..n {
                    self.textarea.delete_next_char();
                }
                self.dirty = true;
                true
            }
            KeyCode::Char('u') => {
                self.textarea.undo();
                self.dirty = true;
                true
            }
            KeyCode::Char('>') => {
                let n = self.count();
                self.indent_lines(n);
                true
            }
            KeyCode::Char('<') => {
                let n = self.count();
                self.unindent_lines(n);
                true
            }
            KeyCode::Char('p') => {
                self.textarea.paste();
                self.dirty = true;
                true
            }
            KeyCode::Char('P') => {
                // Paste before (move to line start, paste, then move up)
                self.textarea.move_cursor(CursorMove::Head);
                self.textarea.insert_newline();
                self.textarea.move_cursor(CursorMove::Up);
                self.textarea.paste();
                self.dirty = true;
                true
            }
            KeyCode::Char('n') => {
                self.textarea.search_forward(false);
                true
            }
            KeyCode::Char('N') => {
                self.textarea.search_back(false);
                true
            }
            KeyCode::Enter if key.modifiers.contains(KeyModifiers::SHIFT) => {
                self.open_url_at_cursor();
                true
            }
            _ => {
                self.pending_count = None;
                false
            }
        }
    }

    fn delete_lines(&mut self, n: usize) {
        for _ in 0..n {
            self.textarea.move_cursor(CursorMove::Head);
            self.textarea.start_selection();
            self.textarea.move_cursor(CursorMove::Down);
            self.textarea.cut();
        }
        self.dirty = true;
    }

    fn yank_lines(&mut self, n: usize) {
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

    fn handle_insert(&mut self, key: KeyEvent) -> bool {
        if key.code == KeyCode::Esc {
            self.enter_normal();
        } else if key.code == KeyCode::Enter {
            self.textarea.insert_newline();
            self.auto_indent();
            self.dirty = true;
        } else {
            self.textarea.input(key);
            self.dirty = true;
        }
        true
    }

    fn handle_visual(&mut self, key: KeyEvent) -> bool {
        match key.code {
            KeyCode::Esc => {
                self.enter_normal();
                true
            }
            KeyCode::Char('h') | KeyCode::Left => {
                self.textarea.move_cursor(CursorMove::Back);
                true
            }
            KeyCode::Char('j') | KeyCode::Down => {
                self.textarea.move_cursor(CursorMove::Down);
                true
            }
            KeyCode::Char('k') | KeyCode::Up => {
                self.textarea.move_cursor(CursorMove::Up);
                true
            }
            KeyCode::Char('l') | KeyCode::Right => {
                self.textarea.move_cursor(CursorMove::Forward);
                true
            }
            KeyCode::Char('w') => {
                self.textarea.move_cursor(CursorMove::WordForward);
                true
            }
            KeyCode::Char('b') => {
                self.textarea.move_cursor(CursorMove::WordBack);
                true
            }
            KeyCode::Char('$') => {
                self.textarea.move_cursor(CursorMove::End);
                true
            }
            KeyCode::Char('0') => {
                self.textarea.move_cursor(CursorMove::Head);
                true
            }
            KeyCode::Char('G') => {
                self.textarea.move_cursor(CursorMove::Bottom);
                true
            }
            KeyCode::Char('g') => {
                self.textarea.move_cursor(CursorMove::Top);
                true
            }
            KeyCode::Char('y') => {
                self.textarea.copy();
                self.sync_system_clipboard();
                self.enter_normal();
                "yanked".clone_into(&mut self.status);
                true
            }
            KeyCode::Char('d' | 'x') => {
                self.textarea.cut();
                self.sync_system_clipboard();
                self.enter_normal();
                self.dirty = true;
                true
            }
            KeyCode::Char('>') => {
                self.indent_selection();
                self.enter_normal();
                true
            }
            KeyCode::Char('<') => {
                self.unindent_selection();
                self.enter_normal();
                true
            }
            _ => false,
        }
    }

    fn handle_command_input(&mut self, key: KeyEvent) -> bool {
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
                let entry = self.command_history[idx].clone();
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
                        let entry = self.command_history[idx + 1].clone();
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

    fn execute_search(&mut self, pattern: &str) {
        if pattern.is_empty() {
            self.textarea.set_search_pattern("").ok();
            return;
        }
        self.textarea.set_search_pattern(pattern).ok();
        self.textarea.search_forward(false);
        self.status = format!("/{pattern}");
    }

    fn execute_editor_command(&mut self, cmd: &str) {
        let cmd = cmd.trim();

        // :set commands
        if let Some(rest) = cmd.strip_prefix("set ") {
            self.execute_set(rest.trim());
            return;
        }

        // Standalone !command (no range) — insert output at cursor
        if let Some(shell_cmd) = cmd.strip_prefix('!')
            && !shell_cmd.is_empty()
        {
            self.insert_shell_output(shell_cmd);
            return;
        }

        // Parse range + command
        let total_lines = self.textarea.lines().len();
        let (row, _) = self.textarea.cursor();
        match parse_ex_command(cmd, row, total_lines) {
            Some(ExCommand::Yank { start, end }) => self.ex_yank(start, end),
            Some(ExCommand::Delete { start, end }) => self.ex_delete(start, end),
            Some(ExCommand::Substitute {
                start,
                end,
                pattern,
                replacement,
                flags,
            }) => {
                self.ex_substitute(start, end, &pattern, &replacement, &flags);
            }
            Some(ExCommand::Shell { start, end, command }) => {
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
            if i < new_lines.len() {
                let old = &lines[i];
                let new = if global {
                    re.replace_all(old, replacement).to_string()
                } else {
                    re.replace(old, replacement).to_string()
                };
                if *old != new {
                    count += 1;
                    new_lines[i] = new;
                }
            }
        }

        if count > 0 {
            // Rebuild textarea with new content
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
            // Restore cursor position
            self.textarea.move_cursor(CursorMove::Top);
            for _ in 0..cursor.0.min(self.textarea.lines().len().saturating_sub(1)) {
                self.textarea.move_cursor(CursorMove::Down);
            }
            self.dirty = true;
        }
        self.status = format!("{count} substitution(s)");
    }

    fn insert_shell_output(&mut self, command: &str) {
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
    fn ex_shell(&mut self, start: usize, end: usize, command: &str) {
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

        // Replace the range with output
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

    fn indent_lines(&mut self, count: usize) {
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

    fn unindent_lines(&mut self, count: usize) {
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

    fn indent_selection(&mut self) {
        if let Some(((start_row, _), (end_row, _))) = self.textarea.selection_range() {
            let mut lines = self.textarea.lines().to_vec();
            for line in lines.iter_mut().take(end_row + 1).skip(start_row) {
                *line = format!("    {line}");
            }
            self.replace_all_lines(lines, start_row);
            self.dirty = true;
        }
    }

    fn unindent_selection(&mut self) {
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

    fn replace_all_lines(&mut self, lines: Vec<String>, restore_row: usize) {
        self.textarea = TextArea::new(lines);
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
        for _ in 0..restore_row.min(self.textarea.lines().len().saturating_sub(1)) {
            self.textarea.move_cursor(CursorMove::Down);
        }
    }

    fn sync_system_clipboard(&self) {
        let text = self.textarea.yank_text();
        if !text.is_empty() {
            crate::clipboard::copy_to_clipboard(&text);
        }
    }
    fn auto_indent(&mut self) {
        let (row, _) = self.textarea.cursor();
        if row == 0 {
            return;
        }
        let prev_line = self.textarea.lines().get(row - 1).cloned().unwrap_or_default();
        let indent: String = prev_line.chars().take_while(|c| c.is_whitespace()).collect();
        if !indent.is_empty() {
            self.textarea.insert_str(&indent);
        }
    }

    pub fn render(&mut self, frame: &mut ratatui::Frame, area: Rect) {
        self.viewport_height = area.height.saturating_sub(2);
        if self.mode == EditorMode::Insert {
            frame.render_widget(&self.textarea, area);
        } else {
            let content = self.content();
            let (cursor_row, _) = self.textarea.cursor();
            let block = self.textarea.block().cloned();
            let lines = crate::markdown_view::highlight_lines(&content, cursor_row);
            #[allow(clippy::cast_possible_truncation)]
            let scroll_offset = cursor_row.saturating_sub(self.viewport_height as usize / 2) as u16;
            let mut paragraph = ratatui::widgets::Paragraph::new(lines)
                .wrap(ratatui::widgets::Wrap { trim: false })
                .scroll((scroll_offset, 0));
            if let Some(b) = block {
                paragraph = paragraph.block(b);
            }
            frame.render_widget(paragraph, area);
        }
    }

    /// Try to open URL under cursor.
    pub fn open_url_at_cursor(&self) {
        let (row, col) = self.textarea.cursor();
        if let Some(line) = self.textarea.lines().get(row)
            && let Some(url) = extract_url(line, col)
        {
            open_in_browser(&url);
        }
    }
}

/// Extract a URL from a line at the given column position.
fn extract_url(line: &str, col: usize) -> Option<String> {
    // Find URLs: https://... or http://...
    let mut start = col;
    while start > 0 && !line[..start].ends_with(|c: char| c.is_whitespace() || c == '(' || c == '<') {
        start -= 1;
    }
    let rest = &line[start..];
    // Try to find a URL starting near the cursor
    for prefix in ["https://", "http://"] {
        if let Some(pos) = rest.find(prefix) {
            let url_start = start + pos;
            let url_end = line[url_start..]
                .find(|c: char| c.is_whitespace() || c == ')' || c == '>' || c == '"' || c == '\'')
                .map_or(line.len(), |e| url_start + e);
            if col <= url_end {
                return Some(line[url_start..url_end].to_owned());
            }
        }
    }
    None
}

fn open_in_browser(url: &str) {
    #[cfg(target_os = "macos")]
    let cmd = "open";
    #[cfg(target_os = "linux")]
    let cmd = "xdg-open";
    #[cfg(target_os = "windows")]
    let cmd = "start";

    std::process::Command::new(cmd)
        .arg(url)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .ok();
}
/// Parsed ex-command with resolved line range.
enum ExCommand {
    Yank {
        start: usize,
        end: usize,
    },
    Delete {
        start: usize,
        end: usize,
    },
    Substitute {
        start: usize,
        end: usize,
        pattern: String,
        replacement: String,
        flags: String,
    },
    Shell {
        start: usize,
        end: usize,
        command: String,
    },
}

/// Parse a vim ex-command string like `1,$y`, `.,+5s/foo/bar/g`, `.,.+3!sort`.
/// `cursor_row` is 0-indexed, `total_lines` is the line count.
fn parse_ex_command(cmd: &str, cursor_row: usize, total_lines: usize) -> Option<ExCommand> {
    let cmd = cmd.trim();

    // Split into range part and command part
    // Find where the command starts (first alpha or !)
    let cmd_start = cmd
        .find(|c: char| c.is_ascii_alphabetic() || c == '!' || c == 's')
        .unwrap_or(cmd.len());

    let range_str = cmd[..cmd_start].trim();
    let cmd_part = &cmd[cmd_start..];

    let (start, end) = parse_range(range_str, cursor_row, total_lines)?;

    if cmd_part.starts_with('y') {
        return Some(ExCommand::Yank { start, end });
    }
    if cmd_part.starts_with('d') {
        return Some(ExCommand::Delete { start, end });
    }
    if let Some(rest) = cmd_part.strip_prefix('s') {
        return parse_substitute(rest).map(|(pattern, replacement, flags)| ExCommand::Substitute {
            start,
            end,
            pattern,
            replacement,
            flags,
        });
    }
    cmd_part.strip_prefix('!').map(|rest| ExCommand::Shell {
        start,
        end,
        command: rest.to_owned(),
    })
}

/// Parse a range like `1,$` or `.,.+5` or `%` or empty (current line).
/// Returns 0-indexed (start, end) inclusive.
fn parse_range(range: &str, cursor: usize, total: usize) -> Option<(usize, usize)> {
    if range.is_empty() {
        return Some((cursor, cursor));
    }
    if range == "%" {
        return Some((0, total.saturating_sub(1)));
    }

    let parts: Vec<&str> = range.splitn(2, ',').collect();
    match parts.len() {
        1 => {
            let addr = parse_address(parts[0].trim(), cursor, total)?;
            Some((addr, addr))
        }
        2 => {
            let start = parse_address(parts[0].trim(), cursor, total)?;
            let end = parse_address(parts[1].trim(), cursor, total)?;
            Some((start, end))
        }
        _ => None,
    }
}

/// Parse a single address: `.`, `$`, a number, or `.+N`, `.-N`.
fn parse_address(addr: &str, cursor: usize, total: usize) -> Option<usize> {
    if addr == "." {
        return Some(cursor);
    }
    if addr == "$" {
        return Some(total.saturating_sub(1));
    }
    if let Ok(n) = addr.parse::<usize>() {
        return Some(n.saturating_sub(1)); // vim is 1-indexed
    }
    // Relative: .+N, .-N
    if let Some(rest) = addr.strip_prefix(".+") {
        let offset: usize = rest.parse().ok()?;
        return Some((cursor + offset).min(total.saturating_sub(1)));
    }
    if let Some(rest) = addr.strip_prefix(".-") {
        let offset: usize = rest.parse().ok()?;
        return Some(cursor.saturating_sub(offset));
    }
    // Just +N or -N relative to cursor
    if let Some(rest) = addr.strip_prefix('+') {
        let offset: usize = rest.parse().ok()?;
        return Some((cursor + offset).min(total.saturating_sub(1)));
    }
    if let Some(rest) = addr.strip_prefix('-') {
        let offset: usize = rest.parse().ok()?;
        return Some(cursor.saturating_sub(offset));
    }
    None
}

/// Parse `s` command body: `/pattern/replacement/flags`
fn parse_substitute(s: &str) -> Option<(String, String, String)> {
    if s.is_empty() {
        return None;
    }
    let delim = s.chars().next()?;
    let rest = &s[delim.len_utf8()..];
    let parts: Vec<&str> = rest.splitn(3, delim).collect();
    if parts.len() < 2 {
        return None;
    }
    let pattern = parts[0].to_owned();
    let replacement = parts[1].to_owned();
    let flags = parts.get(2).unwrap_or(&"").to_string();
    Some((pattern, replacement, flags))
}
