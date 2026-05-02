/// Command completion state for dmenu-style suggestions.
pub struct Completer {
    commands: Vec<&'static str>,
    pub matches: Vec<&'static str>,
    pub selected: Option<usize>,
}

impl Completer {
    #[must_use]
    pub fn new(commands: &[&'static str]) -> Self {
        Self {
            commands: commands.to_vec(),
            matches: Vec::new(),
            selected: None,
        }
    }

    /// Update matches based on current input prefix.
    pub fn update(&mut self, input: &str) {
        self.matches = if input.is_empty() {
            self.commands.clone()
        } else {
            self.commands
                .iter()
                .filter(|cmd| cmd.starts_with(input))
                .copied()
                .collect()
        };
        if let Some(sel) = self.selected
            && sel >= self.matches.len()
        {
            self.selected = None;
        }
    }

    /// Cycle to next match. Returns the selected completion text.
    pub fn next(&mut self) -> Option<&'static str> {
        if self.matches.is_empty() {
            return None;
        }
        let idx = self.selected.map_or(0, |i| (i + 1) % self.matches.len());
        self.selected = Some(idx);
        self.matches.get(idx).copied()
    }

    /// Cycle to previous match.
    pub fn prev(&mut self) -> Option<&'static str> {
        if self.matches.is_empty() {
            return None;
        }
        let idx = self.selected.map_or(self.matches.len() - 1, |i| {
            if i == 0 { self.matches.len() - 1 } else { i - 1 }
        });
        self.selected = Some(idx);
        self.matches.get(idx).copied()
    }

    /// Reset selection (e.g., when user types a character).
    pub const fn reset_selection(&mut self) {
        self.selected = None;
    }
}

/// App-level commands.
pub const APP_COMMANDS: &[&str] = &[
    "about",
    "autosave",
    "autosave all",
    "collapse",
    "config",
    "config write",
    "decrypt",
    "e ",
    "encrypt",
    "expand",
    "export ",
    "help",
    "import ",
    "init",
    "kiro start",
    "kiro stop",
    "kiron",
    "kiron disable",
    "o ",
    "open ",
    "q",
    "q!",
    "qa",
    "saveas ",
    "w",
    "wa",
    "write ",
    "yank",
];

/// Editor-level commands.
pub const EDITOR_COMMANDS: &[&str] = &[
    "set nu",
    "set num",
    "set number",
    "set nonu",
    "set nonum",
    "set nonumber",
    "set li",
    "set list",
    "set noli",
    "set nolist",
];

/// Complete file/directory paths from the filesystem.
/// Returns matching entries for the given partial path.
pub fn complete_path(partial: &str) -> Vec<String> {
    // S3 path completion
    if partial.starts_with("s3://") {
        return complete_s3_path(partial);
    }

    let path = std::path::Path::new(partial);
    let (dir, prefix) = if partial.ends_with('/') {
        (std::path::PathBuf::from(partial), "")
    } else {
        let dir = path.parent().unwrap_or_else(|| std::path::Path::new("."));
        let prefix = path.file_name().and_then(|f| f.to_str()).unwrap_or("");
        (dir.to_path_buf(), prefix)
    };

    let expanded_dir = if dir.starts_with("~") {
        dirs::home_dir().map_or_else(|| dir.clone(), |home| home.join(dir.strip_prefix("~").unwrap_or(&dir)))
    } else {
        dir.clone()
    };

    let mut results = Vec::new();
    if let Ok(entries) = std::fs::read_dir(&expanded_dir) {
        for entry in entries.flatten() {
            let name = entry.file_name();
            let name_str = name.to_string_lossy();
            if !name_str.starts_with('.') && name_str.starts_with(prefix) {
                let is_dir = entry.file_type().is_ok_and(|t| t.is_dir());
                let full = if dir.as_os_str() == "." {
                    name_str.to_string()
                } else {
                    format!("{}/{name_str}", dir.display())
                };
                if is_dir {
                    results.push(format!("{full}/"));
                } else if name_str.ends_with(".md")
                    || name_str.ends_with(".todo.json")
                    || name_str.ends_with(".todo")
                    || name_str.ends_with(".json")
                    || name_str.ends_with(".yaml")
                    || name_str.ends_with(".yml")
                {
                    results.push(full);
                }
            }
        }
    }
    results.sort();
    results
}

fn complete_s3_path(partial: &str) -> Vec<String> {
    let rest = partial.strip_prefix("s3://").unwrap_or("");

    if !rest.contains('/') {
        // List buckets, filter by typed prefix
        if let Ok(s3) = duir_core::s3_storage::S3Storage::new()
            && let Ok(buckets) = s3.list_buckets()
        {
            return buckets.into_iter().filter(|b| b.starts_with(partial)).collect();
        }
        return vec![];
    }

    // Have bucket, list objects
    let (bucket, prefix) = rest.split_once('/').unwrap_or((rest, ""));
    if let Ok(s3) = duir_core::s3_storage::S3Storage::new()
        && let Ok(objects) = s3.list_objects(bucket, prefix)
    {
        return objects.into_iter().filter(|o| o.starts_with(partial)).collect();
    }
    vec![]
}
