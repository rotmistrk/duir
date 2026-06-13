//! Hook registry — event-driven Tcl script execution.

use std::sync::{Arc, Mutex};

use rusticle::error::TclError;
use rusticle::interpreter::Interpreter;
use rusticle::value::TclValue;

/// Events that can trigger hooks.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum HookEvent {
    Focus,
    Blur,
    Select,
    StatusChange,
    Key,
    Startup,
}

impl HookEvent {
    #[must_use]
    pub fn parse_name(s: &str) -> Option<Self> {
        match s {
            "focus" => Some(Self::Focus),
            "blur" => Some(Self::Blur),
            "select" => Some(Self::Select),
            "status-change" => Some(Self::StatusChange),
            "key" => Some(Self::Key),
            "startup" => Some(Self::Startup),
            _ => None,
        }
    }

    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Focus => "focus",
            Self::Blur => "blur",
            Self::Select => "select",
            Self::StatusChange => "status-change",
            Self::Key => "key",
            Self::Startup => "startup",
        }
    }
}

/// A registered hook.
struct Hook {
    event: HookEvent,
    filter: Option<String>,
    body: String,
}

/// Registry of hooks, fired in declaration order.
#[derive(Default)]
pub struct HookRegistry {
    hooks: Vec<Hook>,
}

impl HookRegistry {
    #[must_use]
    pub const fn new() -> Self {
        Self { hooks: Vec::new() }
    }

    pub fn add(&mut self, event: HookEvent, filter: Option<&str>, body: String) {
        self.hooks.push(Hook {
            event,
            filter: filter.map(String::from),
            body,
        });
    }

    #[must_use]
    pub fn fire(&self, event: &HookEvent, context: &str) -> Vec<String> {
        self.hooks
            .iter()
            .filter(|h| &h.event == event)
            .filter(|h| h.filter.as_ref().is_none_or(|f| glob_match(f, context)))
            .map(|h| h.body.clone())
            .collect()
    }
}

/// Register the `on` command for defining hooks.
pub fn register_hook_command(interp: &mut Interpreter, registry: Arc<Mutex<HookRegistry>>) {
    interp.register_fn("on", move |_interp, args| {
        let event_name = super::arg_str(args, 0)?;
        let event =
            HookEvent::parse_name(&event_name).ok_or_else(|| TclError::new(format!("unknown event: {event_name}")))?;
        let body = super::arg_str(args, 1)?;
        if let Ok(mut reg) = registry.lock() {
            reg.add(event, None, body);
        }
        Ok(TclValue::Str(String::new()))
    });
}

fn glob_match(pattern: &str, text: &str) -> bool {
    if pattern == "*" {
        return true;
    }
    pattern == text
}
