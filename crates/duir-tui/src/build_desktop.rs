//! Workspace builder — constructs the initial duir 4-slot layout.
//!
//! Layout: Left=tree, Center=notes, Right=shell/kiro, Bottom=messages.
//! Tree zoomed on start.

use std::path::Path;

use txv_widgets::tiled_workspace::TiledWorkspace;
use txv_widgets::tiled_workspace::types::{PanelConfig, PanelPosition, SplitNode};

use crate::note_view::NoteView;
use crate::shell::new_shell_terminal;
use crate::slots::{PANEL_COUNT, SlotId};
use crate::todo_tree::TodoTreeView;

/// Build duir's 4-slot workspace with tree zoomed on start.
pub fn build_workspace(root_dir: &Path) -> TiledWorkspace {
    let configs = vec![
        PanelConfig::fixed("Tree", PanelPosition::Left),
        PanelConfig::new("Notes", PanelPosition::Center),
        PanelConfig::new("Tools", PanelPosition::Right),
        PanelConfig::new("Messages", PanelPosition::Bottom),
    ];

    let wide_layout = SplitNode::v(vec![
        (
            0.8,
            SplitNode::h(vec![
                (0.25, SplitNode::leaf(0)),
                (0.40, SplitNode::leaf(1)),
                (0.35, SplitNode::leaf(2)),
            ]),
        ),
        (0.2, SplitNode::leaf(3)),
    ]);

    let narrow_layout = SplitNode::v(vec![
        (
            0.6,
            SplitNode::h(vec![(0.3, SplitNode::leaf(0)), (0.7, SplitNode::leaf(1))]),
        ),
        (0.25, SplitNode::leaf(2)),
        (0.15, SplitNode::leaf(3)),
    ]);

    let mut ws = TiledWorkspace::new(configs, wide_layout, narrow_layout, 300);
    ws.set_handle_keys(false);
    ws.set_v_divider_gaps(false);

    for i in 0..PANEL_COUNT {
        if let Some(panel) = ws.panel_mut(i) {
            panel.bar_mut().set_handle_keys(false);
        }
    }

    // Insert tree view in left slot
    let tree = TodoTreeView::new(root_dir);
    ws.insert_tab(SlotId::Left as usize, "Todo", Box::new(tree));

    // Insert note editor in center slot
    let note = NoteView::new();
    ws.insert_tab(SlotId::Center as usize, "Note", Box::new(note));

    // Insert shell in right slot
    let shell = new_shell_terminal();
    ws.insert_tab(SlotId::Right as usize, "Shell:0", shell);

    // Focus left panel and zoom it on start
    ws.focus_panel(SlotId::Left as usize);
    ws.set_zoomed(Some(SlotId::Left as usize));

    ws
}
