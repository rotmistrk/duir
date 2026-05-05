use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};

use crate::markdown_view::owned_line;

pub fn highlight_md(line: &str) -> Line<'static> {
    let base = Style::default();

    // Headings
    if line.starts_with("#### ") || line.starts_with("##### ") || line.starts_with("###### ") {
        return owned_line(line, base.fg(Color::Yellow));
    }
    if line.starts_with("### ") {
        return owned_line(line, base.fg(Color::Yellow).add_modifier(Modifier::BOLD));
    }
    if line.starts_with("## ") {
        return owned_line(line, base.fg(Color::LightCyan).add_modifier(Modifier::BOLD));
    }
    if line.starts_with("# ") {
        return owned_line(
            line,
            base.fg(Color::LightCyan)
                .add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
        );
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
    if let Some(rest) = stripped
        .strip_prefix("- [x] ")
        .or_else(|| stripped.strip_prefix("- [X] "))
    {
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
                parse_link(&mut spans, &mut chars, line, i, base);
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

fn parse_link(
    spans: &mut Vec<Span<'static>>,
    chars: &mut std::iter::Peekable<std::str::CharIndices<'_>>,
    line: &str,
    bracket_pos: usize,
    base: Style,
) {
    let text_start = peek_pos(chars, line.len());
    let mut text_end = text_start;
    let mut found_link = false;
    while let Some(&(idx, c)) = chars.peek() {
        if c == ']' {
            text_end = idx;
            chars.next();
            if chars.peek().is_some_and(|&(_, c2)| c2 == '(') {
                chars.next();
                let url_start = peek_pos(chars, line.len());
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
        spans.push(Span::styled(line[bracket_pos..text_end].to_owned(), base));
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
