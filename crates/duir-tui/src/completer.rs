//! Command completer for the M-x command line.

use txv_core::complete::{Completer, Completion, CompletionVisitor};

const COMMANDS: &[&str] = &["close", "help", "kiro", "layout", "quit", "q", "save", "shell", "w"];

struct SimpleCompletion(&'static str);

impl Completion for SimpleCompletion {
    fn text(&self) -> &'static str {
        self.0
    }
    fn display(&self) -> &'static str {
        self.0
    }
    fn kind(&self) -> &'static str {
        "command"
    }
}

pub struct CommandCompleter;

impl Completer for CommandCompleter {
    fn complete(
        &self,
        input: &str,
        _cursor: usize,
        visitor: &mut CompletionVisitor<'_>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        for &cmd in COMMANDS {
            if cmd.starts_with(input) && !visitor(&SimpleCompletion(cmd))? {
                break;
            }
        }
        Ok(())
    }
}
