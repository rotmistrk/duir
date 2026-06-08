//! Workspace builder — constructs the initial duir 3-slot layout.

use std::path::Path;

use txv_core::clipboard_ring::ClipboardHandle;
use txv_core::prelude::*;
use txv_widgets::tiled_workspace::TiledWorkspace;
use txv_widgets::tiled_workspace::types::{PanelConfig, PanelPosition, SplitNode};

use crate::clipboard_view::ClipboardView;
use crate::messages::MessagesView;
use crate::note_view::NoteView;
use crate::shell::new_shell_terminal;
use crate::slots::{PANEL_COUNT, SlotId};
use crate::todo_tree::TodoTreeView;

/// Build duir's 3-slot workspace. Tree zoomed on start.
pub fn build_workspace(root_dir: &Path, clipboard: ClipboardHandle) -> TiledWorkspace {
    let configs = vec![
        PanelConfig::fixed("Tree", PanelPosition::Left),
        PanelConfig::new("Editor", PanelPosition::Center),
        PanelConfig::new("Tools", PanelPosition::Right).with_splittable(),
    ];

    let wide_layout = SplitNode::h(vec![
        (0.25, SplitNode::leaf(0)),
        (0.40, SplitNode::leaf(1)),
        (0.35, SplitNode::leaf(2)),
    ]);

    let narrow_layout = SplitNode::v(vec![
        (
            0.7,
            SplitNode::h(vec![(0.3, SplitNode::leaf(0)), (0.7, SplitNode::leaf(1))]),
        ),
        (0.3, SplitNode::leaf(2)),
    ]);

    let mut ws = TiledWorkspace::new(configs, wide_layout, narrow_layout, 300);
    ws.set_handle_keys(false);
    ws.set_v_divider_gaps(false);
    configure_keymap(&mut ws);

    for i in 0..PANEL_COUNT {
        if let Some(panel) = ws.panel_mut(i) {
            panel.bar_mut().set_handle_keys(false);
        }
    }

    // Left: todo tree
    let mut tree = TodoTreeView::new(root_dir);
    tree.clipboard = clipboard.clone();
    ws.insert_tab(SlotId::Left as usize, "Todo", Box::new(tree));

    // Center: note editor
    ws.insert_tab(SlotId::Center as usize, "Note", Box::new(NoteView::new()));

    // Right: shell, messages, clipboard viewer
    ws.insert_tab(SlotId::Right as usize, "Shell:0", new_shell_terminal());
    ws.insert_tab(SlotId::Right as usize, "Messages", Box::new(MessagesView::new()));
    ws.insert_tab(
        SlotId::Right as usize,
        "Clipboard",
        Box::new(ClipboardView::new(clipboard)),
    );

    ws.focus_panel(SlotId::Left as usize);
    ws.set_zoomed(Some(SlotId::Left as usize));
    ws
}

fn configure_keymap(ws: &mut TiledWorkspace) {
    let mut km = ws.keymap().clone();
    km.set_tab_dropdown_up(KeyEvent::new(KeyCode::Up, KeyMod::CTRL.with_shift()));
    km.set_tab_dropdown_down(KeyEvent::new(KeyCode::Down, KeyMod::CTRL.with_shift()));
    km.set_focus_up(KeyEvent::new(KeyCode::F(127), KeyMod::NONE));
    km.set_focus_down(KeyEvent::new(KeyCode::F(127), KeyMod::NONE));
    ws.set_keymap(km);
}
