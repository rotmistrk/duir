//! Shell terminal — PTY-backed shell in the right slot.

use txv_core::prelude::*;
use txv_widgets::PtyTerminal;

/// Create a shell terminal view, returning a fallback on failure.
pub fn new_shell_terminal() -> Box<dyn View> {
    match PtyTerminal::spawn_shell(80, 24) {
        Ok(term) => Box::new(term),
        Err(e) => {
            log::error!("Failed to spawn shell: {e}");
            Box::new(FallbackView::new(&format!("Shell (failed: {e})")))
        }
    }
}

/// Placeholder view when PTY fails.
struct FallbackView {
    state: ViewState,
    message: String,
}

impl FallbackView {
    fn new(msg: &str) -> Self {
        Self {
            state: ViewState::default(),
            message: msg.to_owned(),
        }
    }
}

impl View for FallbackView {
    delegate_view_state!(state, override { draw, handle });

    fn draw(&mut self) {
        let buf = self.state.buffer_mut();
        buf.fill(' ', Style::default());
        buf.print(0, 0, &self.message, Style::default());
    }

    fn handle(&mut self, _event: &Event) -> HandleResult {
        HandleResult::Ignored
    }
}
