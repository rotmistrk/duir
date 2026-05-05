use serde::{Deserialize, Serialize};

/// Stable node identity — persisted, survives tree mutations.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct NodeId(pub String);

impl NodeId {
    #[must_use]
    pub fn new() -> Self {
        Self(uuid::Uuid::new_v4().to_string())
    }
}

impl Default for NodeId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for NodeId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Completion {
    #[default]
    Open,
    Done,
    Partial,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum NodeType {
    Kiron,
    Prompt,
    Response,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KironMeta {
    pub session_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TodoItem {
    #[serde(default = "NodeId::new")]
    pub id: NodeId,
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub node_type: Option<NodeType>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kiron: Option<KironMeta>,
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
            id: NodeId::new(),
            title: title.to_owned(),
            completed: Completion::default(),
            important: false,
            folded: false,
            note: String::new(),
            items: Vec::new(),
            node_type: None,
            kiron: None,
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

    /// Returns true if this node is a Kiron session node.
    #[must_use]
    pub const fn is_kiron(&self) -> bool {
        matches!(self.node_type, Some(NodeType::Kiron))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    type TestResult = Result<(), Box<dyn std::error::Error>>;

    #[test]
    fn round_trip_json() -> TestResult {
        let mut file = TodoFile::new("test");
        let mut task = TodoItem::new("Task 1");
        task.items.push(TodoItem::new("Subtask 1.1"));
        file.items.push(task);

        let json = serde_json::to_string_pretty(&file)?;
        let parsed: TodoFile = serde_json::from_str(&json)?;

        assert_eq!(parsed.title, "test");
        assert_eq!(parsed.items.len(), 1);
        let first = parsed.items.first().ok_or("no first item")?;
        assert_eq!(first.items.len(), 1);
        let sub = first.items.first().ok_or("no subtask")?;
        assert_eq!(sub.title, "Subtask 1.1");
        Ok(())
    }

    #[test]
    fn node_id_unique() {
        let a = TodoItem::new("A");
        let b = TodoItem::new("B");
        assert_ne!(a.id, b.id);
    }

    #[test]
    fn node_id_serialization_roundtrip() -> TestResult {
        let item = TodoItem::new("persist me");
        let json = serde_json::to_string(&item)?;
        let parsed: TodoItem = serde_json::from_str(&json)?;
        assert_eq!(parsed.id, item.id);
        Ok(())
    }

    #[test]
    fn node_id_legacy_compat() -> TestResult {
        let json = r#"{"title":"old item"}"#;
        let parsed: TodoItem = serde_json::from_str(json)?;
        assert!(!parsed.id.0.is_empty());
        Ok(())
    }
}
