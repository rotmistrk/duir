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
fn extract_text_from_line_basic() {
    let mut tb = TermBuf::new(80, 5);
    tb.process(b"line0\nline1\nline2\nline3\nline4");
    let full = extract_text(&tb);
    assert!(full.contains("line0"));
    // From line 2 onward
    let partial = extract_text_from_line(&tb, 2);
    assert!(!partial.contains("line0"));
    assert!(!partial.contains("line1"));
    assert!(partial.contains("line2"));
    assert!(partial.contains("line3"));
}

#[test]
fn extract_text_from_line_beyond_end() {
    let mut tb = TermBuf::new(80, 5);
    tb.process(b"hello");
    let result = extract_text_from_line(&tb, 9999);
    assert!(result.is_empty());
}

#[test]
fn extract_text_from_line_zero() {
    let mut tb = TermBuf::new(80, 3);
    tb.process(b"abc\ndef");
    let from_zero = extract_text_from_line(&tb, 0);
    let full = extract_text(&tb);
    assert_eq!(from_zero, full);
}

#[test]
fn extract_text_from_line_with_scrollback() {
    // 3-row terminal, push enough lines to create scrollback
    let mut tb = TermBuf::new(80, 3);
    tb.process(b"A\nB\nC\nD\nE");
    // A and B should be in scrollback, C/D/E in grid
    assert!(tb.total_lines() > 3);
    let from_1 = extract_text_from_line(&tb, 1);
    assert!(!from_1.contains('A'));
    assert!(from_1.contains('B'));
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
