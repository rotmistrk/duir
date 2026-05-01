use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::widgets::{Block, Borders};
use tui_textarea::{CursorMove, TextArea};

/// Vim-like mode for the note editor.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EditorMode {
    Normal,
    Insert,
    Visual,
    Command,
}

/// Wraps `tui_textarea::TextArea` with vim-like keybindings.
pub struct NoteEditor<'a> {
    pub textarea: TextArea<'a>,
    pub mode: EditorMode,
    pub command_buf: String,
    pub search_buf: String,
    pub line_numbers: bool,
    pub dirty: bool,
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
        textarea.set_line_number_style(Style::default().fg(Color::DarkGray));
        textarea.set_cursor_style(Style::default().add_modifier(Modifier::REVERSED));

        Self {
            textarea,
            mode: EditorMode::Normal,
            command_buf: String::new(),
            search_buf: String::new(),
            line_numbers: false,
            dirty: false,
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
        let mode_indicator = match self.mode {
            EditorMode::Normal => " NORMAL",
            EditorMode::Insert => " INSERT",
            EditorMode::Visual => " VISUAL",
            EditorMode::Command => " COMMAND",
        };
        let block = Block::default()
            .title(format!("{title} [{mode_indicator}]"))
            .borders(Borders::ALL)
            .border_style(border_style);
        self.textarea.set_block(block);
    }

    /// Handle a key event. Returns true if consumed.
    pub fn handle_key(&mut self, key: KeyEvent) -> bool {
        match self.mode {
            EditorMode::Normal => self.handle_normal(key),
            EditorMode::Insert => self.handle_insert(key),
            EditorMode::Visual => self.handle_visual(key),
            EditorMode::Command => self.handle_command(key),
        }
    }

    #[allow(clippy::too_many_lines)]
    fn handle_normal(&mut self, key: KeyEvent) -> bool {
        match key.code {
            // Mode switches
            KeyCode::Char('i') => {
                self.mode = EditorMode::Insert;
                self.textarea
                    .set_cursor_style(Style::default().add_modifier(Modifier::REVERSED).fg(Color::Green));
                true
            }
            KeyCode::Char('a') => {
                self.mode = EditorMode::Insert;
                self.textarea.move_cursor(CursorMove::Forward);
                self.textarea
                    .set_cursor_style(Style::default().add_modifier(Modifier::REVERSED).fg(Color::Green));
                true
            }
            KeyCode::Char('A') => {
                self.mode = EditorMode::Insert;
                self.textarea.move_cursor(CursorMove::End);
                self.textarea
                    .set_cursor_style(Style::default().add_modifier(Modifier::REVERSED).fg(Color::Green));
                true
            }
            KeyCode::Char('I') => {
                self.mode = EditorMode::Insert;
                self.textarea.move_cursor(CursorMove::Head);
                self.textarea
                    .set_cursor_style(Style::default().add_modifier(Modifier::REVERSED).fg(Color::Green));
                true
            }
            KeyCode::Char('o') => {
                self.textarea.move_cursor(CursorMove::End);
                self.textarea.insert_newline();
                self.mode = EditorMode::Insert;
                self.textarea
                    .set_cursor_style(Style::default().add_modifier(Modifier::REVERSED).fg(Color::Green));
                self.dirty = true;
                true
            }
            KeyCode::Char('O') => {
                self.textarea.move_cursor(CursorMove::Head);
                self.textarea.insert_newline();
                self.textarea.move_cursor(CursorMove::Up);
                self.mode = EditorMode::Insert;
                self.textarea
                    .set_cursor_style(Style::default().add_modifier(Modifier::REVERSED).fg(Color::Green));
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
                self.mode = EditorMode::Command;
                self.command_buf.clear();
                self.search_buf.clear();
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
                // dd = delete line (simplified: single 'd' deletes line)
                self.textarea.move_cursor(CursorMove::Head);
                self.textarea.delete_line_by_end();
                self.textarea.delete_next_char(); // delete the newline
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
            self.mode = EditorMode::Normal;
            self.textarea
                .set_cursor_style(Style::default().add_modifier(Modifier::REVERSED));
        } else {
            // Let textarea handle all insert-mode keys
            self.textarea.input(key);
            self.dirty = true;
        }
        true
    }

    fn handle_visual(&mut self, key: KeyEvent) -> bool {
        match key.code {
            KeyCode::Esc => {
                self.mode = EditorMode::Normal;
                self.textarea.cancel_selection();
                true
            }
            // Navigation extends selection
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
            KeyCode::Char('y') => {
                self.textarea.copy();
                self.textarea.cancel_selection();
                self.mode = EditorMode::Normal;
                true
            }
            KeyCode::Char('d' | 'x') => {
                self.textarea.cut();
                self.mode = EditorMode::Normal;
                self.dirty = true;
                true
            }
            _ => false,
        }
    }

    #[allow(clippy::too_many_lines)]
    fn handle_command(&mut self, key: KeyEvent) -> bool {
        match key.code {
            KeyCode::Esc => {
                self.mode = EditorMode::Normal;
                self.command_buf.clear();
                true
            }
            KeyCode::Enter => {
                let cmd = self.command_buf.clone();
                self.mode = EditorMode::Normal;
                self.command_buf.clear();
                self.execute_editor_command(&cmd);
                true
            }
            KeyCode::Backspace => {
                if self.command_buf.is_empty() {
                    self.mode = EditorMode::Normal;
                } else {
                    self.command_buf.pop();
                }
                true
            }
            KeyCode::Char(c) => {
                self.command_buf.push(c);
                true
            }
            _ => false,
        }
    }

    fn execute_editor_command(&mut self, cmd: &str) {
        let cmd = cmd.trim();
        if let Some(pattern) = cmd.strip_prefix('/') {
            // Search
            self.textarea.set_search_pattern(pattern).ok();
            self.textarea.search_forward(false);
        } else if cmd == "set nu" || cmd == "set number" {
            self.line_numbers = true;
        } else if cmd == "set nonu" || cmd == "set nonumber" {
            self.line_numbers = false;
        }
        // More commands can be added here
    }

    /// Render the editor into the given area.
    pub fn render(&mut self, frame: &mut ratatui::Frame, area: Rect) {
        self.textarea.set_line_number_style(if self.line_numbers {
            Style::default().fg(Color::DarkGray)
        } else {
            Style::default()
        });
        // Only show line numbers if enabled
        if !self.line_numbers {
            // tui-textarea doesn't have a toggle, but we can set width to 0
            // by not setting line number style — it auto-hides
        }
        frame.render_widget(&self.textarea, area);
    }
}
