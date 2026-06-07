//! Bridge: tree operations — `tree add|remove|toggle|move|promote|demote|priority|effort|status`.

use std::sync::{Arc, Mutex};

use rusticle::error::TclError;
use rusticle::interpreter::Interpreter;
use rusticle::value::TclValue;

use super::{ScriptCommand, arg_str, push};

pub fn register(interp: &mut Interpreter, commands: Arc<Mutex<Vec<ScriptCommand>>>) {
    let cmds = commands;
    interp.register_fn("tree", move |_interp, args| {
        let sub = arg_str(args, 0)?;
        handle(&cmds, args, &sub)
    });
}

fn handle(cmds: &Arc<Mutex<Vec<ScriptCommand>>>, args: &[TclValue], sub: &str) -> Result<TclValue, TclError> {
    match sub {
        "add" => {
            let title = arg_str(args, 1)?;
            let parent = args.get(3).map(|v| v.as_str().into_owned());
            push(cmds, ScriptCommand::TreeAdd { title, parent });
        }
        "remove" => {
            let path = arg_str(args, 1)?;
            push(cmds, ScriptCommand::TreeRemove { path });
        }
        "toggle" => {
            let path = arg_str(args, 1)?;
            push(cmds, ScriptCommand::TreeToggle { path });
        }
        "move" => {
            let path = arg_str(args, 1)?;
            let direction = arg_str(args, 2)?;
            push(cmds, ScriptCommand::TreeMove { path, direction });
        }
        "promote" => {
            let path = arg_str(args, 1)?;
            push(cmds, ScriptCommand::TreePromote { path });
        }
        "demote" => {
            let path = arg_str(args, 1)?;
            push(cmds, ScriptCommand::TreeDemote { path });
        }
        "priority" => {
            let path = arg_str(args, 1)?;
            let val = arg_str(args, 2)?
                .parse::<u8>()
                .map_err(|e| TclError::new(format!("invalid priority: {e}")))?;
            push(cmds, ScriptCommand::TreeSetPriority { path, value: val });
        }
        "effort" => {
            let path = arg_str(args, 1)?;
            let val = arg_str(args, 2)?
                .parse::<u8>()
                .map_err(|e| TclError::new(format!("invalid effort: {e}")))?;
            push(cmds, ScriptCommand::TreeSetEffort { path, value: val });
        }
        "status" => {
            let path = arg_str(args, 1)?;
            let status = arg_str(args, 2)?;
            push(cmds, ScriptCommand::TreeSetStatus { path, status });
        }
        other => return Err(TclError::new(format!("tree: unknown subcommand '{other}'"))),
    }
    Ok(TclValue::Str(String::new()))
}
