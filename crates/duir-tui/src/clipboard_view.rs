//! Clipboard viewer — shows clipboard ring entries in the right panel.

use txv_core::clipboard_ring::ClipboardHandle;
use txv_core::prelude::*;

/// Clipboard ring viewer.
pub struct ClipboardView {
    state: ViewState,
    clipboard: ClipboardHandle,
    selected: usize,
    last_len: usize,
    last_top: String,
}

impl ClipboardView {
    pub fn new(clipboard: ClipboardHandle) -> Self {
        Self {
            state: ViewState::default(),
            clipboard,
            selected: 0,
            last_len: 0,
            last_top: String::new(),
        }
    }
}

impl View for ClipboardView {
    delegate_view_state!(state, override { draw, handle, select, unselect });

    fn select(&mut self) {
        self.state.set_focused(true);
        self.state.mark_dirty();
    }

    fn unselect(&mut self) {
        self.state.set_focused(false);
        self.state.mark_dirty();
    }

    fn draw(&mut self) {
        let b = self.state.bounds();
        if b.w() == 0 || b.h() == 0 {
            return;
        }
        let buf = self.state.buffer_mut();
        buf.fill(' ', Style::default());

        let Ok(ring) = self.clipboard.lock() else { return };
        let w = b.w() as usize;
        let dim = Style::default().with_attrs(Attrs::default().dim());
        let highlight = palette().style(StyleId::StatusBar);

        for (i, entry) in ring.entries().iter().enumerate() {
            let y = u16::try_from(i).unwrap_or(0);
            if y >= b.h() {
                break;
            }
            let text = entry.text();
            let first_line = text.lines().next().unwrap_or("");
            let line_count = text.lines().count();

            let prefix = if i == self.selected { "▸ " } else { "  " };
            let suffix = if line_count > 1 {
                format!(" [{line_count}L]")
            } else {
                String::new()
            };

            let avail = w.saturating_sub(prefix.len() + suffix.len());
            let truncated: String = first_line.chars().take(avail).collect();

            let style = if i == self.selected {
                highlight
            } else if i == 0 {
                Style::default()
            } else {
                dim
            };
            buf.print(0, y, prefix, style);
            buf.print(2, y, &truncated, style);
            if !suffix.is_empty() {
                let sx = u16::try_from(prefix.len() + truncated.len()).unwrap_or(0);
                buf.print(sx, y, &suffix, dim);
            }
        }
    }

    fn handle(&mut self, event: &Event) -> HandleResult {
        if matches!(event, Event::Tick) {
            if let Ok(ring) = self.clipboard.lock() {
                let current_len = ring.len();
                let top = ring.peek().unwrap_or("");
                if current_len != self.last_len || top != self.last_top {
                    self.last_len = current_len;
                    self.last_top = top.to_owned();
                    self.state.mark_dirty();
                }
            }
            return HandleResult::Ignored;
        }
        let Event::Key(key) = event else {
            return HandleResult::Ignored;
        };
        let count = self.clipboard.lock().map_or(0, |r| r.entries().len());
        if count == 0 {
            return HandleResult::Ignored;
        }
        match key.code() {
            KeyCode::Char('j') | KeyCode::Down => {
                if self.selected + 1 < count {
                    self.selected += 1;
                    self.state.mark_dirty();
                }
                HandleResult::Consumed
            }
            KeyCode::Char('k') | KeyCode::Up => {
                self.selected = self.selected.saturating_sub(1);
                self.state.mark_dirty();
                HandleResult::Consumed
            }
            KeyCode::Enter => {
                // Move selected entry to top of ring (editor reads ring on paste)
                if let Ok(mut ring) = self.clipboard.lock() {
                    ring.select(self.selected);
                }
                self.selected = 0;
                self.state.mark_dirty();
                HandleResult::Consumed
            }
            _ => HandleResult::Ignored,
        }
    }
}
