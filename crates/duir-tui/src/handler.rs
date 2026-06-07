//! Command handler — dispatches workspace-level commands.

use txv_core::prelude::*;
use txv_core::program::CommandContext;
use txv_widgets::tiled_workspace::TiledWorkspace;

use crate::note_view::{CM_NOTE_SAVE, NoteView};
use crate::slots::SlotId;
use crate::todo_tree::TodoTreeView;
use crate::todo_tree::model::{self, TreePath};

/// Application command IDs (above `CM_TXV_MAX`).
const CM_APP_BASE: CommandId = txv_core::commands::CM_TXV_MAX + 1;
pub const CM_FOCUS_TREE: CommandId = CM_APP_BASE;
pub const CM_FOCUS_NOTES: CommandId = CM_APP_BASE + 1;
pub const CM_FOCUS_TOOLS: CommandId = CM_APP_BASE + 2;
pub const CM_HELP: CommandId = CM_APP_BASE + 3;
/// Emitted by tree when cursor moves. Payload: `(TreePath, String)`.
pub const CM_NOTE_LOAD: CommandId = CM_APP_BASE + 10;

/// Top-level command handler for the duir event loop.
pub fn handle_command(ctx: &mut CommandContext) {
    match ctx.command {
        CM_FOCUS_TREE => focus_panel(ctx, SlotId::Left),
        CM_FOCUS_NOTES => focus_panel(ctx, SlotId::Center),
        CM_FOCUS_TOOLS => focus_panel(ctx, SlotId::Right),
        CM_HELP => show_help(ctx),
        CM_NOTE_LOAD => handle_note_load(ctx),
        CM_NOTE_SAVE => handle_note_save(ctx),
        _ => {}
    }
}

fn focus_panel(ctx: &mut CommandContext, slot: SlotId) {
    if let Some(ws) = downcast_ws(ctx.desktop) {
        ws.focus_panel(slot as usize);
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
    let Some(panel) = ws.panel_mut(SlotId::Left as usize) else {
        return;
    };
    let Some(view) = panel.active_view_mut() else { return };
    if let Some(tree) = view.as_any_mut().and_then(|a| a.downcast_mut::<TodoTreeView>()) {
        if let Some(item) = model::get_item_mut(&mut tree.data_mut().file, path) {
            item.note.clone_from(content);
        }
        tree.data_mut().save();
    }
}

fn downcast_ws(desktop: &mut dyn View) -> Option<&mut TiledWorkspace> {
    desktop.as_any_mut()?.downcast_mut::<TiledWorkspace>()
}

const HELP_TEXT: &str = "\
# duir — Keybindings

## Navigation
  j/k, ↑/↓     Move cursor
  Enter/Right   Expand / open note
  Left          Collapse
  F2            Focus tree
  F3            Focus notes
  F4            Focus shell
  F5            Zoom toggle
  C-S-←/→      Focus prev/next panel
  Ctrl-Q        Quit

## Tree Operations
  n             New sibling
  b             New child
  d             Delete
  e             Edit title
  Space         Toggle complete
  K/J, S-↑/↓   Move up/down
  H/L, S-←/→   Promote/demote
  S             Sort children

## Priority & Status
  +/-           Priority up/down
  !             Toggle priority 5
  >/<           LOE up/down
  i, =          Toggle in-progress
  \\             Toggle pause
  D             Toggle timestamps

## Other
  /             Filter mode
  Ctrl-L        Encrypt/decrypt
  F1            This help
";

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
