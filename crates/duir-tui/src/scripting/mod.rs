//! Scripting engine — rusticle Tcl interpreter with duir bridge commands.

mod bridge_nav;
mod bridge_note;
mod bridge_system;
mod bridge_tree;
mod hooks;

pub use hooks::{HookEvent, HookRegistry};

use std::fs;
use std::path::Path;
use std::sync::{Arc, Mutex};

use rusticle::interpreter::Interpreter;
use rusticle::value::TclValue;

/// Commands queued by Tcl scripts for the main event loop to apply.
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub enum ScriptCommand {
    // Tree ops
    TreeAdd { title: String, parent: Option<String> },
    TreeRemove { path: String },
    TreeToggle { path: String },
    TreeMove { path: String, direction: String },
    TreePromote { path: String },
    TreeDemote { path: String },
    TreeSetPriority { path: String, value: u8 },
    TreeSetEffort { path: String, value: u8 },
    TreeSetStatus { path: String, status: String },
    // Note ops
    NoteSet { path: String, content: String },
    // Navigation
    NavCursor { position: usize },
    NavFocusPanel { panel: usize },
    NavZoom,
    // System
    Quit,
    Reload,
    Save,
}

/// The scripting engine.
pub struct ScriptEngine {
    interp: Interpreter,
    pub commands: Arc<Mutex<Vec<ScriptCommand>>>,
    pub hook_registry: Arc<Mutex<HookRegistry>>,
}

impl ScriptEngine {
    pub fn new() -> Self {
        let commands: Arc<Mutex<Vec<ScriptCommand>>> = Arc::new(Mutex::new(Vec::new()));
        let hook_registry = Arc::new(Mutex::new(HookRegistry::new()));

        let mut interp = Interpreter::new();
        bridge_tree::register(&mut interp, commands.clone());
        bridge_note::register(&mut interp, commands.clone());
        bridge_nav::register(&mut interp, commands.clone());
        bridge_system::register(&mut interp, commands.clone());
        hooks::register_hook_command(&mut interp, hook_registry.clone());

        Self {
            interp,
            commands,
            hook_registry,
        }
    }

    /// Evaluate a Tcl script.
    pub fn eval(&mut self, script: &str) -> Result<String, String> {
        self.interp
            .eval(script)
            .map(|v| v.as_str().into_owned())
            .map_err(|e| e.message)
    }

    /// Load and eval init.tcl if it exists.
    pub fn load_init(&mut self, root_dir: &Path) {
        let init_path = root_dir.join(".duir").join("init.tcl");
        if let Ok(content) = fs::read_to_string(&init_path)
            && let Err(e) = self.eval(&content)
        {
            log::warn!("init.tcl error: {e}");
        }
    }

    /// Fire hooks for an event and collect scripts to run.
    pub fn fire_hooks(&mut self, event: &HookEvent, context: &str) {
        let scripts = if let Ok(registry) = self.hook_registry.lock() {
            registry.fire(event, context)
        } else {
            return;
        };
        for script in scripts {
            if let Err(e) = self.eval(&script) {
                log::warn!("hook {}: {e}", event.as_str());
            }
        }
    }

    /// Drain queued commands.
    pub fn drain_commands(&self) -> Vec<ScriptCommand> {
        self.commands
            .lock()
            .map_or_else(|_| Vec::new(), |mut cmds| std::mem::take(&mut *cmds))
    }
}

fn arg_str(args: &[TclValue], idx: usize) -> Result<String, rusticle::error::TclError> {
    args.get(idx + 1)
        .map(|v| v.as_str().into_owned())
        .ok_or_else(|| rusticle::error::TclError::new(format!("missing argument {idx}")))
}

fn push(cmds: &Arc<Mutex<Vec<ScriptCommand>>>, cmd: ScriptCommand) {
    if let Ok(mut v) = cmds.lock() {
        v.push(cmd);
    }
}
