use quick_xml::events::Event;
use quick_xml::reader::Reader;

use super::emit;

#[derive(Default)]
pub(super) struct Para {
    pub text: String,
    pub heading_level: u8,
    pub is_list: bool,
    pub list_depth: u8,
    pub is_mono: bool,
}

pub(super) struct Table {
    pub rows: Vec<Vec<String>>,
    pub current_row: Vec<String>,
    pub current_cell: String,
}

#[derive(Default, Clone, Copy)]
pub(super) struct RunState(u8);

impl RunState {
    pub const ACTIVE: u8 = 1;
    pub const BOLD: u8 = 2;
    pub const ITALIC: u8 = 4;
    pub const MONO: u8 = 8;

    pub const fn has(self, flag: u8) -> bool {
        self.0 & flag != 0
    }
    pub const fn set(&mut self, flag: u8) {
        self.0 |= flag;
    }
    pub const fn new_run() -> Self {
        Self(Self::ACTIVE)
    }
}

pub(super) fn parse_document_xml(xml: &str) -> String {
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
                    emit::append_run(&text, run, &mut para);
                }
            }
            _ => {}
        }
    }

    emit::close_code_block(&mut out, &mut code_block_open);
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
                emit::emit_paragraph(para, out, code_block_open);
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
                emit::close_code_block(out, code_block_open);
                emit::emit_table(&t.rows, out);
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
