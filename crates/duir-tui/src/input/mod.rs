mod command;

use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};

use crate::app::{App, FocusState};

/// Handle a key event, returning true if the app should repaint.
pub fn handle_key(app: &mut App, key: KeyEvent) -> bool {
    match &app.state {
        FocusState::Command { .. } => command::handle_command_key(app, key),
        FocusState::Filter { .. } => command::handle_filter_key(app, key),
        FocusState::EditingTitle { .. } => handle_edit_key(app, key),
        FocusState::Tree => handle_tree_key(app, key),
        FocusState::Note { .. } => handle_note_key(app, key),
        FocusState::Kiro | FocusState::Help { .. } | FocusState::About => false,
    }
}

#[allow(clippy::too_many_lines)]
fn handle_tree_key(app: &mut App, key: KeyEvent) -> bool {
    let shift = key.modifiers.contains(KeyModifiers::SHIFT);
    let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);

    // Clear pending delete on any key except 'y' (confirm)
    if app.pending_delete && key.code != KeyCode::Char('y') {
        app.pending_delete = false;
        app.status_message.clear();
    }

    // Shift+Arrow or HJKL: move items
    if shift && !ctrl {
        return match key.code {
            KeyCode::Up => {
                app.swap_up();
                true
            }
            KeyCode::Down => {
                app.swap_down();
                true
            }
            KeyCode::Left => {
                app.promote();
                true
            }
            KeyCode::Right => {
                app.demote();
                true
            }
            _ => false,
        };
    }

    match (key.code, ctrl) {
        (KeyCode::Char('q'), false) => {
            app.should_quit = true;
            true
        }
        (KeyCode::Up, false) => {
            app.move_up();
            true
        }
        (KeyCode::Down, false) => {
            app.move_down();
            true
        }
        (KeyCode::Left, false) => {
            app.collapse_current();
            true
        }
        (KeyCode::Right, false) => {
            app.expand_current();
            true
        }
        (KeyCode::Char(' '), false) => {
            app.toggle_completed();
            true
        }
        (KeyCode::Enter, false) => {
            // Enter: send to kiro if inside kiron subtree, otherwise no-op
            if app.active_kiron_for_cursor().is_some() {
                app.send_to_kiro();
            }
            true
        }
        (KeyCode::Char('e'), false) => {
            app.start_editing();
            true
        }
        (KeyCode::Char('n'), false) => {
            app.new_sibling();
            true
        }
        (KeyCode::Char('b'), false) => {
            app.new_child();
            true
        }
        (KeyCode::Char('d'), false) => {
            app.delete_current();
            true
        }
        (KeyCode::Char('y'), false) if app.pending_delete => {
            app.pending_delete = false;
            app.force_delete_current();
            true
        }
        (KeyCode::Char('!'), false) => {
            app.toggle_important();
            true
        }
        (KeyCode::Char('K'), false) => {
            app.swap_up();
            true
        }
        (KeyCode::Char('J'), false) => {
            app.swap_down();
            true
        }
        (KeyCode::Char('H'), false) => {
            app.promote();
            true
        }
        (KeyCode::Char('L'), false) => {
            app.demote();
            true
        }
        (KeyCode::Char('s'), true) => {
            // Save handled in main loop
            true
        }
        (KeyCode::Tab, false) => {
            app.focus_note();
            true
        }
        (KeyCode::Char('/'), false) => {
            let saved = app.filter_committed_text.clone();
            let text = app.filter_committed_text.clone();
            app.state = FocusState::Filter { text, saved };
            true
        }
        (KeyCode::Char(':'), false) => {
            app.state = FocusState::Command {
                buffer: String::new(),
                history_index: None,
            };
            app.completer.update("");
            true
        }
        (KeyCode::F(1), false) => {
            app.state = FocusState::Help { scroll: 0 };
            true
        }
        (KeyCode::Char(']'), false) => {
            app.note_panel_pct = (app.note_panel_pct + 5).min(80);
            true
        }
        (KeyCode::Char('['), false) => {
            app.note_panel_pct = app.note_panel_pct.saturating_sub(5).max(20);
            true
        }
        (KeyCode::Char('S'), false) => {
            app.sort_children();
            true
        }
        (KeyCode::Char('c'), false) => {
            app.clone_subtree();
            true
        }
        _ => false,
    }
}

fn handle_note_key(app: &mut App, key: KeyEvent) -> bool {
    if let FocusState::Note { ref editor, .. } = app.state {
        // Tab in normal mode returns to tree
        if editor.mode == crate::note_editor::EditorMode::Normal && key.code == KeyCode::Tab {
            app.focus_tree();
            return true;
        }
    }

    if let FocusState::Note { ref mut editor, .. } = app.state {
        editor.handle_key(key)
    } else {
        false
    }
}

fn handle_edit_key(app: &mut App, key: KeyEvent) -> bool {
    let FocusState::EditingTitle {
        ref mut buffer,
        ref mut cursor,
        ref mut select_all,
    } = app.state
    else {
        return false;
    };

    match key.code {
        KeyCode::Enter => {
            app.finish_editing();
            true
        }
        KeyCode::Esc => {
            app.cancel_editing();
            true
        }
        KeyCode::Left => {
            *select_all = false;
            if *cursor > 0 {
                *cursor -= 1;
            }
            true
        }
        KeyCode::Right => {
            *select_all = false;
            if *cursor < buffer.len() {
                *cursor += 1;
            }
            true
        }
        KeyCode::Home => {
            *select_all = false;
            *cursor = 0;
            true
        }
        KeyCode::End => {
            *select_all = false;
            *cursor = buffer.len();
            true
        }
        KeyCode::Backspace => {
            if *select_all {
                buffer.clear();
                *cursor = 0;
                *select_all = false;
            } else if *cursor > 0 {
                buffer.remove(*cursor - 1);
                *cursor -= 1;
            }
            true
        }
        KeyCode::Delete => {
            if *select_all {
                buffer.clear();
                *cursor = 0;
                *select_all = false;
            } else if *cursor < buffer.len() {
                buffer.remove(*cursor);
            }
            true
        }
        KeyCode::Char(c) => {
            if *select_all {
                buffer.clear();
                *cursor = 0;
                *select_all = false;
            }
            buffer.insert(*cursor, c);
            *cursor += 1;
            true
        }
        _ => false,
    }
}

/// Poll for the next event with a timeout.
///
/// # Errors
/// Returns an error if event polling fails.
pub fn poll_event(timeout: std::time::Duration) -> std::io::Result<Option<Event>> {
    if event::poll(timeout)? {
        Ok(Some(event::read()?))
    } else {
        Ok(None)
    }
}
