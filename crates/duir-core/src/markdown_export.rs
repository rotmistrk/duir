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
#[allow(clippy::expect_used)]
mod tests {
    use pretty_assertions::assert_eq;

    use super::*;

    fn item(title: &str) -> TodoItem {
        TodoItem::new(title)
    }

    fn done_item(title: &str) -> TodoItem {
        TodoItem {
            completed: Completion::Done,
            ..TodoItem::new(title)
        }
    }

    fn important_item(title: &str) -> TodoItem {
        TodoItem {
            important: true,
            ..TodoItem::new(title)
        }
    }

    #[test]
    fn single_item_heading() {
        let i = item("Hello");
        let md = export_subtree(&i, 3);
        assert_eq!(md, "# Hello\n");
    }

    #[test]
    fn important_item_bold() {
        let i = important_item("Urgent");
        let md = export_subtree(&i, 3);
        assert_eq!(md, "# **Urgent** <!-- important -->\n");
    }

    #[test]
    fn nested_headings() {
        let mut root = item("Root");
        root.items.push(item("Child"));
        let md = export_subtree(&root, 3);
        assert_eq!(md, "# Root\n\n## Child\n");
    }

    #[test]
    fn heading_to_checkbox_transition() {
        let mut l2 = item("Level 2");
        let mut l3 = item("Level 3");
        l3.items.push(done_item("Level 4"));
        l2.items.push(l3);
        let mut root = item("Level 1");
        root.items.push(l2);

        let md = export_subtree(&root, 3);
        let expected = "\
# Level 1

## Level 2

### Level 3

- [x] Level 4
";
        assert_eq!(md, expected);
    }

    #[test]
    fn nested_checkboxes() {
        let mut l3 = item("L3");
        let mut deep = item("L4");
        deep.items.push(done_item("L5"));
        l3.items.push(deep);
        let mut l2 = item("L2");
        l2.items.push(l3);
        let mut root = item("L1");
        root.items.push(l2);

        let md = export_subtree(&root, 2);
        let expected = "\
# L1

## L2

- [ ] L3
  - [ ] L4
    - [x] L5
";
        assert_eq!(md, expected);
    }

    #[test]
    fn completion_states() {
        let mut root = item("Root");
        root.items.push(item("Open"));
        root.items.push(done_item("Done"));
        root.items.push(TodoItem {
            completed: Completion::Partial,
            ..TodoItem::new("Partial")
        });

        let md = export_subtree(&root, 1);
        let expected = "\
# Root

- [ ] Open
- [x] Done
- [-] Partial
";
        assert_eq!(md, expected);
    }

    #[test]
    fn item_with_note() {
        let mut i = item("Task");
        i.note = "Some details here.".to_owned();
        let md = export_subtree(&i, 3);
        let expected = "\
# Task

Some details here.
";
        assert_eq!(md, expected);
    }

    #[test]
    fn checkbox_with_note() {
        let mut child = done_item("Sub");
        child.note = "Note line.".to_owned();
        let mut root = item("Root");
        root.items.push(child);

        let md = export_subtree(&root, 1);
        let expected = "\
# Root

- [x] Sub

  Note line.

";
        assert_eq!(md, expected);
    }

    #[test]
    fn important_checkbox() {
        let mut root = item("Root");
        root.items.push(important_item("Bold"));
        let md = export_subtree(&root, 1);
        assert!(md.contains("- [ ] **Bold**"));
    }

    #[test]
    fn export_file_basic() {
        let mut file = TodoFile::new("My List");
        file.items.push(item("First"));
        file.items.push(done_item("Second"));

        let md = export_file(&file);
        let expected = "\
# My List

## First

## Second
";
        assert_eq!(md, expected);
    }

    #[test]
    fn export_file_with_note() {
        let mut file = TodoFile::new("Project");
        file.note = "Project description.".to_owned();
        file.items.push(item("Task"));

        let md = export_file(&file);
        assert!(md.starts_with("# Project\n\nProject description.\n"));
        assert!(md.contains("## Task\n"));
    }

    #[test]
    fn export_file_deep_tree() {
        let mut file = TodoFile::new("Deep");
        let mut l1 = item("A");
        let mut l2 = item("B");
        l2.items.push(done_item("C"));
        l1.items.push(l2);
        file.items.push(l1);

        let md = export_file(&file);
        // file title = #, top-level = ##, child = ###, grandchild = checkbox
        assert!(md.contains("## A\n"));
        assert!(md.contains("### B\n"));
        assert!(md.contains("- [x] C\n"));
    }
}
