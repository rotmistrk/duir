//! Export a [`TodoFile`] or subtree as a PDF document via `genpdf`.

use genpdf::elements::{Break, Paragraph};
use genpdf::style::Style;
use genpdf::{Alignment, Element};

use crate::model::{Completion, TodoFile, TodoItem};

/// Embedded Liberation Sans fonts for self-contained PDF generation.
mod embedded_fonts {
    pub const REGULAR: &[u8] = include_bytes!("../fonts/LiberationSans-Regular.ttf");
    pub const BOLD: &[u8] = include_bytes!("../fonts/LiberationSans-Bold.ttf");
    pub const ITALIC: &[u8] = include_bytes!("../fonts/LiberationSans-Italic.ttf");
    pub const BOLD_ITALIC: &[u8] = include_bytes!("../fonts/LiberationSans-BoldItalic.ttf");
}

fn build_font_family() -> crate::Result<genpdf::fonts::FontFamily<genpdf::fonts::FontData>> {
    let regular = genpdf::fonts::FontData::new(embedded_fonts::REGULAR.to_vec(), None)
        .map_err(|e| crate::OmelaError::Other(format!("font error: {e}")))?;
    let bold = genpdf::fonts::FontData::new(embedded_fonts::BOLD.to_vec(), None)
        .map_err(|e| crate::OmelaError::Other(format!("font error: {e}")))?;
    let italic = genpdf::fonts::FontData::new(embedded_fonts::ITALIC.to_vec(), None)
        .map_err(|e| crate::OmelaError::Other(format!("font error: {e}")))?;
    let bold_italic = genpdf::fonts::FontData::new(embedded_fonts::BOLD_ITALIC.to_vec(), None)
        .map_err(|e| crate::OmelaError::Other(format!("font error: {e}")))?;

    Ok(genpdf::fonts::FontFamily {
        regular,
        bold,
        italic,
        bold_italic,
    })
}

fn build_document() -> crate::Result<genpdf::Document> {
    let font_family = build_font_family()?;
    let mut doc = genpdf::Document::new(font_family);
    doc.set_paper_size(genpdf::PaperSize::A4);

    let mut decorator = genpdf::SimplePageDecorator::new();
    decorator.set_margins(15);
    doc.set_page_decorator(decorator);

    Ok(doc)
}

/// Export a [`TodoFile`] as a PDF.
///
/// # Errors
/// Returns an error if the PDF cannot be generated.
pub fn export_pdf(file: &TodoFile) -> crate::Result<Vec<u8>> {
    let mut doc = build_document()?;

    // Title
    doc.push(
        Paragraph::new(&file.title)
            .aligned(Alignment::Left)
            .styled(Style::new().bold().with_font_size(20)),
    );
    doc.push(Break::new(1));

    // File note
    if !file.note.is_empty() {
        push_note(&mut doc, &file.note);
    }

    // Items
    for item in &file.items {
        render_item(&mut doc, item, 1);
    }

    render_to_bytes(doc)
}

/// Export a single subtree as a PDF.
///
/// # Errors
/// Returns an error if the PDF cannot be generated.
pub fn export_subtree_pdf(item: &TodoItem) -> crate::Result<Vec<u8>> {
    let mut doc = build_document()?;
    render_item(&mut doc, item, 1);
    render_to_bytes(doc)
}

fn render_to_bytes(doc: genpdf::Document) -> crate::Result<Vec<u8>> {
    let mut buf = Vec::new();
    doc.render(&mut buf)
        .map_err(|e| crate::OmelaError::Other(format!("PDF render error: {e}")))?;
    Ok(buf)
}

fn render_item(doc: &mut genpdf::Document, item: &TodoItem, depth: usize) {
    let font_size = match depth {
        1 => 16,
        2 => 14,
        3 => 12,
        _ => 10,
    };

    if depth <= 3 {
        // Heading
        let mut style = Style::new().with_font_size(font_size);
        if item.important || depth <= 2 {
            style = style.bold();
        }
        if item.completed == Completion::Done {
            // No strikethrough in genpdf, use prefix
            let text = format!("✓ {}", item.title);
            doc.push(Paragraph::new(&text).styled(style));
        } else {
            doc.push(Paragraph::new(&item.title).styled(style));
        }
    } else {
        // Checkbox list item
        let checkbox = match item.completed {
            Completion::Done => "☑",
            Completion::Open => "☐",
            Completion::Partial => "◐",
        };
        let indent = "    ".repeat(depth.saturating_sub(4));
        let text = format!("{indent}{checkbox} {}", item.title);

        let mut style = Style::new().with_font_size(font_size);
        if item.important {
            style = style.bold();
        }
        doc.push(Paragraph::new(&text).styled(style));
    }

    // Note
    if !item.note.is_empty() {
        push_note(doc, &item.note);
    }

    // Encrypted placeholder
    if item.is_locked() {
        doc.push(Paragraph::new("🔒 [Encrypted content]").styled(Style::new().italic().with_font_size(9)));
        return;
    }

    // Children
    for child in &item.items {
        render_item(doc, child, depth + 1);
    }

    if depth <= 2 {
        doc.push(Break::new(0.5));
    }
}

fn push_note(doc: &mut genpdf::Document, note: &str) {
    let mut in_code_block = false;
    let mut code_lines: Vec<String> = Vec::new();

    for line in note.lines() {
        if line.starts_with("```") {
            if in_code_block {
                // End code block — emit collected lines
                for code_line in &code_lines {
                    doc.push(Paragraph::new(code_line).styled(Style::new().with_font_size(8)));
                }
                code_lines.clear();
                in_code_block = false;
            } else {
                in_code_block = true;
            }
            continue;
        }

        if in_code_block {
            code_lines.push(format!("  {line}"));
        } else if line.is_empty() {
            doc.push(Break::new(0.3));
        } else {
            doc.push(Paragraph::new(line).styled(Style::new().with_font_size(9)));
        }
    }

    // Flush unclosed code block
    for code_line in &code_lines {
        doc.push(Paragraph::new(code_line).styled(Style::new().with_font_size(8)));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn export_pdf_produces_bytes() {
        let mut file = TodoFile::new("Test PDF");
        file.note = "Some file note".to_owned();
        let mut item = TodoItem::new("Task 1");
        item.note = "Task note\n```rust\nfn main() {}\n```".to_owned();
        item.items.push(TodoItem::new("Sub 1"));
        file.items.push(item);

        let bytes = export_pdf(&file);
        assert!(bytes.is_ok());
        let data = bytes.unwrap_or_default();
        assert!(data.len() > 100, "PDF too small: {} bytes", data.len());
        assert!(data.starts_with(b"%PDF"), "Not a PDF");
    }

    #[test]
    fn export_subtree_pdf_produces_bytes() {
        let mut item = TodoItem::new("Root");
        item.items.push(TodoItem::new("Child"));
        let bytes = export_subtree_pdf(&item);
        assert!(bytes.is_ok());
    }
}
