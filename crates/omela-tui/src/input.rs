use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};

use crate::app::{App, Focus};

/// Handle a key event, returning true if the app should repaint.
pub fn handle_key(app: &mut App, key: KeyEvent) -> bool {
    // Filter mode input
    if app.filter_active {
        return handle_filter_key(app, key);
    }

    // Title editing mode
    if app.editing_title {
        return handle_edit_key(app, key);
    }

    // Normal mode
    match app.focus {
        Focus::Tree => handle_tree_key(app, key),
        Focus::Note => handle_note_key(app, key),
    }
}

fn handle_tree_key(app: &mut App, key: KeyEvent) -> bool {
    let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);

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
        (KeyCode::Char('!'), false) => {
            app.toggle_important();
            true
        }
        (KeyCode::Up, true) => {
            app.swap_up();
            true
        }
        (KeyCode::Down, true) => {
            app.swap_down();
            true
        }
        (KeyCode::Left, true) => {
            app.promote();
            true
        }
        (KeyCode::Right, true) => {
            app.demote();
            true
        }
        (KeyCode::Char('s'), true) => {
            // Save handled in main loop
            true
        }
        (KeyCode::Tab, false) => {
            app.focus = Focus::Note;
            true
        }
        (KeyCode::Char('/'), false) => {
            app.filter_active = true;
            app.filter_text.clear();
            true
        }
        (KeyCode::Char('S'), false) => {
            app.sort_children();
            true
        }
        _ => false,
    }
}

const fn handle_note_key(app: &mut App, key: KeyEvent) -> bool {
    match key.code {
        KeyCode::Up => {
            if app.note_scroll > 0 {
                app.note_scroll -= 1;
            }
            true
        }
        KeyCode::Down => {
            app.note_scroll += 1;
            true
        }
        KeyCode::Tab | KeyCode::Esc | KeyCode::Char('q') => {
            app.focus = Focus::Tree;
            true
        }
        _ => false,
    }
}

fn handle_edit_key(app: &mut App, key: KeyEvent) -> bool {
    match key.code {
        KeyCode::Enter => {
            app.finish_editing();
            true
        }
        KeyCode::Esc => {
            app.cancel_editing();
            true
        }
        KeyCode::Backspace => {
            if app.edit_select_all {
                app.edit_buffer.clear();
                app.edit_select_all = false;
            } else {
                app.edit_buffer.pop();
            }
            true
        }
        KeyCode::Char(c) => {
            if app.edit_select_all {
                app.edit_buffer.clear();
                app.edit_select_all = false;
            }
            app.edit_buffer.push(c);
            true
        }
        _ => false,
    }
}

fn handle_filter_key(app: &mut App, key: KeyEvent) -> bool {
    match key.code {
        KeyCode::Esc => {
            app.filter_active = false;
            app.filter_text.clear();
            app.status_message.clear();
            true
        }
        KeyCode::Enter => {
            app.filter_active = false;
            app.status_message = format!("Filter: {}", app.filter_text);
            true
        }
        KeyCode::Backspace => {
            app.filter_text.pop();
            true
        }
        KeyCode::Char(c) => {
            app.filter_text.push(c);
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
