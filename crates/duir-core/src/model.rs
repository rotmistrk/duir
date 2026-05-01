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
    /// If set, this node's children and note are encrypted.
    /// The cipher text contains the serialized children + note.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cipher: Option<String>,
    /// Runtime-only: true if currently decrypted in memory.
    #[serde(skip)]
    pub unlocked: bool,
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
            cipher: None,
            unlocked: false,
        }
    }

    /// Returns true if this node is an encryption root.
    #[must_use]
    pub const fn is_encrypted(&self) -> bool {
        self.cipher.is_some()
    }

    /// Returns true if this node is encrypted and currently locked.
    #[must_use]
    pub const fn is_locked(&self) -> bool {
        self.cipher.is_some() && !self.unlocked
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
