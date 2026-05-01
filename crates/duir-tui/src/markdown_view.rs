use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};

/// Highlight markdown content, returning owned lines suitable for use after
/// the source `content` string has been dropped.
pub fn highlight_lines(content: &str, cursor_row: usize, cursor_col: usize) -> Vec<Line<'static>> {
    let mut in_fence = false;
    content
        .lines()
        .enumerate()
        .map(|(i, line)| {
            let styled = if line.starts_with("```") {
                in_fence = !in_fence;
                owned_line(line, Style::default().fg(Color::DarkGray))
            } else if in_fence {
                owned_line(line, Style::default().fg(Color::Green))
            } else {
                highlight_md(line)
            };
            if i == cursor_row {
                apply_cursor(line, cursor_col)
            } else {
                styled
            }
        })
        .collect()
}

fn owned_line(text: &str, style: Style) -> Line<'static> {
    Line::from(Span::styled(text.to_owned(), style))
}

fn apply_cursor(line: &str, col: usize) -> Line<'static> {
    let rev = Style::default().add_modifier(Modifier::REVERSED);
    if line.is_empty() {
        return Line::from(Span::styled(" ".to_owned(), rev));
    }
    let col = col.min(line.len());
    if col >= line.len() {
        Line::from(vec![Span::raw(line.to_owned()), Span::styled(" ".to_owned(), rev)])
    } else {
        Line::from(vec![
            Span::raw(line[..col].to_owned()),
            Span::styled(line[col..=col].to_owned(), rev),
            Span::raw(line[col + 1..].to_owned()),
        ])
    }
}

fn highlight_md(line: &str) -> Line<'static> {
    let base = Style::default();

    // Headings
    if line.strip_prefix("### ").is_some() {
        return owned_line(line, base.fg(Color::Yellow).add_modifier(Modifier::BOLD));
    }
    if line.strip_prefix("## ").is_some() {
        return owned_line(line, base.fg(Color::LightCyan).add_modifier(Modifier::BOLD));
    }
    if line.strip_prefix("# ").is_some() {
        return owned_line(
            line,
            base.fg(Color::LightCyan)
                .add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
        );
    }
    if line.starts_with('#') {
        return owned_line(line, base.fg(Color::Yellow));
    }

    // Blockquote
    if line.starts_with("> ") {
        return owned_line(line, base.fg(Color::DarkGray).add_modifier(Modifier::ITALIC));
    }

    // Horizontal rule
    let trimmed = line.trim();
    if trimmed == "---" || trimmed == "***" || trimmed == "___" {
        return owned_line(line, base.fg(Color::DarkGray));
    }

    // Checkbox items
    let stripped = line.trim_start();
    let indent = &line[..line.len() - stripped.len()];
    if let Some(rest) = stripped.strip_prefix("- [x] ") {
        return Line::from(vec![
            Span::styled(format!("{indent}- "), base),
            Span::styled("[x]".to_owned(), base.fg(Color::Green)),
            Span::styled(
                format!(" {rest}"),
                base.add_modifier(Modifier::CROSSED_OUT | Modifier::DIM),
            ),
        ]);
    }
    if let Some(rest) = stripped.strip_prefix("- [ ] ") {
        return Line::from(vec![
            Span::styled(format!("{indent}- "), base),
            Span::styled("[ ]".to_owned(), base.fg(Color::Red)),
            Span::styled(format!(" {rest}"), base),
        ]);
    }
    if let Some(rest) = stripped.strip_prefix("- [-] ") {
        return Line::from(vec![
            Span::styled(format!("{indent}- "), base),
            Span::styled("[-]".to_owned(), base.fg(Color::Yellow)),
            Span::styled(format!(" {rest}"), base),
        ]);
    }

    // Bullet list
    if stripped.starts_with("- ") || stripped.starts_with("* ") {
        let rest = &stripped[2..];
        return Line::from(vec![
            Span::styled(format!("{indent}• "), base.fg(Color::Cyan)),
            Span::raw(rest.to_owned()),
        ]);
    }

    // Inline markup
    render_inline(line, base)
}

#[allow(clippy::too_many_lines)]
fn render_inline(line: &str, base: Style) -> Line<'static> {
    let mut spans: Vec<Span<'static>> = Vec::new();
    let mut chars = line.char_indices().peekable();
    let mut plain_start = 0;

    while let Some(&(i, ch)) = chars.peek() {
        match ch {
            '`' => {
                push_plain(&mut spans, line, plain_start, i, base);
                chars.next();
                let start = peek_pos(&mut chars, line.len());
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
                spans.push(Span::styled(line[start..end].to_owned(), base.fg(Color::Green)));
                plain_start = peek_pos(&mut chars, line.len());
            }
            '*' => {
                push_plain(&mut spans, line, plain_start, i, base);
                chars.next();
                if chars.peek().is_some_and(|&(_, c)| c == '*') {
                    chars.next();
                    let start = peek_pos(&mut chars, line.len());
                    let mut end = start;
                    while let Some(&(idx, c)) = chars.peek() {
                        if c == '*' {
                            end = idx;
                            chars.next();
                            if chars.peek().is_some_and(|&(_, c2)| c2 == '*') {
                                chars.next();
                            }
                            break;
                        }
                        end = idx + c.len_utf8();
                        chars.next();
                    }
                    spans.push(Span::styled(
                        line[start..end].to_owned(),
                        base.add_modifier(Modifier::BOLD),
                    ));
                } else {
                    let start = peek_pos(&mut chars, line.len());
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
                        line[start..end].to_owned(),
                        base.add_modifier(Modifier::ITALIC),
                    ));
                }
                plain_start = peek_pos(&mut chars, line.len());
            }
            '[' => {
                push_plain(&mut spans, line, plain_start, i, base);
                chars.next();
                let text_start = peek_pos(&mut chars, line.len());
                let mut text_end = text_start;
                let mut found_link = false;
                while let Some(&(idx, c)) = chars.peek() {
                    if c == ']' {
                        text_end = idx;
                        chars.next();
                        if chars.peek().is_some_and(|&(_, c2)| c2 == '(') {
                            chars.next();
                            let url_start = peek_pos(&mut chars, line.len());
                            let mut url_end = url_start;
                            while let Some(&(idx2, c2)) = chars.peek() {
                                if c2 == ')' {
                                    url_end = idx2;
                                    chars.next();
                                    break;
                                }
                                url_end = idx2 + c2.len_utf8();
                                chars.next();
                            }
                            spans.push(Span::styled(
                                line[text_start..text_end].to_owned(),
                                base.fg(Color::LightBlue).add_modifier(Modifier::UNDERLINED),
                            ));
                            spans.push(Span::styled(
                                line[url_start..url_end].to_owned(),
                                base.fg(Color::DarkGray),
                            ));
                            found_link = true;
                        }
                        break;
                    }
                    text_end = idx + c.len_utf8();
                    chars.next();
                }
                if !found_link {
                    spans.push(Span::styled(line[i..text_end].to_owned(), base));
                }
                plain_start = peek_pos(&mut chars, line.len());
            }
            _ => {
                chars.next();
            }
        }
    }

    if plain_start < line.len() {
        spans.push(Span::styled(line[plain_start..].to_owned(), base));
    }

    if spans.is_empty() {
        owned_line(line, base)
    } else {
        Line::from(spans)
    }
}

fn push_plain(spans: &mut Vec<Span<'static>>, line: &str, start: usize, end: usize, style: Style) {
    if start < end {
        spans.push(Span::styled(line[start..end].to_owned(), style));
    }
}

fn peek_pos(chars: &mut std::iter::Peekable<std::str::CharIndices<'_>>, default: usize) -> usize {
    chars.peek().map_or(default, |&(idx, _)| idx)
}
