use ratatui::style::{Color, Modifier, Style};

use super::{Cell, TermBuf};

// Grid indexing is bounded by cursor_row < self.rows and cursor_col < self.cols checks.
// params[0] is from vte parser which always provides at least one element per param group.
#[allow(clippy::indexing_slicing)]
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
            self.grid[self.cursor_row][self.cursor_col] = Cell {
                ch: c,
                style: self.current_style,
            };
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
        let p: Vec<u16> = params.iter().map(|s| s[0]).collect();
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
#[allow(clippy::indexing_slicing)]
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
        let row = &mut self.grid[self.cursor_row];
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

    #[allow(clippy::cast_possible_truncation)]
    fn handle_csi(&mut self, p: &[u16], action: char) {
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

    fn handle_sgr(&mut self, params: &[u16]) {
        if params.is_empty() {
            self.current_style = Style::default();
            return;
        }
        let mut i = 0;
        while i < params.len() {
            i += self.apply_sgr_code(params, i);
        }
    }

    /// Apply one SGR code at `params[i]`, return how many params consumed.
    fn apply_sgr_code(&mut self, params: &[u16], i: usize) -> usize {
        match params[i] {
            0 => self.current_style = Style::default(),
            1 => self.current_style = self.current_style.add_modifier(Modifier::BOLD),
            3 => {
                self.current_style = self.current_style.add_modifier(Modifier::ITALIC);
            }
            4 => {
                self.current_style = self.current_style.add_modifier(Modifier::UNDERLINED);
            }
            7 => {
                self.current_style = self.current_style.add_modifier(Modifier::REVERSED);
            }
            22 => {
                self.current_style = self.current_style.remove_modifier(Modifier::BOLD);
            }
            23 => {
                self.current_style = self.current_style.remove_modifier(Modifier::ITALIC);
            }
            24 => {
                self.current_style = self.current_style.remove_modifier(Modifier::UNDERLINED);
            }
            27 => {
                self.current_style = self.current_style.remove_modifier(Modifier::REVERSED);
            }
            30..=37 => {
                self.current_style = self.current_style.fg(ansi_color(params[i] - 30));
            }
            38 => {
                let mut j = i + 1;
                if let Some(c) = parse_extended_color(params, &mut j) {
                    self.current_style = self.current_style.fg(c);
                }
                return j - i;
            }
            39 => self.current_style = self.current_style.fg(Color::Reset),
            40..=47 => {
                self.current_style = self.current_style.bg(ansi_color(params[i] - 40));
            }
            48 => {
                let mut j = i + 1;
                if let Some(c) = parse_extended_color(params, &mut j) {
                    self.current_style = self.current_style.bg(c);
                }
                return j - i;
            }
            49 => self.current_style = self.current_style.bg(Color::Reset),
            90..=97 => {
                self.current_style = self.current_style.fg(ansi_bright_color(params[i] - 90));
            }
            100..=107 => {
                self.current_style = self.current_style.bg(ansi_bright_color(params[i] - 100));
            }
            _ => {}
        }
        1
    }
}

const fn ansi_color(n: u16) -> Color {
    match n {
        0 => Color::Black,
        1 => Color::Red,
        2 => Color::Green,
        3 => Color::Yellow,
        4 => Color::Blue,
        5 => Color::Magenta,
        6 => Color::Cyan,
        _ => Color::White,
    }
}

const fn ansi_bright_color(n: u16) -> Color {
    match n {
        0 => Color::DarkGray,
        1 => Color::LightRed,
        2 => Color::LightGreen,
        3 => Color::LightYellow,
        4 => Color::LightBlue,
        5 => Color::LightMagenta,
        6 => Color::LightCyan,
        _ => Color::White,
    }
}

#[allow(clippy::cast_possible_truncation)]
// params[*i] accesses are guarded by `*i >= params.len()` / `*i + 2 < params.len()` checks.
#[allow(clippy::indexing_slicing)]
fn parse_extended_color(params: &[u16], i: &mut usize) -> Option<Color> {
    if *i >= params.len() {
        return None;
    }
    match params[*i] {
        5 => {
            // 256-color: 38;5;N
            *i += 1;
            if *i < params.len() {
                let n = params[*i];
                *i += 1;
                Some(Color::Indexed(n as u8))
            } else {
                None
            }
        }
        2 => {
            // RGB: 38;2;R;G;B
            *i += 1;
            if *i + 2 < params.len() {
                let r = params[*i] as u8;
                let g = params[*i + 1] as u8;
                let b = params[*i + 2] as u8;
                *i += 3;
                Some(Color::Rgb(r, g, b))
            } else {
                None
            }
        }
        _ => None,
    }
}
