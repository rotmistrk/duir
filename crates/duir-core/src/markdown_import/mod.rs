//! Markdown import: parses a structured markdown document into a [`TodoFile`].

mod parse;

#[cfg(test)]
#[path = "tests.rs"]
mod tests;

use crate::model::{TodoFile, TodoItem};

use parse::{Line, classify_line};

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

    let mut heading_stack: Vec<(usize, TodoItem)> = Vec::new();
    let mut checkbox_stack: Vec<(usize, TodoItem)> = Vec::new();
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

    flush_checkbox_stack(&mut checkbox_stack, &mut heading_stack, &mut file);
    flush_headings_to_level(&mut heading_stack, 0, &mut file);

    file
}

enum NoteTarget {
    File,
    Heading,
    Checkbox,
}

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

fn collapse_stack(stack: &mut Vec<(usize, TodoItem)>) {
    let mut changed = true;
    while changed {
        changed = false;
        let mut i = stack.len();
        while i > 1 {
            i -= 1;
            let deeper = match (stack.get(i), stack.get(i.wrapping_sub(1))) {
                (Some(cur), Some(prev)) => cur.0 > prev.0,
                _ => false,
            };
            if deeper {
                let (_, child) = stack.remove(i);
                if let Some(parent) = stack.get_mut(i.wrapping_sub(1)) {
                    parent.1.items.push(child);
                }
                changed = true;
            }
        }
    }
}

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
