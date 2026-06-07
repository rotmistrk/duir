//! Draw logic for `TodoTreeView` — renders tree + inline edit/filter/crypto overlays.

use txv_core::prelude::*;
use txv_widgets::tree_view::TreeData;

use super::TodoTreeView;

impl TodoTreeView {
    pub(super) fn draw_tree(&mut self) {
        let b = self.group.bounds();
        if b.w() == 0 || b.h() == 0 {
            return;
        }
        self.group.buffer_mut().fill(' ', Style::default());

        if self.inner.data().visible_count() == 0 {
            let dim = Style::default().with_attrs(Attrs::default().dim());
            self.group.buffer_mut().print(0, 0, "  (empty — press 'n' to add)", dim);
            return;
        }

        // Draw TreeTableView
        let inner_bounds = Rect::new(0, 0, b.w(), b.h());
        self.inner.state_mut().set_bounds(inner_bounds);
        self.inner.state_mut().set_focused(self.group.is_focused());
        self.inner.draw();
        // Blit inner onto group buffer
        #[allow(unsafe_code, clippy::ref_as_ptr)]
        {
            let buf_ptr = self.group.buffer_mut() as *mut Buffer;
            unsafe { (*buf_ptr).blit(self.inner.state_mut().buffer(), 0, 0) };
        }

        // Draw InputLine child (edit or filter)
        self.position_and_blit_child(b.w(), b.h());

        // Crypto prompt overlay
        if self.is_crypto_active() {
            self.draw_crypto_prompt(b.w(), b.h());
        }
    }

    fn position_and_blit_child(&mut self, w: u16, h: u16) {
        if self.group.child_count() == 0 {
            return;
        }
        let (x, y, cw) = if self.filter_active {
            (1u16, h.saturating_sub(1), w.saturating_sub(1))
        } else if let Some(row) = self.editing_row {
            let scroll_offset = self.inner.scroll_offset();
            if row < scroll_offset || (row - scroll_offset) >= h as usize {
                return;
            }
            let screen_y = u16::try_from(row - scroll_offset).unwrap_or(0);
            let id = self.inner.data().visible_id(row);
            let depth = self.inner.data().depth(id);
            let indent = u16::try_from(depth * 2 + 2).unwrap_or(0);
            (indent, screen_y, w.saturating_sub(indent))
        } else {
            return;
        };
        self.group.set_child_bounds(0, Rect::new(x, y, cw, 1));
        if let Some(child) = self.group.child_mut(0) {
            child.draw();
        }
        #[allow(unsafe_code, clippy::ref_as_ptr)]
        {
            let buf_ptr = self.group.buffer_mut() as *mut Buffer;
            if let Some(child) = self.group.child(0) {
                let (ox, oy) = self.group.child_origin(0);
                unsafe { (*buf_ptr).blit(child.buffer(), ox, oy) };
            }
        }
    }

    fn draw_crypto_prompt(&mut self, w: u16, h: u16) {
        let y = h.saturating_sub(1);
        let style = Style::default().with_attrs(Attrs::default().bold());
        self.group.buffer_mut().hline(0, y, w, ' ', style);
        let mask_len = self.crypto_pending.as_ref().map_or(0, |p| p.passphrase.len());
        let display = format!("Passphrase: {}", "*".repeat(mask_len));
        self.group.buffer_mut().print(0, y, &display, style);
    }
}
