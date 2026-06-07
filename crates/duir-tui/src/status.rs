//! Status bar construction using proper txv `StatusBar` with `KeyLabelView`, `ModalKey`.

use txv_core::clipboard_ring::ClipboardHandle;
use txv_core::prelude::*;
use txv_core::status_bar::{StatusBar, StatusSlot};
use txv_widgets::tiled_workspace::TiledWorkspace;
use txv_widgets::tiled_workspace::commands::{CM_TW_ACTIVATE_TAB, CM_TW_FOCUS_PANEL, CM_TW_ZOOM};
use txv_widgets::{InputLine, KeyLabelView, ModalKey};

use crate::handler::{CM_APP_QUIT, CM_EXECUTE_COMMAND, CM_SHOW_HELP};

/// Build the duir status bar.
pub fn build_status_bar(desktop: &TiledWorkspace, clipboard: ClipboardHandle) -> StatusBar {
    let mut bar = StatusBar::new();

    // Register TiledWorkspace's default bindings (hidden — no label)
    for (k, command, _payload) in desktop.default_bindings() {
        bar.add(StatusSlot::new(Box::new(KeyLabelView::new(k, command, ""))));
    }

    // Visible app bindings
    let f = |code| KeyEvent::new(code, KeyMod::NONE);
    bar.add(StatusSlot::new(Box::new(KeyLabelView::new(
        f(KeyCode::F(1)),
        CM_SHOW_HELP,
        "~F1~:Help",
    ))));
    bar.add(StatusSlot::new(Box::new(
        KeyLabelView::new(f(KeyCode::F(2)), CM_TW_FOCUS_PANEL, "~F2~:Tree").with_data(0),
    )));
    bar.add(StatusSlot::new(Box::new(
        KeyLabelView::new(f(KeyCode::F(3)), CM_TW_FOCUS_PANEL, "~F3~:Notes").with_data(1),
    )));
    bar.add(StatusSlot::new(Box::new(
        KeyLabelView::new(f(KeyCode::F(4)), CM_TW_FOCUS_PANEL, "~F4~:Tools").with_data(2),
    )));
    bar.add(StatusSlot::new(Box::new(KeyLabelView::new(
        f(KeyCode::F(5)),
        CM_TW_ZOOM,
        "~F5~:Zoom",
    ))));
    bar.add(
        StatusSlot::new(Box::new(KeyLabelView::new(
            KeyEvent::new(KeyCode::Char('q'), KeyMod::CTRL),
            CM_APP_QUIT,
            "~C-q~:Quit",
        )))
        .priority(9),
    );

    // Alt-1..9 tab select
    for i in 1..10u8 {
        let k = KeyEvent::new(KeyCode::Char((b'0' + i) as char), KeyMod::ALT);
        bar.add(StatusSlot::new(Box::new(
            KeyLabelView::new(k, CM_TW_ACTIVATE_TAB, "").with_data(u16::from(i - 1)),
        )));
    }

    // Command line (M-x / :)
    let input = InputLine::new()
        .with_clipboard(clipboard)
        .with_command(CM_EXECUTE_COMMAND);
    let command_line = ModalKey::new("M-x", ":")
        .trigger_key(KeyEvent::new(KeyCode::Char('x'), KeyMod::ALT))
        .trigger_key(KeyEvent::new(KeyCode::Char(':'), KeyMod::NONE))
        .terminal_command(CM_EXECUTE_COMMAND)
        .add_child(Box::new(input));
    bar.add(StatusSlot::new(Box::new(command_line)).priority(10).stretch(1));

    bar
}
