//! Bridge: system — `system quit|reload|save`.

use std::sync::{Arc, Mutex};

use rusticle::error::TclError;
use rusticle::interpreter::Interpreter;
use rusticle::value::TclValue;

use super::{ScriptCommand, arg_str, push};

pub fn register(interp: &mut Interpreter, commands: Arc<Mutex<Vec<ScriptCommand>>>) {
    let cmds = commands.clone();
    interp.register_fn("system", move |_interp, args| {
        let sub = arg_str(args, 0)?;
        match sub.as_str() {
            "quit" => push(&cmds, ScriptCommand::Quit),
            "reload" => push(&cmds, ScriptCommand::Reload),
            "save" => push(&cmds, ScriptCommand::Save),
            other => return Err(TclError::new(format!("system: unknown subcommand '{other}'"))),
        }
        Ok(TclValue::Str(String::new()))
    });

    // kiro command: `kiro ?--agent=name? ?--tui? ?args...?`
    // Builds a command string that gets spawned as a PTY in the right panel.
    let cmds2 = commands;
    interp.register_fn("kiro", move |_interp, args| {
        let mut parts = vec!["kiro-cli".to_owned(), "chat".to_owned(), "--restore".to_owned()];
        for arg in args.iter().skip(1) {
            parts.push(arg.as_str().into_owned());
        }
        let cmd = parts.join(" ");
        push(&cmds2, ScriptCommand::Kiro { cmd });
        Ok(TclValue::Str(String::new()))
    });
}
