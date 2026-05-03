#![allow(clippy::indexing_slicing)]

use super::*;

/// Build a minimal .docx in memory with the given document.xml content.
fn make_docx(document_xml: &str) -> Vec<u8> {
    let mut buf = std::io::Cursor::new(Vec::new());
    {
        let mut zip = zip::ZipWriter::new(&mut buf);
        let options = zip::write::SimpleFileOptions::default();
        zip.start_file("word/document.xml", options).ok();
        std::io::Write::write_all(&mut zip, document_xml.as_bytes()).ok();
        zip.finish().ok();
    }
    buf.into_inner()
}

#[test]
fn heading_to_markdown() -> crate::Result<()> {
    let xml = r#"<?xml version="1.0"?>
    <w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
    <w:body>
      <w:p><w:pPr><w:pStyle w:val="Heading1"/></w:pPr>
        <w:r><w:t>Project</w:t></w:r></w:p>
      <w:p><w:pPr><w:pStyle w:val="Heading2"/></w:pPr>
        <w:r><w:t>Phase 1</w:t></w:r></w:p>
      <w:p><w:r><w:t>Some body text</w:t></w:r></w:p>
    </w:body></w:document>"#;

    let data = make_docx(xml);
    let md = docx_to_markdown(std::io::Cursor::new(data))?;

    assert!(md.contains("# Project"), "md: {md}");
    assert!(md.contains("## Phase 1"), "md: {md}");
    assert!(md.contains("Some body text"), "md: {md}");
    Ok(())
}

#[test]
fn bold_italic_runs() -> crate::Result<()> {
    let xml = r#"<?xml version="1.0"?>
    <w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
    <w:body>
      <w:p>
        <w:r><w:rPr><w:b/></w:rPr><w:t>bold</w:t></w:r>
        <w:r><w:t> and </w:t></w:r>
        <w:r><w:rPr><w:i/></w:rPr><w:t>italic</w:t></w:r>
      </w:p>
    </w:body></w:document>"#;

    let data = make_docx(xml);
    let md = docx_to_markdown(std::io::Cursor::new(data))?;

    assert!(md.contains("**bold**"), "md: {md}");
    assert!(md.contains("*italic*"), "md: {md}");
    Ok(())
}

#[test]
fn table_to_markdown() -> crate::Result<()> {
    let xml = r#"<?xml version="1.0"?>
    <w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
    <w:body>
      <w:tbl>
        <w:tr>
          <w:tc><w:p><w:r><w:t>Name</w:t></w:r></w:p></w:tc>
          <w:tc><w:p><w:r><w:t>Age</w:t></w:r></w:p></w:tc>
        </w:tr>
        <w:tr>
          <w:tc><w:p><w:r><w:t>Alice</w:t></w:r></w:p></w:tc>
          <w:tc><w:p><w:r><w:t>30</w:t></w:r></w:p></w:tc>
        </w:tr>
      </w:tbl>
    </w:body></w:document>"#;

    let data = make_docx(xml);
    let md = docx_to_markdown(std::io::Cursor::new(data))?;

    assert!(md.contains("| Name | Age |"), "md: {md}");
    assert!(md.contains("| Alice | 30 |"), "md: {md}");
    assert!(md.contains("| --- |"), "md: {md}");
    Ok(())
}

#[test]
fn monospaced_to_code_block() -> crate::Result<()> {
    let xml = r#"<?xml version="1.0"?>
    <w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
    <w:body>
      <w:p><w:r><w:rPr><w:rFonts w:ascii="Courier New"/></w:rPr>
        <w:t>fn main() {}</w:t></w:r></w:p>
      <w:p><w:r><w:rPr><w:rFonts w:ascii="Courier New"/></w:rPr>
        <w:t>println!("hi")</w:t></w:r></w:p>
      <w:p><w:r><w:t>Normal text</w:t></w:r></w:p>
    </w:body></w:document>"#;

    let data = make_docx(xml);
    let md = docx_to_markdown(std::io::Cursor::new(data))?;

    assert!(md.contains("```\nfn main() {}\n"), "md: {md}");
    assert!(md.contains("```\nNormal text"), "md: {md}");
    Ok(())
}

#[test]
fn list_items() -> crate::Result<()> {
    let xml = r#"<?xml version="1.0"?>
    <w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
    <w:body>
      <w:p><w:pPr><w:numPr><w:ilvl w:val="0"/><w:numId w:val="1"/></w:numPr></w:pPr>
        <w:r><w:t>First item</w:t></w:r></w:p>
      <w:p><w:pPr><w:numPr><w:ilvl w:val="1"/><w:numId w:val="1"/></w:numPr></w:pPr>
        <w:r><w:t>Sub item</w:t></w:r></w:p>
    </w:body></w:document>"#;

    let data = make_docx(xml);
    let md = docx_to_markdown(std::io::Cursor::new(data))?;

    assert!(md.contains("- [ ] First item"), "md: {md}");
    assert!(md.contains("  - [ ] Sub item"), "md: {md}");
    Ok(())
}

#[test]
fn full_import_creates_tree() -> crate::Result<()> {
    let xml = r#"<?xml version="1.0"?>
    <w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
    <w:body>
      <w:p><w:pPr><w:pStyle w:val="Heading1"/></w:pPr>
        <w:r><w:t>Sprint Plan</w:t></w:r></w:p>
      <w:p><w:pPr><w:pStyle w:val="Heading2"/></w:pPr>
        <w:r><w:t>Backend</w:t></w:r></w:p>
      <w:p><w:pPr><w:numPr><w:ilvl w:val="0"/><w:numId w:val="1"/></w:numPr></w:pPr>
        <w:r><w:t>API endpoints</w:t></w:r></w:p>
      <w:p><w:pPr><w:pStyle w:val="Heading2"/></w:pPr>
        <w:r><w:t>Frontend</w:t></w:r></w:p>
    </w:body></w:document>"#;

    let data = make_docx(xml);
    let file = import_docx(std::io::Cursor::new(data))?;

    assert_eq!(file.title, "Sprint Plan");
    assert_eq!(file.items.len(), 1);
    assert_eq!(file.items[0].title, "Sprint Plan");
    assert_eq!(file.items[0].items.len(), 2);
    assert_eq!(file.items[0].items[0].title, "Backend");
    assert_eq!(file.items[0].items[0].items[0].title, "API endpoints");
    assert_eq!(file.items[0].items[1].title, "Frontend");
    Ok(())
}
