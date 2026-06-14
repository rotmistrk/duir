//! Command completer for the M-x command line.

use txv_core::complete::{Completer, Completion, CompletionVisitor};

const COMMANDS: &[&str] = &["close", "help", "kiro", "layout", "quit", "q", "save", "shell", "w"];
const KIRO_ARGS: &[&str] = &["--agent=", "--resume", "--tui"];

struct SimpleCompletion {
    text: String,
}

impl Completion for SimpleCompletion {
    fn text(&self) -> &str {
        &self.text
    }
    fn display(&self) -> &str {
        &self.text
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
        if let Some(rest) = input.strip_prefix("kiro ") {
            // Complete kiro subargs
            for &arg in KIRO_ARGS {
                if arg.starts_with(rest) {
                    let full = format!("kiro {arg}");
                    if !visitor(&SimpleCompletion { text: full })? {
                        break;
                    }
                }
            }
        } else {
            for &cmd in COMMANDS {
                if cmd.starts_with(input) && !visitor(&SimpleCompletion { text: cmd.to_owned() })? {
                    break;
                }
            }
        }
        Ok(())
    }
}
