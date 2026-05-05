use quick_xml::Reader;
use quick_xml::events::Event;

use crate::model::{Completion, TodoFile, TodoItem};

/// Import a legacy `.todo` XML file from the Qt `ToDo` app.
///
/// # Errors
/// Returns an error if the XML cannot be parsed.
pub fn import_legacy_todo(content: &str) -> crate::Result<TodoFile> {
    let mut reader = Reader::from_str(content);
    let mut file = TodoFile::new("Imported");
    let mut stack: Vec<TodoItem> = Vec::new();
    let mut in_note = false;
    let mut note_buf = String::new();

    loop {
        match reader.read_event() {
            Ok(ref evt @ (Event::Start(ref e) | Event::Empty(ref e))) => {
                let is_empty = matches!(evt, Event::Empty(_));
                let name = String::from_utf8_lossy(e.name().as_ref()).to_string();
                match name.as_str() {
                    "item" => {
                        let mut item = TodoItem::new("");
                        for attr in e.attributes().flatten() {
                            let key = String::from_utf8_lossy(attr.key.as_ref()).to_string();
                            let val = String::from_utf8_lossy(&attr.value).to_string();
                            match key.as_str() {
                                "title" => item.title = val,
                                "folded" => item.folded = val == "yes",
                                "important" => item.important = val == "yes",
                                "completed" => {
                                    item.completed = match val.as_str() {
                                        "yes" => Completion::Done,
                                        "part" => Completion::Partial,
                                        _ => Completion::Open,
                                    };
                                }
                                _ => {}
                            }
                        }
                        stack.push(item);
                        if is_empty && let Some(item) = stack.pop() {
                            if let Some(parent) = stack.last_mut() {
                                parent.items.push(item);
                            } else {
                                file.items.push(item);
                            }
                        }
                    }
                    "note" => {
                        in_note = true;
                        note_buf.clear();
                    }
                    _ => {}
                }
            }
            Ok(Event::End(ref e)) => {
                let name = String::from_utf8_lossy(e.name().as_ref()).to_string();
                match name.as_str() {
                    "item" => {
                        if let Some(item) = stack.pop() {
                            if let Some(parent) = stack.last_mut() {
                                parent.items.push(item);
                            } else {
                                file.items.push(item);
                            }
                        }
                    }
                    "note" => {
                        in_note = false;
                        let md = html_to_markdown(&note_buf);
                        if let Some(item) = stack.last_mut() {
                            item.note = md;
                        } else {
                            file.note = md;
                        }
                    }
                    _ => {}
                }
            }
            Ok(Event::Text(ref e)) => {
                if in_note {
                    let text = e.unescape().unwrap_or_default();
                    note_buf.push_str(&text);
                }
            }
            Ok(Event::Eof) => break,
            Err(e) => {
                return Err(crate::OmelaError::Other(format!(
                    "XML parse error at {}: {e}",
                    reader.error_position()
                )));
            }
            _ => {}
        }
    }

    // Set title from first item if file title is default
    if file.title == "Imported"
        && let Some(first) = file.items.first()
        && !first.title.is_empty()
    {
        file.title.clone_from(&first.title);
    }

    Ok(file)
}

/// Convert Qt rich text HTML to plain markdown.
/// Strips HTML tags, extracts text content from `<p>` and `<span>` elements.
fn html_to_markdown(html: &str) -> String {
    let trimmed = html.trim();
    if trimmed.is_empty() {
        return String::new();
    }

    let mut result = String::new();
    let mut in_tag = false;
    let mut tag_buf = String::new();
    let mut pending_newline = false;

    for ch in trimmed.chars() {
        if ch == '<' {
            in_tag = true;
            tag_buf.clear();
            continue;
        }
        if ch == '>' {
            in_tag = false;
            let tag_lower = tag_buf.to_lowercase();
            // Block-level tags get newlines
            if tag_lower.starts_with("p ")
                || tag_lower == "p"
                || tag_lower.starts_with("/p")
                || tag_lower == "br"
                || tag_lower == "br /"
            {
                pending_newline = true;
            }
            // Bold
            if tag_lower == "b" || tag_lower.starts_with("span") && tag_lower.contains("bold") {
                result.push_str("**");
            }
            if tag_lower == "/b" || tag_lower == "/span" && result.ends_with("**") {
                // closing bold handled by context
            }
            continue;
        }
        if in_tag {
            tag_buf.push(ch);
            continue;
        }
        // Text content
        if pending_newline {
            if !result.is_empty() {
                result.push('\n');
            }
            pending_newline = false;
        }
        result.push(ch);
    }

    // Clean up: remove empty lines at start/end, collapse multiple newlines
    let lines: Vec<&str> = result.lines().map(str::trim).collect();

    // Remove leading/trailing empty lines
    let start = lines.iter().position(|l| !l.is_empty()).unwrap_or(0);
    let end = lines.iter().rposition(|l| !l.is_empty()).map_or(0, |i| i + 1);

    if start >= end {
        return String::new();
    }

    lines.get(start..end).unwrap_or_default().join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    type TestResult = Result<(), Box<dyn std::error::Error>>;

    #[test]
    fn parse_simple_todo() -> TestResult {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE todo-tree SYSTEM 'todo-tree.dtd'>
<todo-tree version="1.1">
  <item title="Task 1" folded="no" important="yes" completed="no">
  </item>
  <item title="Task 2" folded="yes" important="no" completed="yes">
  </item>
</todo-tree>"#;

        let file = import_legacy_todo(xml)?;
        assert_eq!(file.items.len(), 2);
        let first = file.items.first().ok_or("no first")?;
        assert_eq!(first.title, "Task 1");
        assert!(first.important);
        assert!(!first.folded);
        let second = file.items.get(1).ok_or("no second")?;
        assert_eq!(second.title, "Task 2");
        assert!(second.folded);
        assert_eq!(second.completed, Completion::Done);
        Ok(())
    }

    #[test]
    fn parse_nested() -> TestResult {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<todo-tree version="1.1">
  <item title="Parent" folded="no" important="no" completed="no">
    <item title="Child 1" folded="no" important="no" completed="yes"/>
    <item title="Child 2" folded="no" important="no" completed="no"/>
  </item>
</todo-tree>"#;

        let file = import_legacy_todo(xml)?;
        assert_eq!(file.items.len(), 1);
        let parent = file.items.first().ok_or("no parent")?;
        assert_eq!(parent.title, "Parent");
        assert_eq!(parent.items.len(), 2);
        let child1 = parent.items.first().ok_or("no child1")?;
        assert_eq!(child1.title, "Child 1");
        assert_eq!(child1.completed, Completion::Done);
        Ok(())
    }

    #[test]
    fn parse_note_with_html() -> TestResult {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<todo-tree version="1.1">
  <item title="With Note" folded="no" important="no" completed="no">
    <note>
&lt;html&gt;&lt;body&gt;&lt;p&gt;Hello world&lt;/p&gt;&lt;p&gt;Second line&lt;/p&gt;&lt;/body&gt;&lt;/html&gt;
    </note>
  </item>
</todo-tree>"#;

        let file = import_legacy_todo(xml)?;
        let first = file.items.first().ok_or("no first")?;
        assert_eq!(first.title, "With Note");
        assert!(first.note.contains("Hello world"));
        assert!(first.note.contains("Second line"));
        Ok(())
    }

    #[test]
    fn html_to_md_strips_tags() {
        let html = r"<html><body><p>Hello</p><p>World</p></body></html>";
        let md = html_to_markdown(html);
        assert!(md.contains("Hello"));
        assert!(md.contains("World"));
        assert!(!md.contains('<'));
    }

    #[test]
    fn html_to_md_empty() {
        assert_eq!(html_to_markdown(""), "");
        assert_eq!(html_to_markdown("   "), "");
    }

    #[test]
    fn partial_completion() -> TestResult {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<todo-tree version="1.1">
  <item title="Partial" folded="no" important="no" completed="part"/>
</todo-tree>"#;

        let file = import_legacy_todo(xml)?;
        let first = file.items.first().ok_or("no first")?;
        assert_eq!(first.completed, Completion::Partial);
        Ok(())
    }
}
