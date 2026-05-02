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

fn render_item_docx(doc: Docx, item: &TodoItem, depth: usize) -> Docx {
    render_item_docx_with_diagrams(doc, item, depth, None)
}

fn render_item_docx_with_diagrams(
    mut doc: Docx,
    item: &TodoItem,
    depth: usize,
    tool_paths: Option<&crate::diagram::ToolPaths>,
) -> Docx {
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

    // Note with diagram support
    if !item.note.is_empty() {
        doc = render_note_with_diagrams(doc, &item.note, tool_paths);
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
        doc = render_item_docx_with_diagrams(doc, child, depth + 1, tool_paths);
    }

    doc
}

/// Export with diagram rendering support.
///
/// # Errors
/// Returns an error if the document cannot be generated.
pub fn export_docx_with_diagrams(file: &TodoFile, tool_paths: &crate::diagram::ToolPaths) -> crate::Result<Vec<u8>> {
    let mut doc = Docx::new();
    doc = doc.add_paragraph(
        Paragraph::new()
            .add_run(Run::new().add_text(&file.title).bold())
            .style("Heading1"),
    );
    if !file.note.is_empty() {
        doc = render_note_with_diagrams(doc, &file.note, Some(tool_paths));
    }
    for item in &file.items {
        doc = render_item_docx_with_diagrams(doc, item, 1, Some(tool_paths));
    }
    let mut buf = Vec::new();
    doc.build()
        .pack(&mut std::io::Cursor::new(&mut buf))
        .map_err(|e| crate::OmelaError::Other(format!("DOCX error: {e}")))?;
    Ok(buf)
}

fn render_note_with_diagrams(mut doc: Docx, note: &str, tool_paths: Option<&crate::diagram::ToolPaths>) -> Docx {
    let mut lines = note.lines();

    while let Some(line) = lines.next() {
        if line.starts_with("```") {
            let tag = line.strip_prefix("```").unwrap_or("").trim();
            let is_diagram = !crate::diagram::extract_diagrams(&format!("{line}\nplaceholder\n```")).is_empty();

            // Collect block content
            let mut block_source = String::new();
            for inner in lines.by_ref() {
                if inner.starts_with("```") {
                    break;
                }
                if !block_source.is_empty() {
                    block_source.push('\n');
                }
                block_source.push_str(inner);
            }

            if is_diagram && let Some(tp) = tool_paths {
                let block = crate::diagram::DiagramBlock {
                    lang: crate::diagram::extract_diagrams(&format!("```{tag}\n{block_source}\n```"))
                        .into_iter()
                        .next()
                        .map_or(crate::diagram::DiagramLang::Mermaid, |b| b.lang),
                    source: block_source.clone(),
                };
                if let Ok(png_bytes) = crate::diagram::render_diagram(&block, tp) {
                    // Embed PNG image
                    let pic = docx_rs::Pic::new(&png_bytes).size(5_000_000, 3_000_000); // ~5x3 inches in EMU
                    doc = doc.add_paragraph(Paragraph::new().add_run(Run::new().add_image(pic)));
                    continue;
                }
                // Fallback: render as code block
            }

            // Code block (non-diagram or failed render): show as text
            doc = doc.add_paragraph(
                Paragraph::new().add_run(Run::new().add_text(format!("[{tag}]")).italic().color("999999")),
            );
            for code_line in block_source.lines() {
                doc = doc.add_paragraph(
                    Paragraph::new().add_run(
                        Run::new()
                            .add_text(code_line)
                            .fonts(docx_rs::RunFonts::new().ascii("Courier New")),
                    ),
                );
            }
        } else if line.is_empty() {
            doc = doc.add_paragraph(Paragraph::new());
        } else {
            doc = doc.add_paragraph(Paragraph::new().add_run(Run::new().add_text(line)));
        }
    }

    doc
}
