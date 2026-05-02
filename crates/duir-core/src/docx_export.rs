use docx_rs::{Docx, Paragraph, Run};

use crate::model::{Completion, TodoFile, TodoItem};

/// Export a `TodoFile` as a .docx document.
///
/// # Errors
/// Returns an error if the document cannot be generated.
pub fn export_docx(file: &TodoFile) -> crate::Result<Vec<u8>> {
    let mut doc = Docx::new();

    // Title
    doc = doc.add_paragraph(
        Paragraph::new()
            .add_run(Run::new().add_text(&file.title).bold())
            .style("Heading1"),
    );

    // File note
    if !file.note.is_empty() {
        for line in file.note.lines() {
            doc = doc.add_paragraph(Paragraph::new().add_run(Run::new().add_text(line)));
        }
    }

    // Items
    for item in &file.items {
        doc = render_item_docx(doc, item, 1);
    }

    let mut buf = Vec::new();
    doc.build()
        .pack(&mut std::io::Cursor::new(&mut buf))
        .map_err(|e| crate::OmelaError::Other(format!("DOCX error: {e}")))?;
    Ok(buf)
}

/// Export a single subtree as a .docx document.
///
/// # Errors
/// Returns an error if the document cannot be generated.
pub fn export_subtree_docx(item: &TodoItem) -> crate::Result<Vec<u8>> {
    let mut doc = Docx::new();
    doc = render_item_docx(doc, item, 1);

    let mut buf = Vec::new();
    doc.build()
        .pack(&mut std::io::Cursor::new(&mut buf))
        .map_err(|e| crate::OmelaError::Other(format!("DOCX error: {e}")))?;
    Ok(buf)
}

fn render_item_docx(mut doc: Docx, item: &TodoItem, depth: usize) -> Docx {
    let heading_style = match depth {
        1 => "Heading1",
        2 => "Heading2",
        3 => "Heading3",
        _ => "Heading4",
    };

    if depth <= 3 {
        // Render as heading
        let mut run = Run::new().add_text(&item.title);
        if item.important {
            run = run.bold();
        }
        if item.completed == Completion::Done {
            run = run.strike();
        }
        doc = doc.add_paragraph(Paragraph::new().add_run(run).style(heading_style));
    } else {
        // Render as checkbox list item
        let checkbox = match item.completed {
            Completion::Done => "☑ ",
            Completion::Open => "☐ ",
            Completion::Partial => "◐ ",
        };
        let mut run = Run::new().add_text(format!("{checkbox}{}", item.title));
        if item.important {
            run = run.bold();
        }
        if item.completed == Completion::Done {
            run = run.strike();
        }
        let indent = i32::try_from((depth - 4) * 720).unwrap_or(0); // 720 twips = 0.5 inch
        doc = doc.add_paragraph(Paragraph::new().add_run(run).indent(Some(indent), None, None, None));
    }

    // Note
    if !item.note.is_empty() {
        for line in item.note.lines() {
            if line.is_empty() {
                doc = doc.add_paragraph(Paragraph::new());
            } else {
                doc = doc.add_paragraph(Paragraph::new().add_run(Run::new().add_text(line)));
            }
        }
    }

    // Encrypted placeholder
    if item.is_locked() {
        doc = doc.add_paragraph(
            Paragraph::new().add_run(Run::new().add_text("🔒 [Encrypted content]").italic().color("999999")),
        );
        return doc;
    }

    // Children
    for child in &item.items {
        doc = render_item_docx(doc, child, depth + 1);
    }

    doc
}
