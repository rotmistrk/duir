//! Mini-commandline — `:` or `M-x` opens a one-line command input.

use txv_core::prelude::*;

/// Command IDs for the command line.
pub const CM_COMMAND_MODE: CommandId = txv_core::commands::CM_TXV_MAX + 20;
pub const CM_COMMAND_EXEC: CommandId = txv_core::commands::CM_TXV_MAX + 21;

/// Command-line state (owned by handler, activated by key).
pub struct CommandLine {
    pub active: bool,
    pub text: String,
}

impl CommandLine {
    pub const fn new() -> Self {
        Self {
            active: false,
            text: String::new(),
        }
    }

    pub fn activate(&mut self) {
        self.active = true;
        self.text.clear();
    }

    pub fn handle_key(&mut self, key: &KeyEvent) -> CommandLineResult {
        match key.code {
            KeyCode::Esc => {
                self.active = false;
                CommandLineResult::Cancel
            }
            KeyCode::Enter => {
                self.active = false;
                let cmd = self.text.clone();
                self.text.clear();
                CommandLineResult::Execute(cmd)
            }
            KeyCode::Backspace => {
                self.text.pop();
                CommandLineResult::Redraw
            }
            KeyCode::Char(c) => {
                self.text.push(c);
                CommandLineResult::Redraw
            }
            _ => CommandLineResult::Redraw,
        }
    }
}

pub enum CommandLineResult {
    Cancel,
    Execute(String),
    Redraw,
}
