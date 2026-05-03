/// Parsed ex-command with resolved line range.
pub enum ExCommand {
    Yank {
        start: usize,
        end: usize,
    },
    Delete {
        start: usize,
        end: usize,
    },
    Substitute {
        start: usize,
        end: usize,
        pattern: String,
        replacement: String,
        flags: String,
    },
    Shell {
        start: usize,
        end: usize,
        command: String,
    },
}

/// Parse a vim ex-command string like `1,$y`, `.,+5s/foo/bar/g`, `.,.+3!sort`.
pub fn parse_ex_command(cmd: &str, cursor_row: usize, total_lines: usize) -> Option<ExCommand> {
    let cmd = cmd.trim();

    let cmd_start = cmd
        .find(|c: char| c.is_ascii_alphabetic() || c == '!' || c == 's')
        .unwrap_or(cmd.len());

    let range_str = cmd[..cmd_start].trim();
    let cmd_part = &cmd[cmd_start..];

    let (start, end) = parse_range(range_str, cursor_row, total_lines)?;

    if cmd_part.starts_with('y') {
        return Some(ExCommand::Yank { start, end });
    }
    if cmd_part.starts_with('d') {
        return Some(ExCommand::Delete { start, end });
    }
    if let Some(rest) = cmd_part.strip_prefix('s') {
        return parse_substitute(rest).map(|(pattern, replacement, flags)| ExCommand::Substitute {
            start,
            end,
            pattern,
            replacement,
            flags,
        });
    }
    cmd_part.strip_prefix('!').map(|rest| ExCommand::Shell {
        start,
        end,
        command: rest.to_owned(),
    })
}

fn parse_range(range: &str, cursor: usize, total: usize) -> Option<(usize, usize)> {
    if range.is_empty() {
        return Some((cursor, cursor));
    }
    if range == "%" {
        return Some((0, total.saturating_sub(1)));
    }

    let parts: Vec<&str> = range.splitn(2, ',').collect();
    match parts.len() {
        1 => {
            let addr = parse_address(parts.first()?.trim(), cursor, total)?;
            Some((addr, addr))
        }
        2 => {
            let start = parse_address(parts.first()?.trim(), cursor, total)?;
            let end = parse_address(parts.get(1)?.trim(), cursor, total)?;
            Some((start, end))
        }
        _ => None,
    }
}

fn parse_address(addr: &str, cursor: usize, total: usize) -> Option<usize> {
    if addr == "." {
        return Some(cursor);
    }
    if addr == "$" {
        return Some(total.saturating_sub(1));
    }
    if let Ok(n) = addr.parse::<usize>() {
        return Some(n.saturating_sub(1));
    }
    if let Some(rest) = addr.strip_prefix(".+") {
        let offset: usize = rest.parse().ok()?;
        return Some((cursor + offset).min(total.saturating_sub(1)));
    }
    if let Some(rest) = addr.strip_prefix(".-") {
        let offset: usize = rest.parse().ok()?;
        return Some(cursor.saturating_sub(offset));
    }
    if let Some(rest) = addr.strip_prefix('+') {
        let offset: usize = rest.parse().ok()?;
        return Some((cursor + offset).min(total.saturating_sub(1)));
    }
    if let Some(rest) = addr.strip_prefix('-') {
        let offset: usize = rest.parse().ok()?;
        return Some(cursor.saturating_sub(offset));
    }
    None
}

fn parse_substitute(s: &str) -> Option<(String, String, String)> {
    if s.is_empty() {
        return None;
    }
    let delim = s.chars().next()?;
    let rest = &s[delim.len_utf8()..];
    let parts: Vec<&str> = rest.splitn(3, delim).collect();
    if parts.len() < 2 {
        return None;
    }
    let pattern = (*parts.first()?).to_owned();
    let replacement = (*parts.get(1)?).to_owned();
    let flags = parts.get(2).unwrap_or(&"").to_string();
    Some((pattern, replacement, flags))
}

pub(super) fn extract_url(line: &str, col: usize) -> Option<String> {
    for prefix in ["https://", "http://"] {
        let mut search_from = 0;
        while let Some(pos) = line[search_from..].find(prefix) {
            let url_start = search_from + pos;
            let url_end = line[url_start..]
                .find(|c: char| c.is_whitespace() || c == ')' || c == '>' || c == '"' || c == '\'')
                .map_or(line.len(), |e| url_start + e);
            if col >= url_start && col <= url_end {
                return Some(line[url_start..url_end].to_owned());
            }
            search_from = url_end;
        }
    }
    if let Some(paren_start) = line.find("](") {
        let url_start = paren_start + 2;
        if let Some(paren_end) = line[url_start..].find(')') {
            let url = &line[url_start..url_start + paren_end];
            if (url.starts_with("http://") || url.starts_with("https://")) && col <= url_start + paren_end {
                return Some(url.to_owned());
            }
        }
    }
    None
}

pub(super) fn open_in_browser(url: &str) {
    #[cfg(target_os = "macos")]
    let cmd = "open";
    #[cfg(target_os = "linux")]
    let cmd = "xdg-open";
    #[cfg(target_os = "windows")]
    let cmd = "start";

    std::process::Command::new(cmd)
        .arg(url)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .ok();
}
