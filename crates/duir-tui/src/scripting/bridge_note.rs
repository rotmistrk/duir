//! Bridge: note operations — `note get|set`.

use std::sync::{Arc, Mutex};

use rusticle::error::TclError;
use rusticle::interpreter::Interpreter;
use rusticle::value::TclValue;

use super::{ScriptCommand, arg_str, push};

pub fn register(interp: &mut Interpreter, commands: Arc<Mutex<Vec<ScriptCommand>>>) {
    let cmds = commands;
    interp.register_fn("note", move |_interp, args| {
        let sub = arg_str(args, 0)?;
        match sub.as_str() {
            "set" => {
                let path = arg_str(args, 1)?;
                let content = arg_str(args, 2)?;
                push(&cmds, ScriptCommand::NoteSet { path, content });
                Ok(TclValue::Str(String::new()))
            }
            other => Err(TclError::new(format!("note: unknown subcommand '{other}'"))),
        }
    });
}
