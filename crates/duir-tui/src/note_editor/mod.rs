mod command;
mod editing;
mod insert;
mod normal;
mod util;
mod visual;

use crossterm::event::KeyEvent;
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
    pub(crate) pending_count: Option<usize>,
    pub(crate) pending_op: Option<char>,
    pub(crate) command_history: Vec<String>,
    pub(crate) history_index: Option<usize>,
    pub(crate) completer: crate::completer::Completer,
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

    pub fn set_block(&mut self, title: ratatui::text::Line<'static>, _focused: bool, zoomed: bool) {
        let mode_str = match self.mode {
            EditorMode::Normal => "NORMAL",
            EditorMode::Insert => "INSERT",
            EditorMode::Visual => "VISUAL",
            EditorMode::Command | EditorMode::Search => "COMMAND",
        };
        self.textarea.set_block(
            Block::default()
                .title(title)
                .title_bottom(format!(" {mode_str} "))
                .borders(if zoomed { Borders::NONE } else { Borders::ALL })
                .border_style(ratatui::style::Style::default().fg(ratatui::style::Color::Rgb(0, 255, 255))),
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

    pub(crate) fn count(&mut self) -> usize {
        self.pending_count.take().unwrap_or(1)
    }

    pub(crate) const fn page_half(&self) -> usize {
        (self.viewport_height as usize) / 2
    }

    pub(crate) fn enter_insert(&mut self) {
        self.mode = EditorMode::Insert;
        self.pending_count = None;
        self.pending_op = None;
        self.textarea
            .set_cursor_style(Style::default().add_modifier(Modifier::REVERSED).fg(Color::Green));
    }

    pub(crate) fn enter_normal(&mut self) {
        self.mode = EditorMode::Normal;
        self.pending_count = None;
        self.pending_op = None;
        self.textarea.cancel_selection();
        self.textarea
            .set_cursor_style(Style::default().add_modifier(Modifier::REVERSED));
    }

    pub fn render(&mut self, frame: &mut ratatui::Frame, area: Rect, highlighter: &crate::syntax::SyntaxHighlighter) {
        self.viewport_height = area.height.saturating_sub(2);
        if self.mode == EditorMode::Insert {
            frame.render_widget(&self.textarea, area);
        } else {
            let content = self.content();
            let (cursor_row, cursor_col) = self.textarea.cursor();
            let block = self.textarea.block().cloned();
            let lines =
                crate::markdown_view::highlight_lines_with_syntax(&content, cursor_row, cursor_col, Some(highlighter));
            #[allow(clippy::cast_possible_truncation)]
            let scroll_offset = cursor_row.saturating_sub(self.viewport_height as usize / 2) as u16;
            let mut paragraph = ratatui::widgets::Paragraph::new(lines).scroll((scroll_offset, 0));
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
            && let Some(url) = util::extract_url(line, col)
        {
            util::open_in_browser(&url);
        }
    }

    pub(crate) fn replace_all_lines(&mut self, lines: Vec<String>, restore_row: usize) {
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

    pub(crate) fn sync_system_clipboard(&self) {
        let text = self.textarea.yank_text();
        if !text.is_empty() {
            crate::clipboard::copy_to_clipboard(&text);
        }
    }

    pub(crate) fn auto_indent(&mut self) {
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
}
