use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};

use crate::markdown_highlight::highlight_md;
use crate::syntax::SyntaxHighlighter;

/// Highlight markdown content with optional syntax highlighting for fenced code blocks.
// All raw_lines.get(i).unwrap_or(&"") accesses are guarded by `while i < raw_lines.len()` loop bounds.
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
        if raw_lines.get(i).unwrap_or(&"").starts_with("```") {
            i = process_code_block(&raw_lines, i, highlighter, cursor_row, cursor_col, &mut result);
        } else {
            let styled = highlight_md(raw_lines.get(i).unwrap_or(&""));
            result.push(apply_cursor(
                i,
                raw_lines.get(i).unwrap_or(&""),
                cursor_row,
                cursor_col,
                &styled,
            ));
            i += 1;
        }
    }

    result
}

/// Process a fenced code block starting at `start`, returning the next line index.
// All raw_lines[] accesses are guarded by bounds checks or loop invariants.
fn process_code_block(
    raw_lines: &[&str],
    start: usize,
    highlighter: Option<&SyntaxHighlighter>,
    cursor_row: usize,
    cursor_col: usize,
    result: &mut Vec<Line<'static>>,
) -> usize {
    let fence_line = raw_lines.get(start).unwrap_or(&"");
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
    while i < raw_lines.len() && !raw_lines.get(i).unwrap_or(&"").starts_with("```") {
        if !code.is_empty() {
            code.push('\n');
        }
        code.push_str(raw_lines.get(i).unwrap_or(&""));
        i += 1;
    }

    let syntax_spans = highlighter
        .filter(|_| !lang.is_empty())
        .map(|h| h.highlight_code(&code, lang));

    let fallback = Style::default().fg(Color::Green);
    for (j, line_idx) in (block_start..i).enumerate() {
        let styled = syntax_span_or_fallback(
            syntax_spans.as_deref(),
            j,
            raw_lines.get(line_idx).unwrap_or(&""),
            fallback,
        );
        result.push(apply_cursor(
            line_idx,
            raw_lines.get(line_idx).unwrap_or(&""),
            cursor_row,
            cursor_col,
            &styled,
        ));
    }

    if i < raw_lines.len() {
        result.push(apply_cursor(
            i,
            raw_lines.get(i).unwrap_or(&""),
            cursor_row,
            cursor_col,
            &owned_line(raw_lines.get(i).unwrap_or(&""), fence_style),
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

pub fn owned_line(text: &str, style: Style) -> Line<'static> {
    Line::from(Span::styled(text.to_owned(), style))
}

/// Insert a reversed cursor character into an already-styled line.
/// Preserves all existing styling — only modifies the character at `cursor_col`.
// Char slicing is safe: col is clamped to chars.len() before use.
// spans[0] is guarded by spans.len() == 1 check.
fn insert_cursor_into_line(styled: &Line<'static>, raw: &str, col: usize) -> Line<'static> {
    let rev = Style::default().add_modifier(Modifier::REVERSED);

    if raw.is_empty() {
        return Line::from(Span::styled(" ".to_owned(), rev));
    }

    // For simple single-span lines, do direct insertion
    if styled.spans.len() == 1 {
        let style = styled.spans.first().map_or_else(Style::default, |s| s.style);
        let chars: Vec<char> = raw.chars().collect();
        let col = col.min(chars.len());
        if col >= chars.len() {
            return Line::from(vec![
                Span::styled(raw.to_owned(), style),
                Span::styled(" ".to_owned(), rev),
            ]);
        }
        let before: String = chars.get(..col).unwrap_or(&[]).iter().collect();
        let cursor_ch: String = chars.get(col..=col).unwrap_or(&[]).iter().collect();
        let after: String = chars.get(col + 1..).unwrap_or(&[]).iter().collect();
        return Line::from(vec![
            Span::styled(before, style),
            Span::styled(cursor_ch, style.patch(rev)),
            Span::styled(after, style),
        ]);
    }

    // For multi-span lines, find which span contains the cursor column
    // and split it to insert the cursor
    let col = col.min(raw.chars().count());
    let mut result: Vec<Span<'static>> = Vec::new();
    let mut char_offset = 0;
    let mut cursor_placed = false;

    for span in &styled.spans {
        let span_chars: Vec<char> = span.content.chars().collect();
        let span_len = span_chars.len();
        let span_end = char_offset + span_len;

        if !cursor_placed && col >= char_offset && col < span_end {
            let local_col = col - char_offset;
            if local_col > 0 {
                let before: String = span_chars.get(..local_col).unwrap_or(&[]).iter().collect();
                result.push(Span::styled(before, span.style));
            }
            if local_col < span_len {
                let cursor_ch: String = span_chars.get(local_col..=local_col).unwrap_or(&[]).iter().collect();
                result.push(Span::styled(cursor_ch, span.style.patch(rev)));
                if local_col + 1 < span_len {
                    let after: String = span_chars.get(local_col + 1..).unwrap_or(&[]).iter().collect();
                    result.push(Span::styled(after, span.style));
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

#[cfg(test)]
#[allow(clippy::unwrap_used)]
#[allow(clippy::indexing_slicing)] // Tests: indices are controlled by test setup
mod tests {
    use super::*;

    #[test]
    fn cursor_on_multibyte_char() {
        // • is 3 bytes (E2 80 A2). Cursor at char position 2 (the bullet).
        let line = "  • item";
        let styled = Line::from(line.to_owned());
        // Should not panic at any position
        let _r = insert_cursor_into_line(&styled, line, 2);
        let _r = insert_cursor_into_line(&styled, line, 0);
        let _r = insert_cursor_into_line(&styled, line, 4);
        let _r = insert_cursor_into_line(&styled, line, 7);
    }

    #[test]
    fn cursor_on_emoji() {
        let line = "🤖 hello";
        let styled = Line::from(line.to_owned());
        let _r = insert_cursor_into_line(&styled, line, 0);
        let _r = insert_cursor_into_line(&styled, line, 1);
        let _r = insert_cursor_into_line(&styled, line, 2);
    }

    #[test]
    fn cursor_on_cjk() {
        let line = "日本語テスト";
        let styled = Line::from(line.to_owned());
        for i in 0..=6 {
            let _r = insert_cursor_into_line(&styled, line, i);
        }
    }

    #[test]
    fn cursor_past_end() {
        let line = "short";
        let styled = Line::from(line.to_owned());
        let _r = insert_cursor_into_line(&styled, line, 100);
    }
}
