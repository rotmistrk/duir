//! Command handler — dispatches application-specific commands only.
use std::sync::OnceLock;

use txv_core::prelude::*;
use txv_core::program::CommandContext;
use txv_widgets::tiled_workspace::TiledWorkspace;

use crate::note_view::NoteView;
use crate::slots::SlotId;
use crate::todo_tree::TodoTreeView;
use crate::todo_tree::model::{self, TreePath};

static KIRO_CMD: OnceLock<String> = OnceLock::new();
static ROOT_DIR: OnceLock<std::path::PathBuf> = OnceLock::new();
static MCP_STARTED: OnceLock<Option<std::path::PathBuf>> = OnceLock::new();

/// Set the default kiro command (called during startup from Tcl config).
pub fn set_kiro_cmd(cmd: String) {
    let _ = KIRO_CMD.set(cmd);
}

/// Set the project root directory.
pub fn set_root_dir(dir: std::path::PathBuf) {
    let _ = ROOT_DIR.set(dir);
}

fn default_kiro_cmd() -> &'static str {
    KIRO_CMD.get().map_or("kiro-cli chat --resume", String::as_str)
}

fn root_dir() -> &'static std::path::Path {
    ROOT_DIR
        .get()
        .map_or_else(|| std::path::Path::new("."), |p| p.as_path())
}

/// Ensure MCP server is running (starts on first call).
fn ensure_mcp() {
    MCP_STARTED.get_or_init(|| crate::mcp::start_mcp(root_dir()));
}
/// Clean up MCP socket on exit.
pub fn cleanup_mcp() {
    if let Some(Some(p)) = MCP_STARTED.get() {
        crate::mcp::cleanup_mcp(p);
    }
}

const CM_APP_BASE: CommandId = txv_core::commands::CM_TXV_MAX + 1;
pub const CM_APP_QUIT: CommandId = CM_QUIT;
pub const CM_SHOW_HELP: CommandId = CM_APP_BASE;
pub const CM_EXECUTE_COMMAND: CommandId = CM_APP_BASE + 1;
pub const CM_NOTE_LOAD: CommandId = CM_APP_BASE + 10;
pub const CM_NOTE_SAVE: CommandId = CM_APP_BASE + 11;
pub const CM_RESTORE_ITEM: CommandId = CM_APP_BASE + 12;

pub fn handle_command(ctx: &mut CommandContext) {
    match ctx.command() {
        CM_SHOW_HELP => show_help(ctx.desktop_mut()),
        CM_EXECUTE_COMMAND => execute_command(ctx),
        CM_NOTE_LOAD => handle_note_load(ctx),
        CM_NOTE_SAVE => handle_note_save(ctx),
        CM_RESTORE_ITEM => handle_restore(ctx),
        _ => {}
    }
}

fn show_help(desktop: &mut dyn View) {
    let Some(ws) = desktop.as_any_mut().and_then(|a| a.downcast_mut::<TiledWorkspace>()) else {
        return;
    };
    let mut ta = txv_widgets::TextArea::new();
    ta.set_content(HELP_TEXT);
    ws.insert_tab(SlotId::Center as usize, "Help", Box::new(ta));
    ws.focus_panel(SlotId::Center as usize);
}

fn execute_command(ctx: &mut CommandContext) {
    let (_, data, sink, desktop) = ctx.split();
    let Some(data) = data else { return };
    let Some(cmd) = data.downcast_ref::<String>() else {
        return;
    };
    let args: Vec<&str> = cmd.trim().splitn(2, char::is_whitespace).collect();
    let cmd_name = args.first().copied().unwrap_or("");
    let arg = args.get(1).copied().unwrap_or("").trim();
    match cmd_name {
        "quit" | "q" => sink.push_command(CM_QUIT, None),
        "help" => show_help(desktop),
        "save" | "w" => {
            let Some(ws) = desktop.as_any_mut().and_then(|a| a.downcast_mut::<TiledWorkspace>()) else {
                return;
            };
            if let Some(tree) = get_tree_mut(ws) {
                tree.data_mut().save();
            }
        }
        "kiro" => {
            let Some(ws) = desktop.as_any_mut().and_then(|a| a.downcast_mut::<TiledWorkspace>()) else {
                return;
            };
            let kiro_cmd = build_kiro_cmd(arg);
            let term = crate::shell::new_kiro_terminal(&kiro_cmd, root_dir());
            ws.insert_tab(SlotId::Right as usize, "Kiro:0", term);
            ws.focus_panel(SlotId::Right as usize);
        }
        "close" => {
            let Some(ws) = desktop.as_any_mut().and_then(|a| a.downcast_mut::<TiledWorkspace>()) else {
                return;
            };
            let focused = ws.focused_panel();
            if let Some(panel) = ws.panel_mut(focused) {
                panel.close_active();
            }
        }
        "layout" => sink.push_command(txv_widgets::tiled_workspace::commands::CM_TW_LAYOUT_CYCLE, None),
        "shell" => {
            let Some(ws) = desktop.as_any_mut().and_then(|a| a.downcast_mut::<TiledWorkspace>()) else {
                return;
            };
            let term = crate::shell::new_shell_terminal();
            ws.insert_tab(SlotId::Right as usize, "Shell:0", term);
            ws.focus_panel(SlotId::Right as usize);
        }
        _ => log::info!("unknown command: {cmd_name:?} (raw: {cmd:?})"),
    }
}

fn handle_note_load(ctx: &mut CommandContext) {
    let (_, data, _, desktop) = ctx.split();
    let Some(data) = data else { return };
    let Some((path, content)) = data.downcast_ref::<(TreePath, String)>() else {
        return;
    };
    let Some(ws) = desktop.as_any_mut().and_then(|a| a.downcast_mut::<TiledWorkspace>()) else {
        return;
    };

    // Save previous note to tree before loading new
    save_current_note(ws);

    // Load new note
    if let Some(panel) = ws.panel_mut(SlotId::Center as usize)
        && let Some(view) = panel.active_view_mut()
        && let Some(note) = view.as_any_mut().and_then(|a| a.downcast_mut::<NoteView>())
    {
        note.load(path.clone(), content);
    }
}

fn handle_note_save(ctx: &mut CommandContext) {
    let (_, _, _, desktop) = ctx.split();
    let Some(ws) = desktop.as_any_mut().and_then(|a| a.downcast_mut::<TiledWorkspace>()) else {
        return;
    };
    save_current_note(ws);
}

fn handle_restore(ctx: &mut CommandContext) {
    let (_, data, _, desktop) = ctx.split();
    let Some(item) = data
        .as_ref()
        .and_then(|d| d.downcast_ref::<duir_core::TodoItem>())
        .cloned()
    else {
        return;
    };
    let Some(ws) = desktop.as_any_mut().and_then(|a| a.downcast_mut::<TiledWorkspace>()) else {
        return;
    };
    if let Some(tree) = get_tree_mut(ws) {
        tree.data_mut().file.items.push(item);
        tree.data_mut().save();
        tree.data_mut().rebuild_flat();
    }
}

fn save_current_note(ws: &mut TiledWorkspace) {
    // Get note content + path
    let note_data: Option<(TreePath, String)> = ws
        .panel_mut(SlotId::Center as usize)
        .and_then(|p| p.active_view_mut())
        .and_then(|v| v.as_any_mut()?.downcast_mut::<NoteView>())
        .and_then(|nv| Some((nv.path()?.clone(), nv.content())));

    // Write to tree
    if let Some((path, content)) = note_data
        && let Some(tree) = get_tree_mut(ws)
    {
        if let Some(item) = model::get_item_mut(&mut tree.data_mut().file, &path) {
            item.note = content;
        }
        tree.data_mut().save();
        tree.data_mut().rebuild_flat();
    }
}

fn build_kiro_cmd(arg: &str) -> String {
    ensure_mcp();
    let socket = root_dir().join(".duir").join("mcp.sock");
    crate::agent_patch::build_kiro_cmd(default_kiro_cmd(), arg, root_dir(), &socket)
}

fn get_tree_mut(ws: &mut TiledWorkspace) -> Option<&mut TodoTreeView> {
    ws.panel_mut(SlotId::Left as usize)?
        .active_view_mut()?
        .as_any_mut()?
        .downcast_mut::<TodoTreeView>()
}

const HELP_TEXT: &str = r"# duir - Keybindings

## Navigation
  j/k             Move cursor
  Enter/Right     Expand / open note
  Left            Collapse
  F2/F3/F4        Focus Tree/Notes/Tools
  F5, Alt-/       Zoom toggle
  C-S-Arrows      Focus panels
  Alt-Up/Down     Tab dropdown
  Alt-1..9        Select tab
  Ctrl-Q          Quit

## Tree Operations
  n               New sibling
  b               New child
  d               Delete
  e               Edit title
  Space           Toggle complete
  K/J, S-Up/Down  Move up/down
  H/L, S-Left/Rt  Promote/demote
  S               Sort children

## Priority & Status
  +/-             Priority up/down
  !               Toggle priority 5
  >/<             LOE up/down
  i, =            Toggle in-progress
  \               Toggle pause
  T               Toggle tree connectors
  D               Toggle timestamps

## Other
  /               Filter mode
  Ctrl-L          Redraw screen
  Alt-L           Encrypt/decrypt
  M-x, :          Command line
  F1              This help
";
