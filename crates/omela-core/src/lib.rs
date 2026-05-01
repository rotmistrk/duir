pub mod error;
pub mod file_storage;
pub mod model;
pub mod storage;

pub use error::{OmelaError, Result};
pub use file_storage::FileStorage;
pub use model::{Completion, TodoFile, TodoItem};
pub use storage::TodoStorage;
