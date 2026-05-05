use duir_core::tree_ops::TreePath;
use duir_core::{Completion, TodoFile};

use super::FileId;

/// Bitflags for boolean properties of a tree row.
#[derive(Debug, Clone, Copy, Default)]
pub struct TreeRowFlags(u16);

impl TreeRowFlags {
    const IMPORTANT: u16 = 1;
    const EXPANDED: u16 = 1 << 1;
    const HAS_CHILDREN: u16 = 1 << 2;
    const IS_FILE_ROOT: u16 = 1 << 3;
    const ENCRYPTED: u16 = 1 << 4;
    const LOCKED: u16 = 1 << 5;
    const HAS_ENCRYPTED_CHILDREN: u16 = 1 << 6;
    const IS_KIRON: u16 = 1 << 7;
    const KIRO_ACTIVE: u16 = 1 << 8;

    const fn has(self, flag: u16) -> bool {
        self.0 & flag != 0
    }

    const fn set(&mut self, flag: u16, value: bool) {
        if value {
            self.0 |= flag;
        } else {
            self.0 &= !flag;
        }
    }

    #[must_use]
    pub const fn important(self) -> bool {
        self.has(Self::IMPORTANT)
    }
    #[must_use]
    pub const fn expanded(self) -> bool {
        self.has(Self::EXPANDED)
    }
    #[must_use]
    pub const fn has_children(self) -> bool {
        self.has(Self::HAS_CHILDREN)
    }
    #[must_use]
    pub const fn is_file_root(self) -> bool {
        self.has(Self::IS_FILE_ROOT)
    }
    #[must_use]
    pub const fn encrypted(self) -> bool {
        self.has(Self::ENCRYPTED)
    }
    #[must_use]
    pub const fn locked(self) -> bool {
        self.has(Self::LOCKED)
    }
    #[must_use]
    pub const fn has_encrypted_children(self) -> bool {
        self.has(Self::HAS_ENCRYPTED_CHILDREN)
    }
    #[must_use]
    pub const fn is_kiron(self) -> bool {
        self.has(Self::IS_KIRON)
    }
    #[must_use]
    pub const fn kiro_active(self) -> bool {
        self.has(Self::KIRO_ACTIVE)
    }

    pub const fn set_important(&mut self, v: bool) {
        self.set(Self::IMPORTANT, v);
    }
    pub const fn set_expanded(&mut self, v: bool) {
        self.set(Self::EXPANDED, v);
    }
    pub const fn set_has_children(&mut self, v: bool) {
        self.set(Self::HAS_CHILDREN, v);
    }
    pub const fn set_is_file_root(&mut self, v: bool) {
        self.set(Self::IS_FILE_ROOT, v);
    }
    pub const fn set_encrypted(&mut self, v: bool) {
        self.set(Self::ENCRYPTED, v);
    }
    pub const fn set_locked(&mut self, v: bool) {
        self.set(Self::LOCKED, v);
    }
    pub const fn set_has_encrypted_children(&mut self, v: bool) {
        self.set(Self::HAS_ENCRYPTED_CHILDREN, v);
    }
    pub const fn set_is_kiron(&mut self, v: bool) {
        self.set(Self::IS_KIRON, v);
    }
    pub const fn set_kiro_active(&mut self, v: bool) {
        self.set(Self::KIRO_ACTIVE, v);
    }
}

/// A flattened row in the tree view, used for rendering and navigation.
#[derive(Debug, Clone)]
pub struct TreeRow {
    pub path: TreePath,
    pub depth: usize,
    pub title: String,
    pub completed: Completion,
    pub flags: TreeRowFlags,
    pub stats_text: String,
    pub file_index: usize,
    #[allow(dead_code)]
    pub file_id: FileId,
    pub file_source: Option<FileSource>,
}

/// Where a file was loaded from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileSource {
    Central,
    Local,
}

/// Loaded file with its data and metadata.
#[derive(Debug)]
pub struct LoadedFile {
    pub id: FileId,
    pub name: String,
    pub data: TodoFile,
    pub source: FileSource,
    pub(crate) modified: bool,
    pub autosave: bool,
    /// Disk mtime when last loaded/saved, for conflict detection.
    pub disk_mtime: Option<std::time::SystemTime>,
    /// True when disk changed while we have local modifications.
    pub conflicted: bool,
}

impl LoadedFile {
    #[must_use]
    pub const fn is_modified(&self) -> bool {
        self.modified
    }
}
