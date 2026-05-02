use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// Application configuration.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub storage: StorageConfig,
    pub editor: EditorConfig,
    pub ui: UiConfig,
    pub diagrams: crate::diagram::ToolPaths,
}

/// Storage paths configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct StorageConfig {
    pub central: PathBuf,
    pub local: PathBuf,
}

/// Editor defaults.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct EditorConfig {
    pub autosave: bool,
    pub autosave_interval_secs: u64,
    pub tab_width: u8,
    pub line_numbers: bool,
}

/// UI defaults.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct UiConfig {
    pub note_panel_pct: u16,
}

impl Default for StorageConfig {
    fn default() -> Self {
        let data_dir = dirs::data_dir().unwrap_or_else(|| PathBuf::from(".")).join("duir");
        Self {
            central: data_dir,
            local: PathBuf::from(".duir"),
        }
    }
}

impl Default for EditorConfig {
    fn default() -> Self {
        Self {
            autosave: true,
            autosave_interval_secs: 30,
            tab_width: 4,
            line_numbers: false,
        }
    }
}

impl Default for UiConfig {
    fn default() -> Self {
        Self { note_panel_pct: 50 }
    }
}

impl Config {
    /// Load config by checking (in order):
    /// 1. `.duir/config.toml` (project-local)
    /// 2. `~/.duirrc` (user shorthand)
    /// 3. `$XDG_CONFIG_HOME/duir/config.toml` (global)
    ///
    /// Falls back to defaults if none found.
    #[must_use]
    pub fn load() -> Self {
        // Project-local
        let local = Path::new(".duir/config.toml");
        if let Some(cfg) = Self::try_load(local) {
            return cfg;
        }

        // User shorthand
        if let Some(home) = dirs::home_dir() {
            let rc = home.join(".duirrc");
            if let Some(cfg) = Self::try_load(&rc) {
                return cfg;
            }
        }

        // XDG global
        if let Some(config_dir) = dirs::config_dir() {
            let global = config_dir.join("duir").join("config.toml");
            if let Some(cfg) = Self::try_load(&global) {
                return cfg;
            }
        }

        Self::default()
    }

    fn try_load(path: &Path) -> Option<Self> {
        let content = fs::read_to_string(path).ok()?;
        toml::from_str(&content).ok()
    }

    /// Write config to the given path.
    ///
    /// # Errors
    /// Returns an error if the file cannot be written.
    pub fn write_to(&self, path: &Path) -> crate::Result<()> {
        let content = toml::to_string_pretty(self).map_err(|e| crate::OmelaError::Other(e.to_string()))?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| crate::OmelaError::io(path, e))?;
        }
        fs::write(path, content).map_err(|e| crate::OmelaError::io(path, e))?;
        Ok(())
    }

    /// Initialize a local `.duir/` directory with config and empty data dir.
    ///
    /// # Errors
    /// Returns an error if the directory cannot be created.
    pub fn init_local(&self) -> crate::Result<()> {
        let dir = Path::new(".duir");
        fs::create_dir_all(dir).map_err(|e| crate::OmelaError::io(dir, e))?;
        self.write_to(&dir.join("config.toml"))?;
        Ok(())
    }

    /// Check if a local `.duir/` directory exists.
    #[must_use]
    pub fn has_local() -> bool {
        Path::new(".duir").is_dir()
    }

    /// Get all storage directories to load from (central + local if present).
    #[must_use]
    pub fn storage_dirs(&self) -> Vec<PathBuf> {
        let mut dirs = vec![self.storage.central.clone()];
        if Self::has_local() {
            dirs.push(self.storage.local.clone());
        }
        dirs
    }
}
