//! Messages view — scrollable log in the right panel.

use txv_core::prelude::*;
use txv_widgets::TextArea;

/// Messages view showing application log entries.
pub struct MessagesView {
    inner: TextArea,
}

impl MessagesView {
    pub fn new() -> Self {
        let mut ta = TextArea::new();
        ta.line_numbers = false;
        Self { inner: ta }
    }

    #[allow(dead_code)]
    pub fn push(&mut self, msg: &str) {
        self.inner.lines.push(msg.to_owned());
        self.inner.scroll.set_total(self.inner.lines.len());
    }
}

impl View for MessagesView {
    delegate_view!(inner, override {});
}
