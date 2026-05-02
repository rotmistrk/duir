use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};

use crate::syntax::SyntaxHighlighter;

/// Highlight markdown content with optional syntax highlighting for fenced code blocks.
pub fn highlight_lines_with_syntax(
    content: &str,
    cursor_row: usize,
    cursor_col: usize,
    highlighter: Option<&SyntaxHighlighter>,
) -> Vec<Line<'static>> {
    if content.is_empty() {
        return if cursor_row == 0 {
            vec![Line::from(Span::styled(
                " ".to_owned(),
                Style::default().add_modifier(Modifier::REVERSED),
            ))]
        } else {
            vec![Line::raw(String::new())]
        };
    }

    let raw_lines: Vec<&str> = content.lines().collect();
    let mut result = Vec::with_capacity(raw_lines.len());
    let mut i = 0;

    while i < raw_lines.len() {
        if raw_lines[i].starts_with("```") {
            i = process_code_block(&raw_lines, i, highlighter, cursor_row, cursor_col, &mut result);
        } else {
            let styled = highlight_md(raw_lines[i]);
            result.push(apply_cursor(i, raw_lines[i], cursor_row, cursor_col, &styled));
            i += 1;
        }
    }

    result
}

/// Process a fenced code block starting at `start`, returning the next line index.
fn process_code_block(
    raw_lines: &[&str],
    start: usize,
    highlighter: Option<&SyntaxHighlighter>,
    cursor_row: usize,
    cursor_col: usize,
    result: &mut Vec<Line<'static>>,
) -> usize {
    let fence_line = raw_lines[start];
    let lang = fence_line.trim_start_matches('`').trim();
    let fence_style = Style::default().fg(Color::DarkGray);
    result.push(apply_cursor(
        start,
        fence_line,
        cursor_row,
        cursor_col,
        &owned_line(fence_line, fence_style),
    ));

    let mut i = start + 1;
    let block_start = i;
    let mut code = String::new();
    while i < raw_lines.len() && !raw_lines[i].starts_with("```") {
        if !code.is_empty() {
            code.push('\n');
        }
        code.push_str(raw_lines[i]);
        i += 1;
    }

    let syntax_spans = highlighter
        .filter(|_| !lang.is_empty())
        .map(|h| h.highlight_code(&code, lang));

    let fallback = Style::default().fg(Color::Green);
    for (j, line_idx) in (block_start..i).enumerate() {
        let styled = syntax_span_or_fallback(syntax_spans.as_deref(), j, raw_lines[line_idx], fallback);
        result.push(apply_cursor(
            line_idx,
            raw_lines[line_idx],
            cursor_row,
            cursor_col,
            &styled,
        ));
    }

    if i < raw_lines.len() {
        result.push(apply_cursor(
            i,
            raw_lines[i],
            cursor_row,
            cursor_col,
            &owned_line(raw_lines[i], fence_style),
        ));
        i += 1;
    }
    i
}

fn syntax_span_or_fallback(
    syntax_spans: Option<&[Vec<Span<'static>>]>,
    idx: usize,
    raw: &str,
    fallback: Style,
) -> Line<'static> {
    syntax_spans
        .and_then(|s| s.get(idx))
        .map_or_else(|| owned_line(raw, fallback), |s| Line::from(s.clone()))
}

fn apply_cursor(
    line_idx: usize,
    raw: &str,
    cursor_row: usize,
    cursor_col: usize,
    styled: &Line<'static>,
) -> Line<'static> {
    if line_idx == cursor_row {
        insert_cursor_into_line(styled, raw, cursor_col)
    } else {
        styled.clone()
    }
}

fn owned_line(text: &str, style: Style) -> Line<'static> {
    Line::from(Span::styled(text.to_owned(), style))
}

/// Insert a reversed cursor character into an already-styled line.
/// Preserves all existing styling — only modifies the character at `cursor_col`.
fn insert_cursor_into_line(styled: &Line<'static>, raw: &str, col: usize) -> Line<'static> {
    let rev = Style::default().add_modifier(Modifier::REVERSED);

    if raw.is_empty() {
        return Line::from(Span::styled(" ".to_owned(), rev));
    }

    // For simple single-span lines, do direct insertion
    if styled.spans.len() == 1 {
        let style = styled.spans[0].style;
        let col = col.min(raw.len());
        if col >= raw.len() {
            return Line::from(vec![
                Span::styled(raw.to_owned(), style),
                Span::styled(" ".to_owned(), rev),
            ]);
        }
        return Line::from(vec![
            Span::styled(raw[..col].to_owned(), style),
            Span::styled(raw[col..=col].to_owned(), style.patch(rev)),
            Span::styled(raw[col + 1..].to_owned(), style),
        ]);
    }

    // For multi-span lines, find which span contains the cursor column
    // and split it to insert the cursor
    let col = col.min(raw.len());
    let mut result: Vec<Span<'static>> = Vec::new();
    let mut char_offset = 0;
    let mut cursor_placed = false;

    for span in &styled.spans {
        let span_len = span.content.len();
        let span_end = char_offset + span_len;

        if !cursor_placed && col >= char_offset && col < span_end {
            // Cursor is in this span
            let local_col = col - char_offset;
            if local_col > 0 {
                result.push(Span::styled(span.content[..local_col].to_owned(), span.style));
            }
            if local_col < span_len {
                result.push(Span::styled(
                    span.content[local_col..=local_col].to_owned(),
                    span.style.patch(rev),
                ));
                if local_col + 1 < span_len {
                    result.push(Span::styled(span.content[local_col + 1..].to_owned(), span.style));
                }
            }
            cursor_placed = true;
        } else {
            result.push(span.clone());
        }
        char_offset = span_end;
    }

    // Cursor at end of line
    if !cursor_placed {
        result.push(Span::styled(" ".to_owned(), rev));
    }

    Line::from(result)
}

fn highlight_md(line: &str) -> Line<'static> {
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
