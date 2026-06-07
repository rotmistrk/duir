//! Status bar construction — proper `StatusBar` with `KeyLabelView`, `ModalKey`, etc.

use txv_core::clipboard_ring::ClipboardHandle;
use txv_core::prelude::*;
use txv_core::status_bar::{StatusBar, StatusSlot};
use txv_widgets::tiled_workspace::TiledWorkspace;
use txv_widgets::tiled_workspace::commands::{CM_TW_FOCUS_PANEL, CM_TW_ZOOM};
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
    bar.add(StatusSlot::new(Box::new(KeyLabelView::new(
        key(KeyCode::F(1)),
        CM_SHOW_HELP,
        "~F1~:Help",
    ))));
    bar.add(StatusSlot::new(Box::new(
        KeyLabelView::new(key(KeyCode::F(2)), CM_TW_FOCUS_PANEL, "~F2~:Tree").with_data(0),
    )));
    bar.add(StatusSlot::new(Box::new(
        KeyLabelView::new(key(KeyCode::F(3)), CM_TW_FOCUS_PANEL, "~F3~:Notes").with_data(1),
    )));
    bar.add(StatusSlot::new(Box::new(
        KeyLabelView::new(key(KeyCode::F(4)), CM_TW_FOCUS_PANEL, "~F4~:Tools").with_data(2),
    )));
    bar.add(StatusSlot::new(Box::new(KeyLabelView::new(
        key(KeyCode::F(5)),
        CM_TW_ZOOM,
        "~F5~:Zoom",
    ))));
    bar.add(
        StatusSlot::new(Box::new(KeyLabelView::new(
            KeyEvent {
                code: KeyCode::Char('q'),
                modifiers: KeyMod {
                    ctrl: true,
                    shift: false,
                    alt: false,
                },
            },
            CM_APP_QUIT,
            "~C-q~:Quit",
        )))
        .priority(9),
    );

    // Ctrl-Shift-Left/Right for panel navigation
    bar.add(StatusSlot::new(Box::new(
        KeyLabelView::new(
            KeyEvent {
                code: KeyCode::Left,
                modifiers: KeyMod {
                    ctrl: true,
                    shift: true,
                    alt: false,
                },
            },
            CM_TW_FOCUS_PANEL,
            "",
        )
        .with_data(0),
    )));
    bar.add(StatusSlot::new(Box::new(
        KeyLabelView::new(
            KeyEvent {
                code: KeyCode::Right,
                modifiers: KeyMod {
                    ctrl: true,
                    shift: true,
                    alt: false,
                },
            },
            CM_TW_FOCUS_PANEL,
            "",
        )
        .with_data(2),
    )));

    // Command line (M-x / :)
    add_command_line(&mut bar, clipboard);

    bar
}

fn add_command_line(bar: &mut StatusBar, clipboard: ClipboardHandle) {
    let input = InputLine::new()
        .with_clipboard(clipboard)
        .with_command(CM_EXECUTE_COMMAND);
    let alt_x = KeyEvent {
        code: KeyCode::Char('x'),
        modifiers: KeyMod {
            ctrl: false,
            shift: false,
            alt: true,
        },
    };
    let colon = KeyEvent {
        code: KeyCode::Char(':'),
        modifiers: KeyMod {
            ctrl: false,
            shift: false,
            alt: false,
        },
    };
    let command_line = ModalKey::new("M-x", ":")
        .trigger_key(alt_x)
        .trigger_key(colon)
        .terminal_command(CM_EXECUTE_COMMAND)
        .add_child(Box::new(input));
    bar.add(StatusSlot::new(Box::new(command_line)).priority(10).stretch(1));
}

const fn key(code: KeyCode) -> KeyEvent {
    KeyEvent {
        code,
        modifiers: KeyMod {
            ctrl: false,
            shift: false,
            alt: false,
        },
    }
}
