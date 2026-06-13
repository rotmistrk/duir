//! duir-tui — TXV-based terminal UI for duir todo trees.

use std::fs;
use std::path::PathBuf;

use clap::Parser;
use txv_core::clipboard_ring::new_clipboard;
use txv_core::program::Program;
use txv_render::backend::CrosstermBackend;

mod build_desktop;
mod clipboard_view;
mod completer;
mod handler;
mod mcp;
#[allow(dead_code)]
mod mcp_permissions;
mod messages;
mod note_view;
#[allow(dead_code)]
mod scripting;
mod session;
mod shell;
mod slots;
mod status;
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
    /// Log level
    #[arg(short = 'L', long = "log-level", default_value = "info")]
    log_level: String,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    let root_dir = fs::canonicalize(&cli.path)?;
    init_logging(&cli.log_file, &cli.log_level)?;

    // Initialize palette (dark theme)
    txv_core::palette::set_palette(std::sync::Arc::new(txv_core::palette::dark::DarkPalette));

    let mcp_socket = mcp::start_mcp(&root_dir);
    let saved = session::load_session(&root_dir);
    let clipboard = new_clipboard(20);

    let mut desktop = build_workspace(&root_dir, clipboard.clone());
    restore_session(&mut desktop, &saved);

    // Scripting: load init.tcl, fire startup hooks
    let mut script_engine = scripting::ScriptEngine::new();
    script_engine.load_init(&root_dir);
    script_engine.fire_hooks(&scripting::HookEvent::Startup, "");
    drop(script_engine);

    let bar = status::build_status_bar(&desktop, clipboard);
    let mut program = Program::new(Box::new(bar), Box::new(desktop));

    let mut backend = CrosstermBackend::new(txv_render::ColorMode::TrueColor);
    program.run(&mut backend, |ctx| {
        handle_command(ctx);
    });

    save_note_on_exit(&mut program);
    save_session_on_exit(&mut program, &root_dir);
    if let Some(ref path) = mcp_socket {
        mcp::cleanup_mcp(path);
    }
    Ok(())
}

fn restore_session(ws: &mut txv_widgets::tiled_workspace::TiledWorkspace, saved: &session::SessionState) {
    use crate::todo_tree::TodoTreeView;
    ws.set_zoomed(saved.zoomed_panel);
    ws.focus_panel(saved.focused_panel);
    if let Some(panel) = ws.panel_mut(slots::SlotId::Left as usize)
        && let Some(view) = panel.active_view_mut()
        && let Some(tree) = view.as_any_mut().and_then(|a| a.downcast_mut::<TodoTreeView>())
    {
        tree.set_cursor(saved.tree_cursor);
        if saved.show_timestamps {
            tree.toggle_timestamps_on();
        }
        tree.set_show_connectors(saved.show_connectors);
    }
}

fn save_note_on_exit(program: &mut Program) {
    use crate::note_view::NoteView;
    use crate::todo_tree::TodoTreeView;
    use crate::todo_tree::model;
    use txv_widgets::tiled_workspace::TiledWorkspace;

    let Some(ws) = program
        .desktop_mut()
        .as_any_mut()
        .and_then(|a| a.downcast_mut::<TiledWorkspace>())
    else {
        return;
    };

    // Get note content + path from NoteView
    let note_data: Option<(Vec<usize>, String)> = ws
        .panel_mut(slots::SlotId::Center as usize)
        .and_then(|p| p.active_view_mut())
        .and_then(|v| v.as_any_mut()?.downcast_mut::<NoteView>())
        .and_then(|nv| {
            let content = nv.content();
            let path = nv.path()?.clone();
            Some((path, content))
        });

    // Apply to tree
    if let Some((path, content)) = note_data {
        if let Some(panel) = ws.panel_mut(slots::SlotId::Left as usize)
            && let Some(view) = panel.active_view_mut()
            && let Some(tree) = view.as_any_mut().and_then(|a| a.downcast_mut::<TodoTreeView>())
        {
            if let Some(item) = model::get_item_mut(&mut tree.data_mut().file, &path) {
                item.note = content;
            }
            tree.data_mut().save();
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
        show_connectors: true,
    };
    if let Some(panel) = ws.panel_mut(slots::SlotId::Left as usize)
        && let Some(view) = panel.active_view_mut()
        && let Some(tree) = view.as_any_mut().and_then(|a| a.downcast_mut::<TodoTreeView>())
    {
        state.tree_cursor = tree.cursor();
        state.show_timestamps = tree.show_timestamps();
        state.show_connectors = tree.show_connectors();
    }
    session::save_session(root_dir, &state);
}

fn init_logging(log_file: &std::path::Path, level: &str) -> Result<(), Box<dyn std::error::Error>> {
    let target = env_logger::Target::Pipe(Box::new(
        fs::OpenOptions::new().create(true).append(true).open(log_file)?,
    ));
    env_logger::Builder::new().target(target).parse_filters(level).init();
    Ok(())
}
