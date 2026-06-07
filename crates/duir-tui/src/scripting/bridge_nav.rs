//! Bridge: navigation — `nav cursor|focus|zoom`.

use std::sync::{Arc, Mutex};

use rusticle::error::TclError;
use rusticle::interpreter::Interpreter;
use rusticle::value::TclValue;

use super::{ScriptCommand, arg_str, push};

pub fn register(interp: &mut Interpreter, commands: Arc<Mutex<Vec<ScriptCommand>>>) {
    let cmds = commands;
    interp.register_fn("nav", move |_interp, args| {
        let sub = arg_str(args, 0)?;
        match sub.as_str() {
            "cursor" => {
                let pos = arg_str(args, 1)?
                    .parse::<usize>()
                    .map_err(|e| TclError::new(format!("invalid cursor: {e}")))?;
                push(&cmds, ScriptCommand::NavCursor { position: pos });
            }
            "focus" => {
                let panel = arg_str(args, 1)?
                    .parse::<usize>()
                    .map_err(|e| TclError::new(format!("invalid panel: {e}")))?;
                push(&cmds, ScriptCommand::NavFocusPanel { panel });
            }
            "zoom" => {
                push(&cmds, ScriptCommand::NavZoom);
            }
            other => return Err(TclError::new(format!("nav: unknown subcommand '{other}'"))),
        }
        Ok(TclValue::Str(String::new()))
    });
}
