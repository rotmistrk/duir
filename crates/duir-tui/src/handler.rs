//! Command handler — dispatches application-specific commands only.

use txv_core::prelude::*;
use txv_core::program::CommandContext;
use txv_widgets::tiled_workspace::TiledWorkspace;

use crate::note_view::{CM_NOTE_SAVE, NoteView};
use crate::slots::SlotId;
use crate::todo_tree::TodoTreeView;
use crate::todo_tree::model::{self, TreePath};

const CM_APP_BASE: CommandId = txv_core::commands::CM_TXV_MAX + 1;
pub const CM_APP_QUIT: CommandId = CM_QUIT;
pub const CM_SHOW_HELP: CommandId = CM_APP_BASE;
pub const CM_EXECUTE_COMMAND: CommandId = CM_APP_BASE + 1;
pub const CM_NOTE_LOAD: CommandId = CM_APP_BASE + 10;

pub fn handle_command(ctx: &mut CommandContext) {
    match ctx.command() {
        CM_SHOW_HELP => show_help(ctx),
        CM_EXECUTE_COMMAND => execute_command(ctx),
        CM_NOTE_LOAD => handle_note_load(ctx),
        CM_NOTE_SAVE => handle_note_save(ctx),
        _ => {}
    }
}

fn show_help(ctx: &mut CommandContext) {
    let (_, _, _, desktop) = ctx.split();
    let Some(ws) = desktop.as_any_mut().and_then(|a| a.downcast_mut::<TiledWorkspace>()) else {
        return;
    };
    if let Some(panel) = ws.panel_mut(SlotId::Center as usize)
        && let Some(view) = panel.active_view_mut()
        && let Some(note) = view.as_any_mut().and_then(|a| a.downcast_mut::<NoteView>())
    {
        note.load(vec![], HELP_TEXT);
    }
    ws.focus_panel(SlotId::Center as usize);
}

fn execute_command(ctx: &mut CommandContext) {
    let (_, data, sink, desktop) = ctx.split();
    let Some(data) = data else { return };
    let Some(cmd) = data.downcast_ref::<String>() else {
        return;
    };
    match cmd.trim() {
        "quit" | "q" => sink.push_command(CM_QUIT, None),
        "save" => {
            let Some(ws) = desktop.as_any_mut().and_then(|a| a.downcast_mut::<TiledWorkspace>()) else {
                return;
            };
            if let Some(tree) = get_tree_mut(ws) {
                tree.data_mut().save();
            }
        }
        _ => log::info!("unknown command: {cmd}"),
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
    if let Some(panel) = ws.panel_mut(SlotId::Center as usize)
        && let Some(view) = panel.active_view_mut()
        && let Some(note) = view.as_any_mut().and_then(|a| a.downcast_mut::<NoteView>())
    {
        note.load(path.clone(), content);
    }
}

fn handle_note_save(ctx: &mut CommandContext) {
    let (_, data, _, desktop) = ctx.split();
    let Some(data) = data else { return };
    let Some((path, content)) = data.downcast_ref::<(TreePath, String)>() else {
        return;
    };
    let Some(ws) = desktop.as_any_mut().and_then(|a| a.downcast_mut::<TiledWorkspace>()) else {
        return;
    };
    if let Some(tree) = get_tree_mut(ws) {
        if let Some(item) = model::get_item_mut(&mut tree.data_mut().file, path) {
            item.note.clone_from(content);
        }
        tree.data_mut().save();
    }
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
  Ctrl-L          Encrypt/decrypt
  M-x, :          Command line
  F1              This help
";
