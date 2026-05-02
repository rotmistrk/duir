// Virtual terminal buffer: grid of styled cells driven by vte parser.

use ratatui::style::{Color, Modifier, Style};

/// A single cell in the terminal grid.
#[derive(Debug, Clone)]
pub struct Cell {
    pub ch: char,
    pub style: Style,
}

impl Default for Cell {
    fn default() -> Self {
        Self {
            ch: ' ',
            style: Style::default(),
        }
    }
}

/// Virtual terminal screen buffer with scrollback.
pub struct TermBuf {
    cols: usize,
    rows: usize,
    /// Active screen grid (rows x cols).
    grid: Vec<Vec<Cell>>,
    /// Scrollback history (oldest first).
    scrollback: Vec<Vec<Cell>>,
    /// Max scrollback lines.
    max_scrollback: usize,
    /// Cursor position.
    cursor_row: usize,
    cursor_col: usize,
    /// Current style for new characters.
    current_style: Style,
    /// Scroll offset (0 = showing live screen).
    pub scroll_offset: usize,
    /// Whether cursor should be visible.
    pub cursor_visible: bool,
    /// Saved cursor position.
    saved_cursor: (usize, usize),
    /// Responses to send back to PTY (e.g. cursor position report).
    pub responses: Vec<Vec<u8>>,
}

impl TermBuf {
    pub fn new(cols: usize, rows: usize) -> Self {
        let grid = vec![vec![Cell::default(); cols]; rows];
        Self {
            cols,
            rows,
            grid,
            scrollback: Vec::new(),
            max_scrollback: 10_000,
            cursor_row: 0,
            cursor_col: 0,
            current_style: Style::default(),
            scroll_offset: 0,
            cursor_visible: true,
            saved_cursor: (0, 0),
            responses: Vec::new(),
        }
    }

    pub const fn cols(&self) -> usize {
        self.cols
    }
    pub const fn rows(&self) -> usize {
        self.rows
    }
    pub const fn cursor(&self) -> (usize, usize) {
        (self.cursor_row, self.cursor_col)
    }

    /// Resize the buffer. Preserves content where possible.
    pub fn resize(&mut self, cols: usize, rows: usize) {
        let mut new_grid = vec![vec![Cell::default(); cols]; rows];
        let copy_rows = self.rows.min(rows);
        let copy_cols = self.cols.min(cols);
        for (r, new_row) in new_grid.iter_mut().enumerate().take(copy_rows) {
            new_row[..copy_cols].clone_from_slice(&self.grid[r][..copy_cols]);
        }
        self.grid = new_grid;
        self.cols = cols;
        self.rows = rows;
        self.cursor_row = self.cursor_row.min(rows.saturating_sub(1));
        self.cursor_col = self.cursor_col.min(cols.saturating_sub(1));
    }

    /// Get a row for rendering. Negative offsets read from scrollback.
    pub fn visible_row(&self, screen_row: usize) -> &[Cell] {
        if self.scroll_offset == 0 {
            return &self.grid[screen_row.min(self.rows - 1)];
        }
        let total = self.scrollback.len() + self.rows;
        let start = total.saturating_sub(self.rows + self.scroll_offset);
        let idx = start + screen_row;
        if idx < self.scrollback.len() {
            &self.scrollback[idx]
        } else {
            let grid_idx = idx - self.scrollback.len();
            &self.grid[grid_idx.min(self.rows - 1)]
        }
    }

    pub const fn total_lines(&self) -> usize {
        self.scrollback.len() + self.rows
    }

    pub fn scroll_up(&mut self, n: usize) {
        let max = self.scrollback.len();
        self.scroll_offset = (self.scroll_offset + n).min(max);
    }

    pub const fn scroll_down(&mut self, n: usize) {
        self.scroll_offset = self.scroll_offset.saturating_sub(n);
    }

    pub const fn scroll_to_bottom(&mut self) {
        self.scroll_offset = 0;
    }

    /// Feed raw bytes from PTY through the vte parser.
    pub fn process(&mut self, data: &[u8]) {
        // Auto-snap to bottom on new output
        self.scroll_offset = 0;
        let mut parser = PARSER.take();
        for byte in data {
            parser.advance(self, *byte);
        }
        PARSER.set(parser);
    }

    fn scroll_screen_up(&mut self) {
        if !self.grid.is_empty() {
            let row = self.grid.remove(0);
            self.scrollback.push(row);
            if self.scrollback.len() > self.max_scrollback {
                self.scrollback.remove(0);
            }
            self.grid.push(vec![Cell::default(); self.cols]);
        }
    }

    fn clear_row(&mut self, row: usize) {
        if row < self.rows {
            self.grid[row] = vec![Cell::default(); self.cols];
        }
    }
}

// Thread-local vte parser (avoids allocation per process() call)
thread_local! {
    static PARSER: std::cell::Cell<vte::Parser> =
        std::cell::Cell::new(vte::Parser::new());
}

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

/// Extract all text (scrollback + grid) from a `TermBuf` as a plain string.
pub fn extract_text(tb: &TermBuf) -> String {
    let lines = collect_all_lines(tb);
    trim_trailing_empty(lines).join("\n")
}

/// Extract text after the last prompt-like line.
/// Heuristic: scans backward for a line starting with `> `, `❯ `, `$ `,
/// or `% ` (common CLI prompts). Returns everything after that line.
/// Falls back to full content if no prompt is found.
pub fn extract_last_output(tb: &TermBuf) -> String {
    let lines = collect_all_lines(tb);
    let trimmed = trim_trailing_empty(lines);
    // Scan backward for the last prompt line.
    for i in (0..trimmed.len()).rev() {
        if is_prompt_line(&trimmed[i]) {
            let output = &trimmed[i + 1..];
            return output.join("\n");
        }
    }
    trimmed.join("\n")
}

fn is_prompt_line(line: &str) -> bool {
    let s = line.trim_start();
    s.starts_with("> ") || s.starts_with("❯ ") || s.starts_with("$ ") || s.starts_with("% ")
}

fn collect_all_lines(tb: &TermBuf) -> Vec<String> {
    let mut lines = Vec::with_capacity(tb.scrollback.len() + tb.rows);
    for row in &tb.scrollback {
        lines.push(row_to_string(row));
    }
    for row in &tb.grid {
        lines.push(row_to_string(row));
    }
    lines
}

fn trim_trailing_empty(mut lines: Vec<String>) -> Vec<String> {
    while lines.last().is_some_and(String::is_empty) {
        lines.pop();
    }
    lines
}

fn row_to_string(row: &[Cell]) -> String {
    let s: String = row.iter().map(|c| c.ch).collect();
    s.trim_end().to_string()
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use ratatui::style::Color;

    use super::*;

    #[test]
    fn termbuf_cursor_positioning() {
        let mut tb = TermBuf::new(80, 24);
        tb.process(b"\x1b[5;10Hhello");
        let row = &tb.grid[4];
        let text: String = row[9..14].iter().map(|c| c.ch).collect();
        assert_eq!(text, "hello");
    }

    #[test]
    fn termbuf_erase_line() {
        let mut tb = TermBuf::new(80, 24);
        tb.process(b"abcdef");
        tb.process(b"\x1b[1G\x1b[K");
        let row = &tb.grid[0];
        assert!(
            row.iter().all(|c| c.ch == ' '),
            "Row 0 should be all spaces after erase"
        );
    }

    #[test]
    fn termbuf_colors() {
        let mut tb = TermBuf::new(80, 24);
        tb.process(b"\x1b[31mred\x1b[0m");
        let cell = &tb.grid[0][0];
        assert_eq!(cell.ch, 'r');
        assert_eq!(cell.style.fg, Some(Color::Red));
    }

    #[test]
    fn termbuf_visible_row_consistency() {
        let mut tb = TermBuf::new(80, 5);
        tb.process(b"AAAA\nBBBB\nCCCC\nDDDD\nEEEE");
        let rows: Vec<String> = (0..5)
            .map(|i| {
                tb.visible_row(i)
                    .iter()
                    .map(|c| c.ch)
                    .collect::<String>()
                    .trim_end()
                    .to_string()
            })
            .collect();
        // Each row should be different
        for i in 0..rows.len() {
            for j in (i + 1)..rows.len() {
                assert_ne!(rows[i], rows[j], "Row {i} and {j} should differ: {rows:?}");
            }
        }
    }

    #[test]
    fn termbuf_resize_preserves_content() {
        let mut tb = TermBuf::new(80, 24);
        tb.process(b"hello");
        tb.resize(40, 12);
        let text: String = tb.grid[0][..5].iter().map(|c| c.ch).collect();
        assert_eq!(text, "hello");
    }

    #[test]
    fn render_termbuf_to_buffer() {
        let mut tb = TermBuf::new(10, 3);
        tb.process(b"ABCDE");

        let area = ratatui::layout::Rect::new(0, 0, 10, 3);
        let mut buf = ratatui::buffer::Buffer::empty(area);

        // Inline render logic (mirrors render_termbuf in main.rs)
        for row in 0..area.height as usize {
            if row >= tb.rows() {
                break;
            }
            let cells = tb.visible_row(row);
            for col in 0..area.width as usize {
                if col >= cells.len() {
                    break;
                }
                #[allow(clippy::cast_possible_truncation)]
                let x = area.x + col as u16;
                #[allow(clippy::cast_possible_truncation)]
                let y = area.y + row as u16;
                let cell = &cells[col];
                let buf_cell = &mut buf[(x, y)];
                buf_cell.set_char(cell.ch);
                buf_cell.set_style(cell.style);
            }
        }

        // Verify first row matches
        for (i, expected) in "ABCDE".chars().enumerate() {
            #[allow(clippy::cast_possible_truncation)]
            let symbol = buf[(i as u16, 0)].symbol();
            assert_eq!(symbol, &expected.to_string(), "Mismatch at col {i}");
        }
    }
}
