//! duir-tui — TXV-based terminal UI for duir todo trees.

use std::fs;
use std::path::PathBuf;

use clap::Parser;
use txv_core::prelude::*;
use txv_core::program::Program;
use txv_render::backend::CrosstermBackend;
use txv_widgets::status_bar::StatusBar;
use txv_widgets::tiled_workspace::commands::CM_TW_ZOOM;

mod build_desktop;
mod handler;
mod mcp;
#[allow(dead_code)]
mod mcp_permissions;
mod note_view;
#[allow(dead_code)]
mod scripting;
mod session;
mod shell;
mod slots;
mod todo_tree;

use build_desktop::build_workspace;
use handler::handle_command;

#[derive(Parser)]
#[command(name = "duir", about = "Hierarchical todo tree TUI")]
struct Cli {
    /// Working directory (where .duir/ lives)
    #[arg(default_value = ".")]
    path: PathBuf,

    /// Log file
    #[arg(short = 'l', long = "log", default_value = ".duir.log")]
    log_file: PathBuf,

    /// Log level (error, warn, info, debug, trace)
    #[arg(short = 'L', long = "log-level", default_value = "info")]
    log_level: String,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    let root_dir = fs::canonicalize(&cli.path)?;
    init_logging(&cli.log_file, &cli.log_level)?;

    let mcp_socket = mcp::start_mcp(&root_dir);
    let saved = session::load_session(&root_dir);

    let mut desktop = build_workspace(&root_dir);
    restore_session(&mut desktop, &saved);

    // Init scripting engine and load init.tcl
    let mut script_engine = scripting::ScriptEngine::new();
    script_engine.load_init(&root_dir);
    script_engine.fire_hooks(&scripting::HookEvent::Startup, "");
    drop(script_engine); // Will be kept alive once event loop integration is added

    let status = build_status_bar();
    let mut program = Program::new(Box::new(status), Box::new(desktop));

    let mut backend = CrosstermBackend::new(txv_render::ColorMode::TrueColor);
    program.run(&mut backend, |ctx| {
        handle_command(ctx);
    });

    save_session_on_exit(&mut program, &root_dir);

    if let Some(ref path) = mcp_socket {
        mcp::cleanup_mcp(path);
    }

    Ok(())
}

fn restore_session(ws: &mut txv_widgets::tiled_workspace::TiledWorkspace, saved: &session::SessionState) {
    use crate::todo_tree::TodoTreeView;

    // Restore zoom
    if saved.zoomed_panel.is_some() {
        ws.set_zoomed(saved.zoomed_panel);
    }
    ws.focus_panel(saved.focused_panel);

    // Restore tree cursor and timestamps
    if let Some(panel) = ws.panel_mut(slots::SlotId::Left as usize)
        && let Some(view) = panel.active_view_mut()
        && let Some(tree) = view.as_any_mut().and_then(|a| a.downcast_mut::<TodoTreeView>())
    {
        tree.set_cursor(saved.tree_cursor);
        if saved.show_timestamps {
            tree.toggle_timestamps_on();
        }
    }
}

fn save_session_on_exit(program: &mut Program, root_dir: &std::path::Path) {
    use crate::todo_tree::TodoTreeView;
    use txv_widgets::tiled_workspace::TiledWorkspace;

    let Some(ws) = program
        .desktop_mut()
        .as_any_mut()
        .and_then(|a| a.downcast_mut::<TiledWorkspace>())
    else {
        return;
    };

    let mut state = session::SessionState {
        zoomed_panel: ws.zoomed_panel(),
        focused_panel: 0,
        tree_cursor: 0,
        show_timestamps: false,
    };

    if let Some(panel) = ws.panel_mut(slots::SlotId::Left as usize)
        && let Some(view) = panel.active_view_mut()
        && let Some(tree) = view.as_any_mut().and_then(|a| a.downcast_mut::<TodoTreeView>())
    {
        state.tree_cursor = tree.cursor();
        state.show_timestamps = tree.show_timestamps();
    }

    session::save_session(root_dir, &state);
}

fn build_status_bar() -> StatusBar {
    use handler::{CM_FOCUS_NOTES, CM_FOCUS_TOOLS, CM_FOCUS_TREE};

    let mut bar = StatusBar::new();

    bar.add_item(key(KeyCode::F(2)), CM_FOCUS_TREE, "F2:Tree");
    bar.add_item(key(KeyCode::F(3)), CM_FOCUS_NOTES, "F3:Notes");
    bar.add_item(key(KeyCode::F(4)), CM_FOCUS_TOOLS, "F4:Tools");
    bar.add_item(key(KeyCode::F(5)), CM_TW_ZOOM, "F5:Zoom");
    bar.add_item(
        KeyEvent {
            code: KeyCode::Char('q'),
            modifiers: KeyMod {
                ctrl: true,
                shift: false,
                alt: false,
            },
        },
        CM_QUIT,
        "^Q:Quit",
    );

    bar
}

const fn key(code: KeyCode) -> KeyEvent {
    KeyEvent {
        code,
        modifiers: KeyMod {
            ctrl: false,
            shift: false,
            alt: false,
        },
    }
}

fn init_logging(log_file: &std::path::Path, level: &str) -> Result<(), Box<dyn std::error::Error>> {
    let target = env_logger::Target::Pipe(Box::new(
        fs::OpenOptions::new().create(true).append(true).open(log_file)?,
    ));
    env_logger::Builder::new().target(target).parse_filters(level).init();
    Ok(())
}
