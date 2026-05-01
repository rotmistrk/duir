use std::fs;
use std::path::{Path, PathBuf};

use crate::error::{OmelaError, Result};
use crate::model::TodoFile;
use crate::storage::TodoStorage;

const EXTENSION: &str = ".todo.json";

pub struct FileStorage {
    base_dir: PathBuf,
}

impl FileStorage {
    /// Create a new `FileStorage` rooted at the given directory.
    /// Creates the directory if it does not exist.
    ///
    /// # Errors
    /// Returns an error if the directory cannot be created.
    pub fn new(base_dir: impl Into<PathBuf>) -> Result<Self> {
        let base_dir = base_dir.into();
        fs::create_dir_all(&base_dir).map_err(|e| OmelaError::io(&base_dir, e))?;
        Ok(Self { base_dir })
    }

    fn file_path(&self, name: &str) -> PathBuf {
        self.base_dir.join(format!("{name}{EXTENSION}"))
    }
}

impl TodoStorage for FileStorage {
    fn list(&self) -> Result<Vec<String>> {
        let mut names = Vec::new();
        let entries = fs::read_dir(&self.base_dir).map_err(|e| OmelaError::io(&self.base_dir, e))?;
        for entry in entries {
            let entry = entry.map_err(|e| OmelaError::io(&self.base_dir, e))?;
            if let Some(name) = entry.file_name().to_str()
                && let Some(stem) = name.strip_suffix(EXTENSION)
            {
                names.push(stem.to_owned());
            }
        }
        names.sort();
        Ok(names)
    }

    fn load(&self, name: &str) -> Result<TodoFile> {
        let path = self.file_path(name);
        let content = fs::read_to_string(&path).map_err(|e| OmelaError::io(&path, e))?;
        let file: TodoFile = serde_json::from_str(&content)?;
        Ok(file)
    }

    fn save(&self, name: &str, file: &TodoFile) -> Result<()> {
        let path = self.file_path(name);
        let content = serde_json::to_string_pretty(file)?;
        fs::write(&path, content).map_err(|e| OmelaError::io(&path, e))?;
        Ok(())
    }

    fn delete(&self, name: &str) -> Result<()> {
        let path = self.file_path(name);
        fs::remove_file(&path).map_err(|e| OmelaError::io(&path, e))?;
        Ok(())
    }

    fn exists(&self, name: &str) -> Result<bool> {
        Ok(self.file_path(name).exists())
    }
}

/// Export a `TodoFile` as YAML string.
///
/// # Errors
/// Returns an error if serialization fails.
pub fn to_yaml(file: &TodoFile) -> Result<String> {
    Ok(serde_yaml::to_string(file)?)
}

/// Import a `TodoFile` from YAML string.
///
/// # Errors
/// Returns an error if the YAML is invalid.
pub fn from_yaml(content: &str) -> Result<TodoFile> {
    Ok(serde_yaml::from_str(content)?)
}

/// Import a `TodoFile` from JSON string.
///
/// # Errors
/// Returns an error if the JSON is invalid.
pub fn from_json(content: &str) -> Result<TodoFile> {
    Ok(serde_json::from_str(content)?)
}

/// Detect format and import: tries JSON first, falls back to YAML.
///
/// # Errors
/// Returns an error if neither JSON nor YAML parsing succeeds.
pub fn from_auto(content: &str) -> Result<TodoFile> {
    from_json(content).or_else(|_| from_yaml(content))
}

/// Load a `TodoFile` from an arbitrary path (auto-detecting format).
///
/// # Errors
/// Returns an error if the file cannot be read or parsed.
pub fn load_path(path: &Path) -> Result<TodoFile> {
    let content = fs::read_to_string(path).map_err(|e| OmelaError::io(path, e))?;
    from_auto(&content)
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;
    use crate::model::TodoItem;

    fn make_test_file() -> TodoFile {
        let mut file = TodoFile::new("test-project");
        let mut task = TodoItem::new("Task 1");
        task.important = true;
        task.note = "Some **markdown** note".to_owned();
        task.items.push(TodoItem::new("Subtask 1.1"));
        file.items.push(task);
        file
    }

    #[test]
    fn file_storage_round_trip() {
        let dir = tempfile::tempdir().expect("tempdir");
        let storage = FileStorage::new(dir.path()).expect("new");

        let file = make_test_file();
        storage.save("myproject", &file).expect("save");

        assert!(storage.exists("myproject").expect("exists"));

        let loaded = storage.load("myproject").expect("load");
        assert_eq!(loaded.title, "test-project");
        assert_eq!(loaded.items.len(), 1);
        assert!(loaded.items[0].important);
        assert_eq!(loaded.items[0].items[0].title, "Subtask 1.1");

        let names = storage.list().expect("list");
        assert_eq!(names, vec!["myproject"]);

        storage.delete("myproject").expect("delete");
        assert!(!storage.exists("myproject").expect("exists after delete"));
    }

    #[test]
    fn yaml_round_trip() {
        let file = make_test_file();
        let yaml = to_yaml(&file).expect("to_yaml");
        let parsed = from_yaml(&yaml).expect("from_yaml");
        assert_eq!(parsed.title, file.title);
        assert_eq!(parsed.items.len(), file.items.len());
    }

    #[test]
    fn auto_detect_json() {
        let file = make_test_file();
        let json = serde_json::to_string_pretty(&file).expect("json");
        let parsed = from_auto(&json).expect("auto json");
        assert_eq!(parsed.title, "test-project");
    }

    #[test]
    fn auto_detect_yaml() {
        let file = make_test_file();
        let yaml = to_yaml(&file).expect("yaml");
        let parsed = from_auto(&yaml).expect("auto yaml");
        assert_eq!(parsed.title, "test-project");
    }

    #[test]
    fn load_path_json() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("test.json");
        let file = make_test_file();
        fs::write(&path, serde_json::to_string_pretty(&file).expect("json")).expect("write");
        let loaded = load_path(&path).expect("load_path");
        assert_eq!(loaded.title, "test-project");
    }
}
