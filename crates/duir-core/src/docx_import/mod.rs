//! Import `.docx` files by converting to markdown, then using [`crate::markdown_import`].

mod emit;
mod parser;

#[cfg(test)]
mod tests;

use std::io::{Read, Seek};

use crate::model::TodoFile;

/// Import a `.docx` file from a reader into a [`TodoFile`].
///
/// # Errors
/// Returns an error if the zip or XML cannot be parsed.
pub fn import_docx<R: Read + Seek>(reader: R) -> crate::Result<TodoFile> {
    let md = docx_to_markdown(reader)?;
    Ok(crate::markdown_import::import_markdown(&md))
}

/// Convert a `.docx` file to a markdown string.
///
/// # Errors
/// Returns an error if the zip or XML cannot be parsed.
pub fn docx_to_markdown<R: Read + Seek>(reader: R) -> crate::Result<String> {
    let mut archive =
        zip::ZipArchive::new(reader).map_err(|e| crate::OmelaError::Other(format!("bad docx zip: {e}")))?;

    let mut xml = String::new();
    archive
        .by_name("word/document.xml")
        .map_err(|e| crate::OmelaError::Other(format!("no document.xml: {e}")))?
        .read_to_string(&mut xml)
        .map_err(|e| crate::OmelaError::Other(format!("read error: {e}")))?;

    Ok(parser::parse_document_xml(&xml))
}
