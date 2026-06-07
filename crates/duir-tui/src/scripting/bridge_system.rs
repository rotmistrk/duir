//! Bridge: system — `system quit|reload|save`.

use std::sync::{Arc, Mutex};

use rusticle::error::TclError;
use rusticle::interpreter::Interpreter;
use rusticle::value::TclValue;

use super::{ScriptCommand, arg_str, push};

pub fn register(interp: &mut Interpreter, commands: Arc<Mutex<Vec<ScriptCommand>>>) {
    let cmds = commands;
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
}
