//! Import `.docx` files by converting to markdown, then using [`crate::markdown_import`].

use std::io::{Read, Seek};

use quick_xml::events::Event;
use quick_xml::reader::Reader;

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

    Ok(parse_document_xml(&xml))
}

#[derive(Default)]
struct Para {
    text: String,
    heading_level: u8,
    is_list: bool,
    list_depth: u8,
    is_mono: bool,
}

struct Table {
    rows: Vec<Vec<String>>,
    current_row: Vec<String>,
    current_cell: String,
}

#[derive(Default, Clone, Copy)]
struct RunState(u8);

impl RunState {
    const ACTIVE: u8 = 1;
    const BOLD: u8 = 2;
    const ITALIC: u8 = 4;
    const MONO: u8 = 8;

    const fn has(self, flag: u8) -> bool {
        self.0 & flag != 0
    }
    const fn set(&mut self, flag: u8) {
        self.0 |= flag;
    }
    const fn new_run() -> Self {
        Self(Self::ACTIVE)
    }
}

fn parse_document_xml(xml: &str) -> String {
    let mut reader = Reader::from_str(xml);
    let mut out = String::new();
    let mut para = Para::default();
    let mut table: Option<Table> = None;
    let mut run = RunState::default();
    let mut code_block_open = false;

    loop {
        match reader.read_event() {
            Err(_) | Ok(Event::Eof) => break,
            Ok(Event::Start(ref e) | Event::Empty(ref e)) => {
                let name = e.name();
                let local = local_name(name.as_ref());
                handle_start(local, e, &mut para, &mut table, &mut run);
            }
            Ok(Event::End(ref e)) => {
                let name = e.name();
                let local = local_name(name.as_ref());
                handle_end(local, &para, &mut table, &mut run, &mut out, &mut code_block_open);
                if local == b"p" {
                    para = Para::default();
                }
            }
            Ok(Event::Text(ref e)) => {
                let text = e.unescape().unwrap_or_default();
                if text.is_empty() {
                    continue;
                }
                if let Some(t) = table.as_mut() {
                    t.current_cell.push_str(&text);
                } else {
                    append_run(&text, run, &mut para);
                }
            }
            _ => {}
        }
    }

    close_code_block(&mut out, &mut code_block_open);
    while out.ends_with("\n\n") {
        out.pop();
    }
    out
}

fn handle_start(
    local: &[u8],
    e: &quick_xml::events::BytesStart<'_>,
    para: &mut Para,
    table: &mut Option<Table>,
    run: &mut RunState,
) {
    match local {
        b"pStyle" => {
            if let Some(val) = attr_val(e, b"w:val") {
                parse_pstyle(&val, para);
            }
        }
        b"ilvl" => {
            if let Some(Ok(n)) = attr_val(e, b"w:val").map(|v| v.parse::<u8>()) {
                para.list_depth = n;
                para.is_list = true;
            }
        }
        b"numId" => {
            if attr_val(e, b"w:val").is_some() {
                para.is_list = true;
            }
        }
        b"r" => *run = RunState::new_run(),
        b"b" if run.has(RunState::ACTIVE) => run.set(RunState::BOLD),
        b"i" if run.has(RunState::ACTIVE) => run.set(RunState::ITALIC),
        b"rFonts" if run.has(RunState::ACTIVE) => {
            if let Some(font) = attr_val(e, b"w:ascii").or_else(|| attr_val(e, b"w:hAnsi")) {
                let fl = font.to_lowercase();
                if fl.contains("courier") || fl.contains("consolas") || fl.contains("mono") {
                    run.set(RunState::MONO);
                }
            }
        }
        b"tbl" => {
            *table = Some(Table {
                rows: Vec::new(),
                current_row: Vec::new(),
                current_cell: String::new(),
            });
        }
        b"tr" => {
            if let Some(t) = table.as_mut() {
                t.current_row = Vec::new();
            }
        }
        b"tc" => {
            if let Some(t) = table.as_mut() {
                t.current_cell = String::new();
            }
        }
        _ => {}
    }
}

fn handle_end(
    local: &[u8],
    para: &Para,
    table: &mut Option<Table>,
    run: &mut RunState,
    out: &mut String,
    code_block_open: &mut bool,
) {
    match local {
        b"r" => *run = RunState::default(),
        b"p" => {
            if table.is_none() {
                emit_paragraph(para, out, code_block_open);
            }
        }
        b"tc" => {
            if let Some(t) = table.as_mut() {
                t.current_row.push(t.current_cell.trim().to_owned());
                t.current_cell = String::new();
            }
        }
        b"tr" => {
            if let Some(t) = table.as_mut() {
                t.rows.push(std::mem::take(&mut t.current_row));
            }
        }
        b"tbl" => {
            if let Some(t) = table.take() {
                close_code_block(out, code_block_open);
                emit_table(&t.rows, out);
            }
        }
        _ => {}
    }
}

fn parse_pstyle(val: &str, para: &mut Para) {
    let lower = val.to_lowercase();
    if let Some(n) = lower.strip_prefix("heading") {
        if let Ok(level) = n.parse::<u8>() {
            para.heading_level = level;
        }
    } else if lower.contains("list") || lower.contains("bullet") {
        para.is_list = true;
    } else if lower.contains("code") || lower.contains("source") {
        para.is_mono = true;
    }
}

fn append_run(text: &str, run: RunState, para: &mut Para) {
    if run.has(RunState::MONO) {
        para.is_mono = true;
        para.text.push_str(text);
        return;
    }

    match (run.has(RunState::BOLD), run.has(RunState::ITALIC)) {
        (true, true) => {
            para.text.push_str("***");
            para.text.push_str(text);
            para.text.push_str("***");
        }
        (true, false) => {
            para.text.push_str("**");
            para.text.push_str(text);
            para.text.push_str("**");
        }
        (false, true) => {
            para.text.push('*');
            para.text.push_str(text);
            para.text.push('*');
        }
        (false, false) => para.text.push_str(text),
    }
}

fn emit_paragraph(para: &Para, out: &mut String, code_block_open: &mut bool) {
    let text = para.text.trim();

    if text.is_empty() {
        if !*code_block_open {
            out.push('\n');
        }
        return;
    }

    if para.is_mono {
        if !*code_block_open {
            out.push_str("```\n");
            *code_block_open = true;
        }
        out.push_str(text);
        out.push('\n');
        return;
    }

    close_code_block(out, code_block_open);

    if para.heading_level > 0 {
        for _ in 0..para.heading_level {
            out.push('#');
        }
        out.push(' ');
    } else if para.is_list {
        let indent = "  ".repeat(para.list_depth as usize);
        out.push_str(&indent);
        out.push_str("- [ ] ");
    }

    out.push_str(text);
    out.push('\n');
}

fn close_code_block(out: &mut String, code_block_open: &mut bool) {
    if *code_block_open {
        out.push_str("```\n");
        *code_block_open = false;
    }
}

fn emit_table(rows: &[Vec<String>], out: &mut String) {
    let Some(first) = rows.first() else {
        return;
    };

    let cols = rows.iter().map(Vec::len).max().unwrap_or(0);
    if cols == 0 {
        return;
    }

    // Header
    out.push('|');
    for cell in first {
        out.push(' ');
        out.push_str(cell);
        out.push_str(" |");
    }
    out.push('\n');

    // Separator
    out.push('|');
    for _ in 0..cols {
        out.push_str(" --- |");
    }
    out.push('\n');

    // Data rows
    for row in rows.iter().skip(1) {
        out.push('|');
        for i in 0..cols {
            out.push(' ');
            if let Some(cell) = row.get(i) {
                out.push_str(cell);
            }
            out.push_str(" |");
        }
        out.push('\n');
    }
    out.push('\n');
}

fn local_name(name: &[u8]) -> &[u8] {
    name.iter()
        .rposition(|&b| b == b':')
        .and_then(|pos| name.get(pos + 1..))
        .unwrap_or(name)
}

fn attr_val(e: &quick_xml::events::BytesStart<'_>, name: &[u8]) -> Option<String> {
    e.attributes().filter_map(Result::ok).find_map(|a| {
        if a.key.as_ref() == name {
            String::from_utf8(a.value.to_vec()).ok()
        } else {
            None
        }
    })
}

#[cfg(test)]
mod tests;
