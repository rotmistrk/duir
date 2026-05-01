use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Completion {
    #[default]
    Open,
    Done,
    Partial,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TodoItem {
    pub title: String,
    #[serde(default)]
    pub completed: Completion,
    #[serde(default)]
    pub important: bool,
    #[serde(default)]
    pub folded: bool,
    #[serde(default)]
    pub note: String,
    #[serde(default)]
    pub items: Vec<Self>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TodoFile {
    pub version: String,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub note: String,
    #[serde(default)]
    pub items: Vec<TodoItem>,
}

impl TodoFile {
    #[must_use]
    pub fn new(title: &str) -> Self {
        Self {
            version: "2.0".to_owned(),
            title: title.to_owned(),
            note: String::new(),
            items: Vec::new(),
        }
    }
}

impl TodoItem {
    #[must_use]
    pub fn new(title: &str) -> Self {
        Self {
            title: title.to_owned(),
            completed: Completion::default(),
            important: false,
            folded: false,
            note: String::new(),
            items: Vec::new(),
        }
    }
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_json() {
        let mut file = TodoFile::new("test");
        let mut task = TodoItem::new("Task 1");
        task.items.push(TodoItem::new("Subtask 1.1"));
        file.items.push(task);

        let json = serde_json::to_string_pretty(&file).expect("serialize");
        let parsed: TodoFile = serde_json::from_str(&json).expect("deserialize");

        assert_eq!(parsed.title, "test");
        assert_eq!(parsed.items.len(), 1);
        assert_eq!(parsed.items[0].items.len(), 1);
        assert_eq!(parsed.items[0].items[0].title, "Subtask 1.1");
    }
}
