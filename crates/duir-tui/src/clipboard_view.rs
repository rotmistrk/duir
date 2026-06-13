//! Clipboard viewer — shows clipboard ring entries in the right panel.

use txv_core::clipboard_ring::ClipboardHandle;
use txv_core::prelude::*;

/// Clipboard ring viewer.
pub struct ClipboardView {
    state: ViewState,
    clipboard: ClipboardHandle,
}

impl ClipboardView {
    pub fn new(clipboard: ClipboardHandle) -> Self {
        Self {
            state: ViewState::default(),
            clipboard,
        }
    }
}

impl View for ClipboardView {
    delegate_view_state!(state, override { draw, handle });

    fn draw(&mut self) {
        let b = self.state.bounds();
        if b.w() == 0 || b.h() == 0 {
            return;
        }
        let buf = self.state.buffer_mut();
        buf.fill(' ', Style::default());

        let Ok(ring) = self.clipboard.lock() else { return };
        let dim = Style::default().with_attrs(Attrs::default().dim());

        for (i, entry) in ring.entries().iter().enumerate() {
            let y = u16::try_from(i).unwrap_or(0);
            if y >= b.h() {
                break;
            }
            let prefix = if i == 0 { "▸ " } else { "  " };
            let line: String = entry.text().chars().take(b.w() as usize - 2).collect();
            let style = if i == 0 { Style::default() } else { dim };
            buf.print(0, y, prefix, style);
            buf.print(2, y, &line, style);
        }
    }

    fn handle(&mut self, _event: &Event) -> HandleResult {
        HandleResult::Ignored
    }
}
