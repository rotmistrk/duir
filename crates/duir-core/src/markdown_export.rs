//! Markdown export for todo trees.
//!
//! Converts [`TodoItem`] subtrees and [`TodoFile`]s into readable markdown.

use std::fmt::Write;

use crate::model::{Completion, TodoFile, TodoItem};

const DEFAULT_MAX_HEADING_DEPTH: usize = 3;

/// Converts a [`TodoItem`] subtree to a markdown string.
///
/// Items at depth 1 through `max_heading_depth` are rendered as headings
/// (`#`, `##`, etc.). Items deeper than `max_heading_depth` become nested
/// checkbox list items. Important items get **bold** titles. Completed
/// items use `[x]`, open use `[ ]`, and partial use `[-]`.
///
/// Non-empty notes are rendered as paragraphs after the heading or checkbox.
#[must_use]
pub fn export_subtree(item: &TodoItem, max_heading_depth: usize) -> String {
    let mut buf = String::new();
    render_item(&mut buf, item, 1, max_heading_depth, 0);
    buf
}

/// Export a subtree as markdown, redacting locked encrypted nodes.
/// Encrypted nodes show as `🔒 [Encrypted]` without exposing content.
#[must_use]
pub fn export_subtree_safe(item: &TodoItem, max_heading_depth: usize) -> String {
    let mut buf = String::new();
    render_item_safe(&mut buf, item, 1, max_heading_depth, 0);
    buf
}

/// Converts a [`TodoFile`] to a markdown string.
///
/// The file title is rendered as a `#` heading, followed by the file note
/// (if non-empty), then each top-level item starting at depth 2.
#[must_use]
pub fn export_file(file: &TodoFile) -> String {
    let mut buf = String::new();
    let _ = writeln!(buf, "# {}", file.title);
    if !file.note.is_empty() {
        let _ = writeln!(buf, "\n{}", file.note);
    }
    for item in &file.items {
        let _ = writeln!(buf);
        render_item(&mut buf, item, 2, DEFAULT_MAX_HEADING_DEPTH, 0);
    }
    buf
}

const fn checkbox(completion: &Completion) -> &'static str {
    match completion {
        Completion::Done => "[x]",
        Completion::Open => "[ ]",
        Completion::Partial => "[-]",
    }
}

fn format_title(item: &TodoItem) -> String {
    if item.important {
        format!("**{}**", item.title)
    } else {
        item.title.clone()
    }
}

fn meta_comment(item: &TodoItem) -> String {
    let mut flags = Vec::new();
    if item.folded {
        flags.push("folded");
    }
    if item.important {
        flags.push("important");
    }
    if flags.is_empty() {
        String::new()
    } else {
        format!(" <!-- {} -->", flags.join(" "))
    }
}

fn render_item(buf: &mut String, item: &TodoItem, depth: usize, max_heading_depth: usize, list_indent: usize) {
    if depth <= max_heading_depth {
        render_heading(buf, item, depth, max_heading_depth);
    } else {
        render_checkbox(buf, item, max_heading_depth, list_indent);
    }
}

fn render_heading(buf: &mut String, item: &TodoItem, depth: usize, max_heading_depth: usize) {
    let hashes: String = "#".repeat(depth);
    let meta = meta_comment(item);
    let _ = writeln!(buf, "{hashes} {}{meta}", format_title(item));
    if !item.note.is_empty() {
        let _ = writeln!(buf, "\n{}", item.note);
    }
    let children_are_checkboxes = depth >= max_heading_depth;
    if children_are_checkboxes && !item.items.is_empty() {
        let _ = writeln!(buf);
    }
    for child in &item.items {
        if !children_are_checkboxes {
            let _ = writeln!(buf);
        }
        render_item(buf, child, depth + 1, max_heading_depth, 0);
    }
}

fn render_checkbox(buf: &mut String, item: &TodoItem, max_heading_depth: usize, list_indent: usize) {
    let indent = "  ".repeat(list_indent);
    let cb = checkbox(&item.completed);
    let meta = meta_comment(item);
    let _ = writeln!(buf, "{indent}- {cb} {}{meta}", format_title(item));
    if !item.note.is_empty() {
        let _ = writeln!(buf);
        let note_indent = "  ".repeat(list_indent + 1);
        for line in item.note.lines() {
            let _ = writeln!(buf, "{note_indent}{line}");
        }
        let _ = writeln!(buf);
    }
    for child in &item.items {
        render_item(buf, child, max_heading_depth + 1, max_heading_depth, list_indent + 1);
    }
}

fn render_item_safe(buf: &mut String, item: &TodoItem, depth: usize, max_heading_depth: usize, list_indent: usize) {
    // Redact locked encrypted nodes
    if item.is_encrypted() && !item.unlocked {
        if depth <= max_heading_depth {
            let hashes: String = "#".repeat(depth);
            let _ = writeln!(buf, "{hashes} 🔒 {}", item.title);
        } else {
            let indent = "  ".repeat(list_indent);
            let _ = writeln!(buf, "{indent}- 🔒 {}", item.title);
        }
        return;
    }
    // Otherwise render normally, but recurse with safe variant
    if depth <= max_heading_depth {
        let hashes: String = "#".repeat(depth);
        let meta = meta_comment(item);
        let _ = writeln!(buf, "{hashes} {}{meta}", format_title(item));
        if !item.note.is_empty() {
            let _ = writeln!(buf, "\n{}", item.note);
        }
        let children_are_checkboxes = depth >= max_heading_depth;
        if children_are_checkboxes && !item.items.is_empty() {
            let _ = writeln!(buf);
        }
        for child in &item.items {
            if !children_are_checkboxes {
                let _ = writeln!(buf);
            }
            render_item_safe(buf, child, depth + 1, max_heading_depth, 0);
        }
    } else {
        let indent = "  ".repeat(list_indent);
        let cb = checkbox(&item.completed);
        let meta = meta_comment(item);
        let _ = writeln!(buf, "{indent}- {cb} {}{meta}", format_title(item));
        if !item.note.is_empty() {
            let _ = writeln!(buf);
            let note_indent = "  ".repeat(list_indent + 1);
            for line in item.note.lines() {
                let _ = writeln!(buf, "{note_indent}{line}");
            }
            let _ = writeln!(buf);
        }
        for child in &item.items {
            render_item_safe(buf, child, max_heading_depth + 1, max_heading_depth, list_indent + 1);
        }
    }
}
#[cfg(test)]
#[path = "markdown_export_tests.rs"]
mod tests;
