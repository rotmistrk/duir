use super::{Cell, TermBuf};

// Grid indexing is bounded by cursor_row < self.rows and cursor_col < self.cols checks.
// params[0] is from vte parser which always provides at least one element per param group.
impl vte::Perform for TermBuf {
    fn print(&mut self, c: char) {
        if self.cursor_col >= self.cols {
            self.cursor_col = 0;
            self.cursor_row += 1;
            if self.cursor_row >= self.rows {
                self.scroll_screen_up();
                self.cursor_row = self.rows - 1;
            }
        }
        if self.cursor_row < self.rows && self.cursor_col < self.cols {
            if let Some(row) = self.grid.get_mut(self.cursor_row)
                && let Some(cell) = row.get_mut(self.cursor_col)
            {
                *cell = Cell {
                    ch: c,
                    style: self.current_style,
                };
            }
            self.cursor_col += 1;
        }
    }

    fn execute(&mut self, byte: u8) {
        match byte {
            b'\n' => {
                self.cursor_row += 1;
                if self.cursor_row >= self.rows {
                    self.scroll_screen_up();
                    self.cursor_row = self.rows - 1;
                }
            }
            b'\r' => self.cursor_col = 0,
            b'\t' => {
                let next_tab = (self.cursor_col / 8 + 1) * 8;
                self.cursor_col = next_tab.min(self.cols - 1);
            }
            8 => {
                // Backspace
                self.cursor_col = self.cursor_col.saturating_sub(1);
            }
            _ => {}
        }
    }

    fn csi_dispatch(&mut self, params: &vte::Params, intermediates: &[u8], _ignore: bool, action: char) {
        let p: Vec<u16> = params.iter().map(|s| s.first().copied().unwrap_or(0)).collect();
        if intermediates == [b'?'] {
            self.handle_dec_mode(&p, action);
        } else {
            self.handle_csi(&p, action);
        }
    }

    fn hook(&mut self, _params: &vte::Params, _intermediates: &[u8], _ignore: bool, _action: char) {}
    fn put(&mut self, _byte: u8) {}
    fn unhook(&mut self) {}
    fn osc_dispatch(&mut self, _params: &[&[u8]], _bell_terminated: bool) {}
    fn esc_dispatch(&mut self, _intermediates: &[u8], _ignore: bool, byte: u8) {
        match byte {
            b'7' => self.saved_cursor = (self.cursor_row, self.cursor_col),
            b'8' => {
                self.cursor_row = self.saved_cursor.0.min(self.rows - 1);
                self.cursor_col = self.saved_cursor.1.min(self.cols - 1);
            }
            _ => {}
        }
    }
}

// Grid/params indexing is bounded by cursor bounds checks and loop invariants.
impl TermBuf {
    const fn cursor_up(&mut self, n: usize) {
        self.cursor_row = self.cursor_row.saturating_sub(n);
    }

    fn cursor_down(&mut self, n: usize) {
        self.cursor_row = (self.cursor_row + n).min(self.rows - 1);
    }

    fn cursor_forward(&mut self, n: usize) {
        self.cursor_col = (self.cursor_col + n).min(self.cols - 1);
    }

    const fn cursor_back(&mut self, n: usize) {
        self.cursor_col = self.cursor_col.saturating_sub(n);
    }

    fn erase_display(&mut self, mode: u16) {
        match mode {
            0 => {
                // Clear from cursor to end
                self.erase_line(0);
                for r in (self.cursor_row + 1)..self.rows {
                    self.clear_row(r);
                }
            }
            1 => {
                // Clear from start to cursor
                for r in 0..self.cursor_row {
                    self.clear_row(r);
                }
            }
            2 | 3 => {
                // Clear entire screen
                for r in 0..self.rows {
                    self.clear_row(r);
                }
                self.cursor_row = 0;
                self.cursor_col = 0;
            }
            _ => {}
        }
    }

    fn erase_line(&mut self, mode: u16) {
        if self.cursor_row >= self.rows {
            return;
        }
        let Some(row) = self.grid.get_mut(self.cursor_row) else {
            return;
        };
        match mode {
            0 => {
                for c in row.iter_mut().skip(self.cursor_col) {
                    *c = Cell::default();
                }
            }
            1 => {
                for c in row.iter_mut().take(self.cursor_col + 1) {
                    *c = Cell::default();
                }
            }
            2 => {
                for c in row.iter_mut() {
                    *c = Cell::default();
                }
            }
            _ => {}
        }
    }

    fn handle_dec_mode(&mut self, p: &[u16], action: char) {
        let mode = p.first().copied().unwrap_or(0);
        match (mode, action) {
            (25, 'h') => self.cursor_visible = true,
            (25, 'l') => self.cursor_visible = false,
            _ => {}
        }
    }

    pub(crate) fn handle_csi(&mut self, p: &[u16], action: char) {
        match action {
            'm' => self.handle_sgr(p),
            'A' => self.cursor_up(p.first().copied().unwrap_or(1).max(1) as usize),
            'B' => self.cursor_down(p.first().copied().unwrap_or(1).max(1) as usize),
            'C' => {
                self.cursor_forward(p.first().copied().unwrap_or(1).max(1) as usize);
            }
            'D' => self.cursor_back(p.first().copied().unwrap_or(1).max(1) as usize),
            'H' | 'f' => {
                let row = p.first().copied().unwrap_or(1) as usize;
                let col = p.get(1).copied().unwrap_or(1) as usize;
                self.cursor_row = row.saturating_sub(1).min(self.rows - 1);
                self.cursor_col = col.saturating_sub(1).min(self.cols - 1);
            }
            'J' => self.erase_display(p.first().copied().unwrap_or(0)),
            'K' => self.erase_line(p.first().copied().unwrap_or(0)),
            'G' => {
                let col = p.first().copied().unwrap_or(1) as usize;
                self.cursor_col = col.saturating_sub(1).min(self.cols - 1);
            }
            'n' if p.first().copied() == Some(6) => {
                let resp = format!("\x1b[{};{}R", self.cursor_row + 1, self.cursor_col + 1);
                self.responses.push(resp.into_bytes());
            }
            's' => self.saved_cursor = (self.cursor_row, self.cursor_col),
            'u' => {
                self.cursor_row = self.saved_cursor.0.min(self.rows - 1);
                self.cursor_col = self.saved_cursor.1.min(self.cols - 1);
            }
            'd' => {
                let row = p.first().copied().unwrap_or(1) as usize;
                self.cursor_row = row.saturating_sub(1).min(self.rows - 1);
            }
            _ => {}
        }
    }
}
