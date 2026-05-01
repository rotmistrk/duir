use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};

/// State tracker for fenced code blocks.
enum BlockState {
    Normal,
    InFence(Option<String>), // language tag
}

/// Highlight all lines, tracking fenced code block state.
#[allow(clippy::option_if_let_else)]
pub fn highlight_lines(content: &str, cursor_row: usize) -> Vec<Line<'_>> {
    let mut state = BlockState::Normal;
    content
        .lines()
        .enumerate()
        .map(|(i, line)| {
            let is_cursor = i == cursor_row;
            match &state {
                BlockState::Normal => {
                    if let Some(lang) = line.strip_prefix("```") {
                        let lang = lang.trim();
                        state = BlockState::InFence(if lang.is_empty() { None } else { Some(lang.to_owned()) });
                        Line::styled(line, Style::default().fg(Color::DarkGray))
                    } else {
                        highlight_line(line, is_cursor)
                    }
                }
                BlockState::InFence(_lang) => {
                    if line.starts_with("```") {
                        state = BlockState::Normal;
                        Line::styled(line, Style::default().fg(Color::DarkGray))
                    } else {
                        // Code block content — dim green
                        let style = Style::default().fg(Color::Green);
                        if is_cursor {
                            Line::styled(line, style.add_modifier(Modifier::UNDERLINED))
                        } else {
                            Line::styled(line, style)
                        }
                    }
                }
            }
        })
        .collect()
}

fn highlight_line(line: &str, is_cursor: bool) -> Line<'_> {
    let base = if is_cursor {
        Style::default().add_modifier(Modifier::UNDERLINED)
    } else {
        Style::default()
    };

    // Headings
    if let Some(_rest) = line.strip_prefix("### ") {
        return Line::styled(line, base.fg(Color::Yellow).add_modifier(Modifier::BOLD));
    }
    if let Some(_rest) = line.strip_prefix("## ") {
        return Line::styled(line, base.fg(Color::LightCyan).add_modifier(Modifier::BOLD));
    }
    if let Some(_rest) = line.strip_prefix("# ") {
        return Line::styled(
            line,
            base.fg(Color::LightCyan)
                .add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
        );
    }
    // Deeper headings
    if line.starts_with('#') {
        return Line::styled(line, base.fg(Color::Yellow));
    }

    // Blockquote
    if line.starts_with("> ") {
        return Line::styled(line, base.fg(Color::DarkGray).add_modifier(Modifier::ITALIC));
    }

    // Horizontal rule
    if line.trim() == "---" || line.trim() == "***" || line.trim() == "___" {
        return Line::styled(line, base.fg(Color::DarkGray));
    }

    // Checkbox items
    if let Some(rest) = line.trim_start().strip_prefix("- [x] ") {
        let indent = &line[..line.len() - line.trim_start().len()];
        return Line::from(vec![
            Span::styled(format!("{indent}- "), base),
            Span::styled("[x]", base.fg(Color::Green)),
            Span::styled(
                format!(" {rest}"),
                base.add_modifier(Modifier::CROSSED_OUT | Modifier::DIM),
            ),
        ]);
    }
    if let Some(rest) = line.trim_start().strip_prefix("- [ ] ") {
        let indent = &line[..line.len() - line.trim_start().len()];
        return Line::from(vec![
            Span::styled(format!("{indent}- "), base),
            Span::styled("[ ]", base.fg(Color::Red)),
            Span::styled(format!(" {rest}"), base),
        ]);
    }
    if let Some(rest) = line.trim_start().strip_prefix("- [-] ") {
        let indent = &line[..line.len() - line.trim_start().len()];
        return Line::from(vec![
            Span::styled(format!("{indent}- "), base),
            Span::styled("[-]", base.fg(Color::Yellow)),
            Span::styled(format!(" {rest}"), base),
        ]);
    }

    // List items
    if line.trim_start().starts_with("- ") || line.trim_start().starts_with("* ") {
        let indent = &line[..line.len() - line.trim_start().len()];
        let rest = &line.trim_start()[2..];
        return Line::from(vec![
            Span::styled(format!("{indent}• "), base.fg(Color::Cyan)),
            Span::styled(rest, base),
        ]);
    }

    // Inline markup
    render_inline(line, base)
}

#[allow(clippy::too_many_lines)]
fn render_inline(line: &str, base: Style) -> Line<'_> {
    let mut spans = Vec::new();
    let mut chars = line.char_indices().peekable();
    let mut plain_start = 0;

    while let Some(&(i, ch)) = chars.peek() {
        match ch {
            '`' => {
                if i > plain_start {
                    spans.push(Span::styled(&line[plain_start..i], base));
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
                spans.push(Span::styled(&line[start..end], base.fg(Color::Green)));
                plain_start = chars.peek().map_or(line.len(), |&(idx, _)| idx);
            }
            '*' => {
                if i > plain_start {
                    spans.push(Span::styled(&line[plain_start..i], base));
                }
                chars.next();
                if chars.peek().is_some_and(|&(_, c)| c == '*') {
                    // Bold
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
                    spans.push(Span::styled(&line[start..end], base.add_modifier(Modifier::BOLD)));
                } else {
                    // Italic
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
                    spans.push(Span::styled(&line[start..end], base.add_modifier(Modifier::ITALIC)));
                }
                plain_start = chars.peek().map_or(line.len(), |&(idx, _)| idx);
            }
            '[' => {
                // Markdown link: [text](url)
                if i > plain_start {
                    spans.push(Span::styled(&line[plain_start..i], base));
                }
                chars.next();
                let text_start = chars.peek().map_or(line.len(), |&(idx, _)| idx);
                let mut text_end = text_start;
                let mut found_link = false;
                while let Some(&(idx, c)) = chars.peek() {
                    if c == ']' {
                        text_end = idx;
                        chars.next();
                        if chars.peek().is_some_and(|&(_, c2)| c2 == '(') {
                            chars.next();
                            let url_start = chars.peek().map_or(line.len(), |&(idx2, _)| idx2);
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
                                &line[text_start..text_end],
                                base.fg(Color::LightBlue).add_modifier(Modifier::UNDERLINED),
                            ));
                            spans.push(Span::styled(&line[url_start..url_end], base.fg(Color::DarkGray)));
                            found_link = true;
                        }
                        break;
                    }
                    text_end = idx + c.len_utf8();
                    chars.next();
                }
                if !found_link {
                    spans.push(Span::styled(&line[i..text_end], base));
                }
                plain_start = chars.peek().map_or(line.len(), |&(idx, _)| idx);
            }
            _ => {
                chars.next();
            }
        }
    }

    if plain_start < line.len() {
        spans.push(Span::styled(&line[plain_start..], base));
    }

    if spans.is_empty() {
        Line::styled(line, base)
    } else {
        Line::from(spans)
    }
}
