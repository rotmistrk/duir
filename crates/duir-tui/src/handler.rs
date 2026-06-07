//! Command handler — dispatches application-specific commands only.
//! Panel focus/zoom/navigation is handled by `TiledWorkspace` via `StatusBar` bindings.

use txv_core::prelude::*;
use txv_core::program::CommandContext;
use txv_widgets::tiled_workspace::TiledWorkspace;

use crate::note_view::{CM_NOTE_SAVE, NoteView};
use crate::slots::SlotId;
use crate::todo_tree::TodoTreeView;
use crate::todo_tree::model::{self, TreePath};

/// Application command IDs (above `CM_TXV_MAX`).
const CM_APP_BASE: CommandId = txv_core::commands::CM_TXV_MAX + 1;
pub const CM_APP_QUIT: CommandId = CM_QUIT; // reuse core quit
pub const CM_SHOW_HELP: CommandId = CM_APP_BASE;
pub const CM_EXECUTE_COMMAND: CommandId = CM_APP_BASE + 1;
/// Emitted by tree when cursor moves. Payload: `(TreePath, String)`.
pub const CM_NOTE_LOAD: CommandId = CM_APP_BASE + 10;

/// Top-level command handler.
pub fn handle_command(ctx: &mut CommandContext) {
    match ctx.command {
        CM_SHOW_HELP => show_help(ctx),
        CM_EXECUTE_COMMAND => execute_command(ctx),
        CM_NOTE_LOAD => handle_note_load(ctx),
        CM_NOTE_SAVE => handle_note_save(ctx),
        _ => {}
    }
}

fn show_help(ctx: &mut CommandContext) {
    let Some(ws) = downcast_ws(ctx.desktop) else { return };
    let Some(panel) = ws.panel_mut(SlotId::Center as usize) else {
        return;
    };
    let Some(view) = panel.active_view_mut() else { return };
    if let Some(note) = view.as_any_mut().and_then(|a| a.downcast_mut::<NoteView>()) {
        note.load(vec![], HELP_TEXT);
    }
    ws.focus_panel(SlotId::Center as usize);
}

fn execute_command(ctx: &mut CommandContext) {
    let Some(data) = ctx.data else { return };
    let Some(cmd) = data.downcast_ref::<String>() else {
        return;
    };
    match cmd.trim() {
        "quit" | "q" => ctx.sink.push_command(CM_QUIT, None),
        "help" => show_help(ctx),
        "save" => {
            // Force tree save
            let Some(ws) = downcast_ws(ctx.desktop) else { return };
            if let Some(tree) = get_tree_mut(ws) {
                tree.data_mut().save();
            }
        }
        _ => log::info!("unknown command: {cmd}"),
    }
}

fn handle_note_load(ctx: &mut CommandContext) {
    let Some(data) = ctx.data else { return };
    let Some((path, content)) = data.downcast_ref::<(TreePath, String)>() else {
        return;
    };
    let Some(ws) = downcast_ws(ctx.desktop) else { return };
    let Some(panel) = ws.panel_mut(SlotId::Center as usize) else {
        return;
    };
    let Some(view) = panel.active_view_mut() else { return };
    if let Some(note) = view.as_any_mut().and_then(|a| a.downcast_mut::<NoteView>()) {
        note.load(path.clone(), content);
    }
}

fn handle_note_save(ctx: &mut CommandContext) {
    let Some(data) = ctx.data else { return };
    let Some((path, content)) = data.downcast_ref::<(TreePath, String)>() else {
        return;
    };
    let Some(ws) = downcast_ws(ctx.desktop) else { return };
    if let Some(tree) = get_tree_mut(ws) {
        if let Some(item) = model::get_item_mut(&mut tree.data_mut().file, path) {
            item.note.clone_from(content);
        }
        tree.data_mut().save();
    }
}

fn get_tree_mut(ws: &mut TiledWorkspace) -> Option<&mut TodoTreeView> {
    let panel = ws.panel_mut(SlotId::Left as usize)?;
    let view = panel.active_view_mut()?;
    view.as_any_mut()?.downcast_mut::<TodoTreeView>()
}

fn downcast_ws(desktop: &mut dyn View) -> Option<&mut TiledWorkspace> {
    desktop.as_any_mut()?.downcast_mut::<TiledWorkspace>()
}

const HELP_TEXT: &str = "\
# duir — Keybindings

## Navigation
  j/k, ↑/↓       Move cursor
  Enter/Right     Expand / open note
  Left            Collapse
  F2/F3/F4        Focus Tree/Notes/Tools
  F5              Zoom toggle
  C-S-←/→         Focus prev/next panel
  C-S-↑/↓         Tab dropdown
  Ctrl-Q          Quit

## Tree Operations
  n               New sibling
  b               New child
  d               Delete
  e               Edit title (InputLine)
  Space           Toggle complete
  K/J, S-↑/↓     Move up/down
  H/L, S-←/→     Promote/demote
  S               Sort children

## Priority & Status
  +/-             Priority up/down
  !               Toggle priority 5
  >/<             LOE up/down
  i, =            Toggle in-progress
  \\               Toggle pause
  T               Toggle tree connectors
  D               Toggle timestamps

## Other
  /               Filter mode
  Ctrl-L          Encrypt/decrypt
  M-x, :          Command line
  F1              This help
";
