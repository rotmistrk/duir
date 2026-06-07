//! Workspace builder — constructs the initial duir 3-slot layout.
//!
//! Layout: Left=tree, Center=notes, Right=shell/kiro/messages/clipboard.
//! Tree zoomed on start. Right panel uses LRU tab bar.

use std::path::Path;

use txv_core::clipboard_ring::ClipboardHandle;
use txv_widgets::tiled_workspace::TiledWorkspace;
use txv_widgets::tiled_workspace::types::{PanelConfig, PanelPosition, SplitNode};

use crate::clipboard_view::ClipboardView;
use crate::messages::MessagesView;
use crate::note_view::NoteView;
use crate::shell::new_shell_terminal;
use crate::slots::{PANEL_COUNT, SlotId};
use crate::todo_tree::TodoTreeView;

/// Build duir's 3-slot workspace with tree zoomed on start.
pub fn build_workspace(root_dir: &Path, clipboard: ClipboardHandle) -> TiledWorkspace {
    let configs = vec![
        PanelConfig::fixed("Tree", PanelPosition::Left),
        PanelConfig::new("Editor", PanelPosition::Center),
        PanelConfig {
            splittable: true,
            ..PanelConfig::new("Tools", PanelPosition::Right)
        },
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
    let note = NoteView::new();
    ws.insert_tab(SlotId::Center as usize, "Note", Box::new(note));

    // Right: shell, messages, clipboard viewer (LRU)
    let shell = new_shell_terminal();
    ws.insert_tab(SlotId::Right as usize, "Shell:0", shell);

    let messages = MessagesView::new();
    ws.insert_tab(SlotId::Right as usize, "Messages", Box::new(messages));

    let clip_view = ClipboardView::new(clipboard);
    ws.insert_tab(SlotId::Right as usize, "Clipboard", Box::new(clip_view));

    // Focus tree, zoom it
    ws.focus_panel(SlotId::Left as usize);
    ws.set_zoomed(Some(SlotId::Left as usize));

    ws
}
