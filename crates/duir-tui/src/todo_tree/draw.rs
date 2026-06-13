//! Draw logic for `TodoTreeView` — only draws own pixels (filter row).
//! `TreeTableView` (child 0) and `InputLine` (child 1) are rendered by the group pipeline.

use txv_core::prelude::*;
use txv_widgets::tree_view::TreeData;

use super::TodoTreeView;

impl TodoTreeView {
    pub(super) fn draw_tree(&mut self) {
        let b = self.group.bounds();
        let w = b.w();
        let h = b.h();
        if w == 0 || h == 0 {
            return;
        }

        if self.inner().data().visible_count() == 0 {
            let dim = palette().style(StyleId::Dim);
            self.group.buffer_mut().fill(' ', Style::default());
            self.group.buffer_mut().print(0, 0, "  (empty — press 'n' to add)", dim);
            return;
        }

        // Draw filter row if active
        let has_filter = self.filter_active || !self.inner().data().filter_text.is_empty();
        if has_filter {
            let filter_row = h.saturating_sub(1);
            let style = palette().style(StyleId::StatusBar);
            self.group.buffer_mut().hline(0, filter_row, w, ' ', style);
            self.group.buffer_mut().print(0, filter_row, "/", style);
            if !self.filter_active {
                let ft = self.inner().data().filter_text.clone();
                self.group.buffer_mut().print(1, filter_row, &ft, style);
            }
        }

        // Crypto prompt
        if self.crypto_pending.is_some() {
            let y = h.saturating_sub(1);
            let style = Style::default().with_attrs(Attrs::default().bold());
            self.group.buffer_mut().hline(0, y, w, ' ', style);
            let mask_len = self.crypto_pending.as_ref().map_or(0, |p| p.passphrase.len());
            let display = format!("Passphrase: {}", "*".repeat(mask_len));
            self.group.buffer_mut().print(0, y, &display, style);
        }
    }
}
