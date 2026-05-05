use super::parser::{Para, RunState};

pub(super) fn append_run(text: &str, run: RunState, para: &mut Para) {
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

pub(super) fn emit_paragraph(para: &Para, out: &mut String, code_block_open: &mut bool) {
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

pub(super) fn close_code_block(out: &mut String, code_block_open: &mut bool) {
    if *code_block_open {
        out.push_str("```\n");
        *code_block_open = false;
    }
}

pub(super) fn emit_table(rows: &[Vec<String>], out: &mut String) {
    let Some(first) = rows.first() else {
        return;
    };

    let cols = rows.iter().map(Vec::len).max().unwrap_or(0);
    if cols == 0 {
        return;
    }

    out.push('|');
    for cell in first {
        out.push(' ');
        out.push_str(cell);
        out.push_str(" |");
    }
    out.push('\n');

    out.push('|');
    for _ in 0..cols {
        out.push_str(" --- |");
    }
    out.push('\n');

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
