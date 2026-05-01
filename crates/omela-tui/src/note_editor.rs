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
        textarea.set_tab_length(4);

        Self {
            textarea,
            mode: EditorMode::Normal,
            command_buf: String::new(),
            line_numbers: false,
            dirty: false,
            status: String::new(),
        }
    }

    /// Get the full text content.
    #[must_use]
    pub fn content(&self) -> String {
        self.textarea.lines().join("\n")
    }

    /// Set the block (border) for rendering.
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
        let block = Block::default()
            .title(format!("{title} [{mode_str}]"))
            .borders(Borders::ALL)
            .border_style(border_style);
        self.textarea.set_block(block);
    }

    /// Get the status/command line to render below the editor.
    #[must_use]
    pub fn status_line(&self) -> Line<'_> {
        match self.mode {
            EditorMode::Command => Line::from(vec![
                Span::raw(":"),
                Span::styled(&self.command_buf, Style::default().add_modifier(Modifier::BOLD)),
                Span::raw("▏"),
            ]),
            EditorMode::Search => Line::from(vec![
                Span::raw("/"),
                Span::styled(&self.command_buf, Style::default().add_modifier(Modifier::BOLD)),
                Span::raw("▏"),
            ]),
            _ => {
                if self.status.is_empty() {
                    Line::raw("")
                } else {
                    Line::styled(&self.status, Style::default().fg(Color::DarkGray))
                }
            }
        }
    }

    /// Handle a key event. Returns true if consumed.
    pub fn handle_key(&mut self, key: KeyEvent) -> bool {
        match self.mode {
            EditorMode::Normal => self.handle_normal(key),
            EditorMode::Insert => self.handle_insert(key),
            EditorMode::Visual => self.handle_visual(key),
            EditorMode::Command | EditorMode::Search => self.handle_command_input(key),
        }
    }

    fn enter_insert(&mut self) {
        self.mode = EditorMode::Insert;
        self.textarea
            .set_cursor_style(Style::default().add_modifier(Modifier::REVERSED).fg(Color::Green));
    }

    fn enter_normal(&mut self) {
        self.mode = EditorMode::Normal;
        self.textarea.cancel_selection();
        self.textarea
            .set_cursor_style(Style::default().add_modifier(Modifier::REVERSED));
    }

    #[allow(clippy::too_many_lines)]
    fn handle_normal(&mut self, key: KeyEvent) -> bool {
        match key.code {
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
                self.textarea.start_selection();
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

            // Navigation
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
            KeyCode::Char('0') => {
                self.textarea.move_cursor(CursorMove::Head);
                true
            }
            KeyCode::Char('$') => {
                self.textarea.move_cursor(CursorMove::End);
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
            KeyCode::Char('g') => {
                self.textarea.move_cursor(CursorMove::Top);
                true
            }
            KeyCode::Char('G') => {
                self.textarea.move_cursor(CursorMove::Bottom);
                true
            }

            // Editing
            KeyCode::Char('x') => {
                self.textarea.delete_next_char();
                self.dirty = true;
                true
            }
            KeyCode::Char('d') => {
                self.textarea.move_cursor(CursorMove::Head);
                self.textarea.delete_line_by_end();
                self.textarea.delete_next_char();
                self.dirty = true;
                true
            }
            KeyCode::Char('u') => {
                self.textarea.undo();
                self.dirty = true;
                true
            }
            KeyCode::Char('r') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.textarea.redo();
                self.dirty = true;
                true
            }
            KeyCode::Char('p') => {
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
            _ => false,
        }
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
            KeyCode::Char('y') => {
                self.textarea.copy();
                self.enter_normal();
                "yanked".clone_into(&mut self.status);
                true
            }
            KeyCode::Char('d' | 'x') => {
                self.textarea.cut();
                self.enter_normal();
                self.dirty = true;
                true
            }
            _ => false,
        }
    }

    fn handle_command_input(&mut self, key: KeyEvent) -> bool {
        match key.code {
            KeyCode::Esc => {
                self.enter_normal();
                true
            }
            KeyCode::Enter => {
                let cmd = self.command_buf.clone();
                let is_search = self.mode == EditorMode::Search;
                self.enter_normal();
                if is_search {
                    self.execute_search(&cmd);
                } else {
                    self.execute_editor_command(&cmd);
                }
                true
            }
            KeyCode::Backspace => {
                if self.command_buf.is_empty() {
                    self.enter_normal();
                } else {
                    self.command_buf.pop();
                }
                true
            }
            KeyCode::Char(c) => {
                self.command_buf.push(c);
                // Live search preview
                if self.mode == EditorMode::Search {
                    self.textarea.set_search_pattern(&self.command_buf).ok();
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
        match cmd {
            "set nu" | "set number" => {
                self.line_numbers = true;
                "line numbers on".clone_into(&mut self.status);
            }
            "set nonu" | "set nonumber" => {
                self.line_numbers = false;
                "line numbers off".clone_into(&mut self.status);
            }
            _ => {
                self.status = format!("Unknown: {cmd}");
            }
        }
    }

    fn auto_indent(&mut self) {
        // Copy leading whitespace from the previous line
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

    /// Render the editor into the given area.
    pub fn render(&mut self, frame: &mut ratatui::Frame, area: Rect) {
        if self.line_numbers {
            self.textarea
                .set_line_number_style(Style::default().fg(Color::DarkGray));
        }
        frame.render_widget(&self.textarea, area);
    }
}
