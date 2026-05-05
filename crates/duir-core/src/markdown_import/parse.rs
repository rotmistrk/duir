use crate::model::Completion;

/// Parsed line classification.
pub(super) enum Line<'a> {
    Heading {
        level: usize,
        text: &'a str,
        folded: bool,
        important: bool,
    },
    Checkbox {
        depth: usize,
        state: Completion,
        text: &'a str,
        important: bool,
        folded: bool,
    },
    Text(&'a str),
}

/// Parse a single line into its classification.
pub(super) fn classify_line(line: &str) -> Line<'_> {
    if let Some(heading) = try_parse_heading(line) {
        return heading;
    }
    if let Some(checkbox) = try_parse_checkbox(line) {
        return checkbox;
    }
    Line::Text(line)
}

/// Try to parse a heading line like `# Title` or `## Sub`.
fn try_parse_heading(line: &str) -> Option<Line<'_>> {
    let trimmed = line.trim_start();
    if !trimmed.starts_with('#') {
        return None;
    }
    let level = trimmed.bytes().take_while(|&b| b == b'#').count();
    let rest = trimmed[level..].trim();
    if rest.is_empty() {
        return None;
    }
    let (text, folded, important_meta) = strip_meta(rest);
    let (text, important_bold) = strip_bold(text);
    Some(Line::Heading {
        level,
        text,
        folded,
        important: important_meta || important_bold,
    })
}

/// Try to parse a checkbox line like `- [x] text` or `  - [ ] text`.
fn try_parse_checkbox(line: &str) -> Option<Line<'_>> {
    let indent = line.len() - line.trim_start().len();
    let depth = indent / 2;
    let trimmed = line.trim_start();

    let after_dash = trimmed.strip_prefix("- ")?;

    let (state, rest) = if let Some(r) = after_dash
        .strip_prefix("[x] ")
        .or_else(|| after_dash.strip_prefix("[X] "))
    {
        (Completion::Done, r)
    } else if let Some(r) = after_dash.strip_prefix("[ ] ") {
        (Completion::Open, r)
    } else if let Some(r) = after_dash.strip_prefix("[-] ") {
        (Completion::Partial, r)
    } else if after_dash.starts_with("**") && after_dash.ends_with("**") && after_dash.len() > 4 {
        let (text, folded, _) = strip_meta(after_dash);
        let inner = text
            .strip_prefix("**")
            .and_then(|s| s.strip_suffix("**"))
            .unwrap_or(text);
        return Some(Line::Checkbox {
            depth,
            state: Completion::Open,
            text: inner,
            important: true,
            folded,
        });
    } else {
        return None;
    };

    let (text_with_meta, folded, important_meta) = strip_meta(rest);
    let (text, important_bold) = strip_bold(text_with_meta);
    Some(Line::Checkbox {
        depth,
        state,
        text,
        important: important_meta || important_bold,
        folded,
    })
}

/// Extract `<!-- flags -->` metadata from end of line.
fn strip_meta(s: &str) -> (&str, bool, bool) {
    if let Some(start) = s.rfind("<!-- ")
        && let Some(end) = s[start..].find(" -->")
    {
        let flags = &s[start + 5..start + end];
        let text = s[..start].trim_end();
        let folded = flags.contains("folded");
        let important = flags.contains("important");
        return (text, folded, important);
    }
    (s, false, false)
}

fn strip_bold(s: &str) -> (&str, bool) {
    if s.starts_with("**") && s.ends_with("**") && s.len() > 4 {
        (&s[2..s.len() - 2], true)
    } else {
        (s, false)
    }
}
