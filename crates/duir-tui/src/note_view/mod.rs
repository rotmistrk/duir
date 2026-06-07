//! `NoteView` — simple text editor for item notes, placed in center slot.

use txv_core::prelude::*;

use crate::todo_tree::model::TreePath;

mod highlight;

/// Command emitted when note content changes (payload: `(TreePath, String)`).
pub const CM_NOTE_SAVE: CommandId = txv_core::commands::CM_TXV_MAX + 12;

/// The note editor view.
pub struct NoteView {
    state: ViewState,
    lines: Vec<String>,
    cursor_line: usize,
    cursor_col: usize,
    scroll_offset: usize,
    path: Option<TreePath>,
    dirty: bool,
}

impl NoteView {
    pub fn new() -> Self {
        Self {
            state: ViewState::default(),
            lines: vec![String::new()],
            cursor_line: 0,
            cursor_col: 0,
            scroll_offset: 0,
            path: None,
            dirty: false,
        }
    }

    /// Load note content for a given tree path.
    pub fn load(&mut self, path: TreePath, content: &str) {
        self.save_if_dirty();
        self.path = Some(path);
        self.lines = if content.is_empty() {
            vec![String::new()]
        } else {
            content.lines().map(String::from).collect()
        };
        self.cursor_line = 0;
        self.cursor_col = 0;
        self.scroll_offset = 0;
        self.dirty = false;
        self.state.mark_dirty();
    }

    /// Get the current text content.
    pub fn content(&self) -> String {
        self.lines.join("\n")
    }

    /// Emit save command if dirty.
    fn save_if_dirty(&mut self) {
        if self.dirty
            && let Some(path) = self.path.clone()
        {
            let content = self.content();
            self.state.put_command(CM_NOTE_SAVE, Some(Box::new((path, content))));
            self.dirty = false;
        }
    }

    fn insert_char(&mut self, ch: char) {
        if let Some(line) = self.lines.get_mut(self.cursor_line) {
            let byte_pos = char_to_byte(line, self.cursor_col);
            line.insert(byte_pos, ch);
            self.cursor_col += 1;
            self.dirty = true;
            self.state.mark_dirty();
        }
    }

    fn insert_newline(&mut self) {
        if let Some(line) = self.lines.get_mut(self.cursor_line) {
            let byte_pos = char_to_byte(line, self.cursor_col);
            let rest = line.split_off(byte_pos);
            self.cursor_line += 1;
            self.cursor_col = 0;
            self.lines.insert(self.cursor_line, rest);
            self.dirty = true;
            self.state.mark_dirty();
        }
    }

    fn backspace(&mut self) {
        if self.cursor_col > 0 {
            if let Some(line) = self.lines.get_mut(self.cursor_line) {
                self.cursor_col -= 1;
                let byte_pos = char_to_byte(line, self.cursor_col);
                line.remove(byte_pos);
                self.dirty = true;
                self.state.mark_dirty();
            }
        } else if self.cursor_line > 0 {
            let removed = self.lines.remove(self.cursor_line);
            self.cursor_line -= 1;
            self.cursor_col = self.lines.get(self.cursor_line).map_or(0, |l| l.chars().count());
            if let Some(line) = self.lines.get_mut(self.cursor_line) {
                line.push_str(&removed);
            }
            self.dirty = true;
            self.state.mark_dirty();
        }
    }

    fn ensure_cursor_visible(&mut self) {
        let h = self.state.bounds().h as usize;
        if h == 0 {
            return;
        }
        if self.cursor_line < self.scroll_offset {
            self.scroll_offset = self.cursor_line;
        } else if self.cursor_line >= self.scroll_offset + h {
            self.scroll_offset = self.cursor_line - h + 1;
        }
    }
}

impl View for NoteView {
    delegate_view_state!(state, override { title, select, unselect, draw, handle, cursor });

    fn title(&self) -> &'static str {
        "Note"
    }

    fn draw(&mut self) {
        let b = self.state.bounds();
        let w = b.w as usize;
        let h = b.h as usize;
        if w == 0 || h == 0 {
            return;
        }
        self.ensure_cursor_visible();
        let focused = self.state.is_focused();
        let buf = self.state.buffer_mut();
        buf.fill(' ', Style::default());

        if self.path.is_none() {
            let dim = Style {
                attrs: Attrs {
                    dim: true,
                    ..Attrs::default()
                },
                ..Style::default()
            };
            buf.print(1, 0, "(select an item to edit its note)", dim);
            return;
        }

        for row in 0..h {
            let line_idx = self.scroll_offset + row;
            let Some(line) = self.lines.get(line_idx) else {
                break;
            };
            let base = highlight::line_style(line);
            let style = if focused && line_idx == self.cursor_line {
                Style {
                    attrs: Attrs {
                        underline: true,
                        ..base.attrs
                    },
                    ..base
                }
            } else {
                base
            };
            let display: String = line.chars().take(w).collect();
            buf.print(0, u16::try_from(row).unwrap_or(0), &display, style);
        }
    }

    fn cursor(&self) -> Option<CursorRequest> {
        if !self.state.is_focused() || self.path.is_none() {
            return None;
        }
        let b = self.state.bounds();
        let y = self.cursor_line.saturating_sub(self.scroll_offset);
        if y >= b.h as usize {
            return None;
        }
        Some(CursorRequest {
            x: b.x + u16::try_from(self.cursor_col).unwrap_or(0),
            y: b.y + u16::try_from(y).unwrap_or(0),
            shape: CursorShape::Bar,
        })
    }

    fn handle(&mut self, event: &Event) -> HandleResult {
        let Event::Key(key) = event else {
            return HandleResult::Ignored;
        };
        match key.code {
            KeyCode::Up => {
                if self.cursor_line > 0 {
                    self.cursor_line -= 1;
                    self.clamp_col();
                    self.state.mark_dirty();
                }
                HandleResult::Consumed
            }
            KeyCode::Down => {
                if self.cursor_line + 1 < self.lines.len() {
                    self.cursor_line += 1;
                    self.clamp_col();
                    self.state.mark_dirty();
                }
                HandleResult::Consumed
            }
            KeyCode::Left => {
                if self.cursor_col > 0 {
                    self.cursor_col -= 1;
                    self.state.mark_dirty();
                }
                HandleResult::Consumed
            }
            KeyCode::Right => {
                let len = self.lines.get(self.cursor_line).map_or(0, |l| l.chars().count());
                if self.cursor_col < len {
                    self.cursor_col += 1;
                    self.state.mark_dirty();
                }
                HandleResult::Consumed
            }
            KeyCode::Enter => {
                self.insert_newline();
                HandleResult::Consumed
            }
            KeyCode::Backspace => {
                self.backspace();
                HandleResult::Consumed
            }
            KeyCode::Char(c) if !key.modifiers.ctrl => {
                self.insert_char(c);
                HandleResult::Consumed
            }
            _ => HandleResult::Ignored,
        }
    }

    fn unselect(&mut self) {
        self.save_if_dirty();
        self.state.set_focused(false);
        self.state.mark_dirty();
    }

    fn select(&mut self) {
        self.state.set_focused(true);
        self.state.mark_dirty();
    }
}

impl NoteView {
    fn clamp_col(&mut self) {
        let len = self.lines.get(self.cursor_line).map_or(0, |l| l.chars().count());
        if self.cursor_col > len {
            self.cursor_col = len;
        }
    }
}

fn char_to_byte(s: &str, char_idx: usize) -> usize {
    s.char_indices().nth(char_idx).map_or(s.len(), |(byte_idx, _)| byte_idx)
}
