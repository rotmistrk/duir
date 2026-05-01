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
        if input.is_empty() {
            self.matches.clear();
            self.selected = None;
            return;
        }
        self.matches = self
            .commands
            .iter()
            .filter(|cmd| cmd.starts_with(input))
            .copied()
            .collect();
        // Reset selection if it's out of bounds
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
        Some(self.matches[idx])
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
        Some(self.matches[idx])
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
    "export md ",
    "help",
    "import md ",
    "init",
    "o ",
    "open md ",
    "q",
    "q!",
    "qa",
    "w",
    "wa",
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
