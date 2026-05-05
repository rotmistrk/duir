use crate::error::Result;
use crate::model::TodoFile;

pub trait TodoStorage {
    /// List available file names (without extension).
    ///
    /// # Errors
    /// Returns an error if the storage backend cannot be read.
    fn list(&self) -> Result<Vec<String>>;

    /// Load a todo file by name.
    ///
    /// # Errors
    /// Returns an error if the file does not exist or cannot be parsed.
    fn load(&self, name: &str) -> Result<TodoFile>;

    /// Save a todo file by name.
    ///
    /// # Errors
    /// Returns an error if the file cannot be written.
    fn save(&self, name: &str, file: &TodoFile) -> Result<()>;

    /// Delete a todo file by name.
    ///
    /// # Errors
    /// Returns an error if the file does not exist or cannot be removed.
    fn delete(&self, name: &str) -> Result<()>;

    /// Check if a file exists.
    ///
    /// # Errors
    /// Returns an error if the storage backend cannot be queried.
    fn exists(&self, name: &str) -> Result<bool>;

    /// Get the modification time of a file (if supported).
    fn mtime(&self, name: &str) -> Option<std::time::SystemTime> {
        let _ = name;
        None
    }
}
