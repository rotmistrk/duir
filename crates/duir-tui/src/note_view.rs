use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Paragraph, Widget, Wrap};

/// Simple markdown-aware note renderer for the TUI.
/// Renders bold (**text**), italic (*text*), and code (`text`) with terminal styles.
pub struct NoteView<'a> {
    content: &'a str,
    block: Option<Block<'a>>,
    scroll: u16,
}

impl<'a> NoteView<'a> {
    #[must_use]
    pub const fn new(content: &'a str) -> Self {
        Self {
            content,
            block: None,
            scroll: 0,
        }
    }

    #[must_use]
    pub fn block(mut self, block: Block<'a>) -> Self {
        self.block = Some(block);
        self
    }

    #[must_use]
    pub const fn scroll(mut self, offset: u16) -> Self {
        self.scroll = offset;
        self
    }
}

impl Widget for NoteView<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let lines: Vec<Line<'_>> = self.content.lines().map(|line| render_md_line(line)).collect();

        let mut paragraph = Paragraph::new(lines)
            .wrap(Wrap { trim: false })
            .scroll((self.scroll, 0));

        if let Some(block) = self.block {
            paragraph = paragraph.block(block);
        }

        paragraph.render(area, buf);
    }
}

fn render_md_line(line: &str) -> Line<'_> {
    let mut spans = Vec::new();
    let mut chars = line.char_indices().peekable();
    let mut plain_start = 0;

    while let Some(&(i, ch)) = chars.peek() {
        match ch {
            '*' => {
                // Flush plain text before this
                if i > plain_start {
                    spans.push(Span::raw(&line[plain_start..i]));
                }
                chars.next();
                if chars.peek().is_some_and(|&(_, c)| c == '*') {
                    // Bold: **text**
                    chars.next();
                    let start = chars.peek().map_or(line.len(), |&(idx, _)| idx);
                    let mut end = start;
                    while let Some(&(idx, c)) = chars.peek() {
                        if c == '*' {
                            end = idx;
                            chars.next();
                            if chars.peek().is_some_and(|&(_, c2)| c2 == '*') {
                                chars.next();
                                break;
                            }
                        } else {
                            end = idx + c.len_utf8();
                            chars.next();
                        }
                    }
                    spans.push(Span::styled(
                        &line[start..end],
                        Style::default().add_modifier(Modifier::BOLD),
                    ));
                } else {
                    // Italic: *text*
                    let start = chars.peek().map_or(line.len(), |&(idx, _)| idx);
                    let mut end = start;
                    while let Some(&(idx, c)) = chars.peek() {
                        if c == '*' {
                            end = idx;
                            chars.next();
                            break;
                        }
                        end = idx + c.len_utf8();
                        chars.next();
                    }
                    spans.push(Span::styled(
                        &line[start..end],
                        Style::default().add_modifier(Modifier::ITALIC),
                    ));
                }
                plain_start = chars.peek().map_or(line.len(), |&(idx, _)| idx);
            }
            '`' => {
                if i > plain_start {
                    spans.push(Span::raw(&line[plain_start..i]));
                }
                chars.next();
                let start = chars.peek().map_or(line.len(), |&(idx, _)| idx);
                let mut end = start;
                while let Some(&(idx, c)) = chars.peek() {
                    if c == '`' {
                        end = idx;
                        chars.next();
                        break;
                    }
                    end = idx + c.len_utf8();
                    chars.next();
                }
                spans.push(Span::styled(
                    &line[start..end],
                    Style::default().add_modifier(Modifier::DIM),
                ));
                plain_start = chars.peek().map_or(line.len(), |&(idx, _)| idx);
            }
            _ => {
                chars.next();
            }
        }
    }

    if plain_start < line.len() {
        spans.push(Span::raw(&line[plain_start..]));
    }

    Line::from(spans)
}
