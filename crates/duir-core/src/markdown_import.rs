//! Markdown import: parses a structured markdown document into a [`TodoFile`].

use crate::model::{Completion, TodoFile, TodoItem};

/// Parsed line classification.
enum Line<'a> {
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
fn classify_line(line: &str) -> Line<'_> {
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
/// Returns (text without meta, folded, important from meta).
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

/// Import a markdown string into a [`TodoFile`].
///
/// # Parsing rules
///
/// - Lines starting with `#` become tree nodes; heading level determines depth.
/// - Lines matching `- [x]`, `- [ ]`, or `- [-]` become leaf items with the
///   appropriate [`Completion`] state. Indentation (2 or 4 spaces per level)
///   controls nesting depth.
/// - Bold checkbox items (`- **text**`) set `important = true`.
/// - Text between headings/checkboxes becomes the note for the preceding item.
/// - The file title is taken from the first `#` heading, or `"Imported"` if none.
/// - Version is always `"2.0"`.
#[must_use]
pub fn import_markdown(content: &str) -> TodoFile {
    let mut file = TodoFile::new("Imported");
    let mut title_set = false;

    // Stack tracks (depth, item) for heading hierarchy.
    // heading_stack[i] corresponds to heading depth i+1.
    let mut heading_stack: Vec<(usize, TodoItem)> = Vec::new();
    // Separate stack for checkbox nesting within current heading.
    let mut checkbox_stack: Vec<(usize, TodoItem)> = Vec::new();
    // Track what the last emitted item was, so notes attach correctly.
    // `NoteTarget::File` = before any item, `Heading` = last was heading, `Checkbox` = last was checkbox.
    let mut note_target = NoteTarget::File;

    for line in content.lines() {
        let classified = classify_line(line);
        match classified {
            Line::Heading {
                level,
                text,
                folded,
                important,
            } => {
                flush_checkbox_stack(&mut checkbox_stack, &mut heading_stack, &mut file);

                if !title_set {
                    text.clone_into(&mut file.title);
                    title_set = true;
                }

                flush_headings_to_level(&mut heading_stack, level, &mut file);

                let mut item = TodoItem::new(text);
                item.folded = folded;
                item.important = important;
                heading_stack.push((level, item));
                note_target = NoteTarget::Heading;
            }
            Line::Checkbox {
                depth,
                state,
                text,
                important,
                folded,
            } => {
                flush_checkboxes_to_depth(&mut checkbox_stack, depth);

                let mut item = TodoItem::new(text);
                item.completed = state;
                item.important = important;
                item.folded = folded;
                checkbox_stack.push((depth, item));
                note_target = NoteTarget::Checkbox;
            }
            Line::Text(text) => {
                let trimmed = text.trim();
                if trimmed.is_empty() {
                    continue;
                }
                append_note(
                    &note_target,
                    trimmed,
                    &mut heading_stack,
                    &mut checkbox_stack,
                    &mut file,
                );
            }
        }
    }

    // Flush remaining stacks.
    flush_checkbox_stack(&mut checkbox_stack, &mut heading_stack, &mut file);
    flush_headings_to_level(&mut heading_stack, 0, &mut file);

    file
}

enum NoteTarget {
    File,
    Heading,
    Checkbox,
}

/// Append note text to the appropriate target.
fn append_note(
    target: &NoteTarget,
    text: &str,
    heading_stack: &mut [(usize, TodoItem)],
    checkbox_stack: &mut [(usize, TodoItem)],
    file: &mut TodoFile,
) {
    let note = match target {
        NoteTarget::Checkbox => checkbox_stack.last_mut().map(|(_, item)| &mut item.note),
        NoteTarget::Heading => heading_stack.last_mut().map(|(_, item)| &mut item.note),
        NoteTarget::File => None,
    }
    .unwrap_or(&mut file.note);

    if note.is_empty() {
        text.clone_into(note);
    } else {
        note.push('\n');
        note.push_str(text);
    }
}

/// Flush checkbox stack items deeper than `target_depth`, folding children into parents.
fn flush_checkboxes_to_depth(stack: &mut Vec<(usize, TodoItem)>, target_depth: usize) {
    while stack.len() > 1 {
        let last_depth = stack.last().map_or(0, |(d, _)| *d);
        if last_depth <= target_depth {
            break;
        }
        let prev_depth = stack.get(stack.len() - 2).map_or(0, |(d, _)| *d);
        if last_depth <= prev_depth {
            break;
        }
        if let Some((_, child)) = stack.pop()
            && let Some((_, parent)) = stack.last_mut()
        {
            parent.items.push(child);
        }
    }
}

/// Flush all checkboxes into the current heading (or file root).
fn flush_checkbox_stack(
    checkbox_stack: &mut Vec<(usize, TodoItem)>,
    heading_stack: &mut [(usize, TodoItem)],
    file: &mut TodoFile,
) {
    collapse_stack(checkbox_stack);
    let items: Vec<TodoItem> = checkbox_stack.drain(..).map(|(_, item)| item).collect();
    if let Some((_, heading)) = heading_stack.last_mut() {
        heading.items.extend(items);
    } else {
        file.items.extend(items);
    }
}

/// Collapse a depth-tagged stack so that deeper items become children of shallower ones.
fn collapse_stack(stack: &mut Vec<(usize, TodoItem)>) {
    // Process from end: fold any item deeper than its predecessor.
    // Repeat until no more folds are possible.
    let mut changed = true;
    while changed {
        changed = false;
        let mut i = stack.len();
        while i > 1 {
            i -= 1;
            if stack[i].0 > stack[i - 1].0 {
                let (_, child) = stack.remove(i);
                stack[i - 1].1.items.push(child);
                changed = true;
            }
        }
    }
}

/// Flush headings with level >= `target_level`, folding children into parents.
fn flush_headings_to_level(stack: &mut Vec<(usize, TodoItem)>, target_level: usize, file: &mut TodoFile) {
    while let Some(&(level, _)) = stack.last() {
        if level < target_level {
            break;
        }
        if let Some((_, child)) = stack.pop() {
            if let Some((_, parent)) = stack.last_mut() {
                parent.items.push(child);
            } else {
                file.items.push(child);
            }
        }
    }
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn headings_only() {
        let md = "# Project\n## Phase 1\n## Phase 2\n### Sub-phase";
        let file = import_markdown(md);

        assert_eq!(file.title, "Project");
        assert_eq!(file.version, "2.0");
        assert_eq!(file.items.len(), 1);
        assert_eq!(file.items[0].title, "Project");
        assert_eq!(file.items[0].items.len(), 2);
        assert_eq!(file.items[0].items[0].title, "Phase 1");
        assert_eq!(file.items[0].items[1].title, "Phase 2");
        assert_eq!(file.items[0].items[1].items.len(), 1);
        assert_eq!(file.items[0].items[1].items[0].title, "Sub-phase");
    }

    #[test]
    fn checkboxes_only() {
        let md = "- [ ] Open task\n- [x] Done task\n- [-] Partial task";
        let file = import_markdown(md);

        assert_eq!(file.title, "Imported");
        assert_eq!(file.items.len(), 3);
        assert_eq!(file.items[0].completed, Completion::Open);
        assert_eq!(file.items[1].completed, Completion::Done);
        assert_eq!(file.items[2].completed, Completion::Partial);
    }

    #[test]
    fn mixed_headings_and_checkboxes() {
        let md = "# Tasks\n- [ ] Task A\n- [x] Task B\n## Section\n- [ ] Task C";
        let file = import_markdown(md);

        assert_eq!(file.items.len(), 1);
        let root = &file.items[0];
        assert_eq!(root.title, "Tasks");
        // Task A, Task B, then Section heading
        assert_eq!(root.items.len(), 3);
        assert_eq!(root.items[0].title, "Task A");
        assert_eq!(root.items[1].title, "Task B");
        assert_eq!(root.items[2].title, "Section");
        assert_eq!(root.items[2].items[0].title, "Task C");
    }

    #[test]
    fn nested_lists() {
        let md = "- [ ] Parent\n  - [ ] Child 1\n  - [x] Child 2\n    - [ ] Grandchild";
        let file = import_markdown(md);

        assert_eq!(file.items.len(), 1);
        assert_eq!(file.items[0].title, "Parent");
        assert_eq!(file.items[0].items.len(), 2);
        assert_eq!(file.items[0].items[0].title, "Child 1");
        assert_eq!(file.items[0].items[1].title, "Child 2");
        assert_eq!(file.items[0].items[1].items.len(), 1);
        assert_eq!(file.items[0].items[1].items[0].title, "Grandchild");
    }

    #[test]
    fn notes_attached_to_items() {
        let md = "# Heading\nSome note text\nMore notes\n- [ ] Task\nTask note here";
        let file = import_markdown(md);

        assert_eq!(file.items[0].note, "Some note text\nMore notes");
        assert_eq!(file.items[0].items[0].note, "Task note here");
    }

    #[test]
    fn importance_bold() {
        let md = "- **Important item**\n- [ ] **Bold open**\n- [x] **Bold done**";
        let file = import_markdown(md);

        assert!(file.items[0].important);
        assert_eq!(file.items[0].title, "Important item");
        assert!(file.items[1].important);
        assert_eq!(file.items[1].title, "Bold open");
        assert!(file.items[2].important);
        assert_eq!(file.items[2].title, "Bold done");
    }

    #[test]
    fn no_heading_uses_default_title() {
        let md = "- [ ] Just a task";
        let file = import_markdown(md);

        assert_eq!(file.title, "Imported");
    }

    #[test]
    fn empty_input() {
        let file = import_markdown("");

        assert_eq!(file.title, "Imported");
        assert!(file.items.is_empty());
    }
}
