use crossterm::event::{KeyCode, KeyEvent};

use crate::app::{App, FocusState};

pub(super) fn handle_edit_key(app: &mut App, key: KeyEvent) -> bool {
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
