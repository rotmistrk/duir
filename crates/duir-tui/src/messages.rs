//! Messages view — scrollable log in the right panel.

use txv_core::prelude::*;
use txv_widgets::TextArea;

/// Messages view showing application log entries.
pub struct MessagesView {
    inner: TextArea,
    lines: Vec<String>,
}

impl MessagesView {
    pub fn new() -> Self {
        let mut ta = TextArea::new();
        ta.show_line_numbers(false);
        Self {
            inner: ta,
            lines: Vec::new(),
        }
    }

    #[allow(dead_code)]
    pub fn push(&mut self, msg: &str) {
        self.lines.push(msg.to_owned());
        self.inner.set_content(&self.lines.join("\n"));
    }
}

impl View for MessagesView {
    delegate_view!(inner, override {});
}
