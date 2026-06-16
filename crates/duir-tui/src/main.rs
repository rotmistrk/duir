//! duir-tui — TXV-based terminal UI for duir todo trees.

use std::fs;
use std::path::PathBuf;

use clap::Parser;
use txv_core::clipboard_ring::new_clipboard;
use txv_core::program::Program;
use txv_render::backend::CrosstermBackend;

mod agent_patch;
mod archive_view;
mod build_desktop;
mod clipboard_view;
mod completer;
mod handler;
mod mcp;
mod mcp_bridge;
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
    /// Run as MCP stdio bridge (connects to `DUIR_MCP_SOCKET`)
    #[arg(long = "mcp-connect")]
    mcp_connect: bool,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    if cli.mcp_connect {
        return mcp_bridge::run().map_err(Into::into);
    }
    let root_dir = fs::canonicalize(&cli.path)?;
    init_logging(&cli.log_file, &cli.log_level)?;
    handler::set_root_dir(root_dir.clone());

    // Initialize palette (dark theme)
    txv_core::palette::set_palette(std::sync::Arc::new(txv_core::palette::dark::DarkPalette));

    let saved = session::load_session(&root_dir);
    let clipboard = new_clipboard(20);

    let mut desktop = build_workspace(&root_dir, clipboard.clone());
    restore_session(&mut desktop, &saved);

    // Scripting: load init.tcl, apply palette, fire startup hooks
    let mut script_engine = scripting::ScriptEngine::new();
    script_engine.load_init(&root_dir);
    scripting::palette_config::apply_palette_from_config(&script_engine);
    script_engine.fire_hooks(&scripting::HookEvent::Startup, "");
    if let Some(cmd) = script_engine.get_var("kiro.cmd") {
        handler::set_kiro_cmd(cmd);
    }
    run_lifecycle(&root_dir, &script_engine);
    drop(script_engine);

    let bar = status::build_status_bar(&desktop, clipboard);
    let mut program = Program::new(Box::new(bar), Box::new(desktop));

    let mut backend = CrosstermBackend::new(txv_render::ColorMode::TrueColor);
    program.run(&mut backend, |ctx| {
        handle_command(ctx);
    });

    save_note_on_exit(&mut program);
    save_session_on_exit(&mut program, &root_dir);
    handler::cleanup_mcp();
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
    if let Some((path, content)) = note_data
        && let Some(panel) = ws.panel_mut(slots::SlotId::Left as usize)
        && let Some(view) = panel.active_view_mut()
        && let Some(tree) = view.as_any_mut().and_then(|a| a.downcast_mut::<TodoTreeView>())
    {
        if let Some(item) = model::get_item_mut(&mut tree.data_mut().file, &path) {
            item.note = content;
        }
        tree.data_mut().save();
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
        focused_panel: ws.focused_panel(),
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

fn run_lifecycle(root_dir: &std::path::Path, engine: &scripting::ScriptEngine) {
    let hours: u64 = engine
        .get_var("lifecycle.archive_after_hours")
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);
    let days: u64 = engine
        .get_var("lifecycle.delete_after_days")
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);
    if hours == 0 && days == 0 {
        return;
    }
    let lc = duir_core::lifecycle::Lifecycle::new(hours, days);
    let duir_dir = root_dir.join(".duir");
    let main_path = duir_dir.join("todo.json");
    let archive_path = duir_dir.join("archive.json");
    let trash_path = duir_dir.join("trash.json");

    let mut main = crate::todo_tree::model::load_todo_file(&main_path);
    let mut archive = crate::todo_tree::model::load_todo_file(&archive_path);
    let mut trash = crate::todo_tree::model::load_todo_file(&trash_path);

    lc.run(&mut main, &mut archive, &mut trash);

    if !crate::todo_tree::model::save_todo_file(&main_path, &main) {
        log::error!("lifecycle: failed to save main");
    }
    if !crate::todo_tree::model::save_todo_file(&archive_path, &archive) {
        log::error!("lifecycle: failed to save archive");
    }
    if !crate::todo_tree::model::save_todo_file(&trash_path, &trash) {
        log::error!("lifecycle: failed to save trash");
    }
}
