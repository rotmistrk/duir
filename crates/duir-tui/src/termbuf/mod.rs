// Virtual terminal buffer: grid of styled cells driven by vte parser.

mod parser;

use ratatui::style::Style;

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
    // Grid indexing is safe: r < copy_rows <= self.rows, copy_cols <= self.cols.
    pub fn resize(&mut self, cols: usize, rows: usize) {
        let mut new_grid = vec![vec![Cell::default(); cols]; rows];
        let copy_rows = self.rows.min(rows);
        let copy_cols = self.cols.min(cols);
        for (r, new_row) in new_grid.iter_mut().enumerate().take(copy_rows) {
            if let Some(old_row) = self.grid.get(r) {
                let len = copy_cols.min(old_row.len()).min(new_row.len());
                new_row
                    .get_mut(..len)
                    .unwrap_or(&mut [])
                    .clone_from_slice(old_row.get(..len).unwrap_or(&[]));
            }
        }
        self.grid = new_grid;
        self.cols = cols;
        self.rows = rows;
        self.cursor_row = self.cursor_row.min(rows.saturating_sub(1));
        self.cursor_col = self.cursor_col.min(cols.saturating_sub(1));
    }

    /// Get a row for rendering. Negative offsets read from scrollback.
    // Grid/scrollback indices are clamped to valid ranges via .min().
    pub fn visible_row(&self, screen_row: usize) -> &[Cell] {
        if self.scroll_offset == 0 {
            return self
                .grid
                .get(screen_row.min(self.rows.saturating_sub(1)))
                .map_or(&[], |r| r.as_slice());
        }
        let total = self.scrollback.len() + self.rows;
        let start = total.saturating_sub(self.rows + self.scroll_offset);
        let idx = start + screen_row;
        if idx < self.scrollback.len() {
            self.scrollback.get(idx).map_or(&[], |r| r.as_slice())
        } else {
            let grid_idx = idx - self.scrollback.len();
            self.grid
                .get(grid_idx.min(self.rows.saturating_sub(1)))
                .map_or(&[], |r| r.as_slice())
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

    // row is checked against self.rows before indexing.
    fn clear_row(&mut self, row: usize) {
        if let Some(r) = self.grid.get_mut(row) {
            *r = vec![Cell::default(); self.cols];
        }
    }
}

// Thread-local vte parser (avoids allocation per process() call)
thread_local! {
    static PARSER: std::cell::Cell<vte::Parser> =
        std::cell::Cell::new(vte::Parser::new());
}

// Grid indexing is bounded by cursor_row < self.rows and cursor_col < self.cols checks.
// params[0] is from vte parser which always provides at least one element per param group.

pub fn extract_text(tb: &TermBuf) -> String {
    let lines = collect_all_lines(tb);
    trim_trailing_empty(lines).join("\n")
}

/// Extract text from a given line number onward (scrollback + grid).
/// `from_line` is 0-based into the combined scrollback+grid.
pub fn extract_text_from_line(tb: &TermBuf, from_line: usize) -> String {
    let lines = collect_all_lines(tb);
    let trimmed = trim_trailing_empty(lines);
    if from_line >= trimmed.len() {
        return String::new();
    }
    trimmed.get(from_line..).unwrap_or(&[]).join("\n")
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
        if let Some(line) = trimmed.get(i)
            && is_prompt_line(line)
        {
            return trimmed.get(i + 1..).unwrap_or(&[]).join("\n");
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
#[allow(clippy::indexing_slicing)]
mod tests;
