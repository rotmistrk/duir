use crossterm::event::{KeyCode, KeyModifiers};

use crate::app::{App, FocusState};
use crate::event_helpers::key_to_bytes;

pub fn handle_focus_keys(app: &mut App, key: crossterm::event::KeyEvent) -> bool {
    if matches!(key.code, KeyCode::F(2))
        || matches!(key.code, KeyCode::Char('2') if key.modifiers.contains(KeyModifiers::ALT))
        || matches!(key.code, KeyCode::Char('™'))
    {
        if app.is_note_focused() {
            app.save_editor();
        }
        app.state = FocusState::Tree;
        return true;
    }

    if (matches!(key.code, KeyCode::F(3))
        || matches!(key.code, KeyCode::Char('3') if key.modifiers.contains(KeyModifiers::ALT))
        || matches!(key.code, KeyCode::Char('£')))
        && !app.is_command_active()
        && !app.is_filter_active()
    {
        if !app.is_note_focused() {
            app.flags.set_kiro_tab_focused(false);
            app.focus_note();
        }
        return true;
    }

    if (matches!(key.code, KeyCode::F(4))
        || matches!(key.code, KeyCode::Char('4') if key.modifiers.contains(KeyModifiers::ALT))
        || matches!(key.code, KeyCode::Char('¢')))
        && app.active_kiron_for_cursor().is_some()
    {
        if app.is_note_focused() {
            app.save_editor();
        }
        app.state = FocusState::Kiro;
        app.flags.set_kiro_tab_focused(true);
        app.clear_response_ready();
        return true;
    }

    if key.code == KeyCode::Char('l')
        && key.modifiers.contains(KeyModifiers::CONTROL)
        && !app.is_command_active()
        && !app.is_filter_active()
        && !app.is_editing_title()
        && !app.is_kiro_focused()
    {
        app.cmd_layout(None);
        return true;
    }

    if key.code == KeyCode::Char('t')
        && key.modifiers.contains(KeyModifiers::CONTROL)
        && !app.is_command_active()
        && !app.is_filter_active()
        && !app.is_editing_title()
    {
        let has_kiron = app.active_kiron_for_cursor().is_some();
        if app.is_kiro_focused() {
            app.state = FocusState::Tree;
        } else if app.is_note_focused() {
            app.save_editor();
            if has_kiron {
                app.state = FocusState::Kiro;
                app.flags.set_kiro_tab_focused(true);
                app.clear_response_ready();
            } else {
                app.state = FocusState::Tree;
            }
        } else {
            app.focus_note();
        }
        return true;
    }

    if app.is_kiro_focused() && app.active_kiron_for_cursor().is_some() {
        return route_kiro_key(app, key);
    }

    false
}

fn route_kiro_key(app: &mut App, key: crossterm::event::KeyEvent) -> bool {
    if matches!(key.code, KeyCode::PageUp | KeyCode::PageDown) {
        if let Some(kiron_key) = app.active_kiron_for_cursor()
            && let Some(kiron) = app.active_kirons.get_mut(&kiron_key)
        {
            let half = kiron.pty.termbuf.rows() / 2;
            match key.code {
                KeyCode::PageUp => kiron.pty.termbuf.scroll_up(half),
                KeyCode::PageDown => kiron.pty.termbuf.scroll_down(half),
                _ => {}
            }
        }
        return true;
    }

    if let Some(kiron_key) = app.active_kiron_for_cursor()
        && let Some(kiron) = app.active_kirons.get_mut(&kiron_key)
    {
        kiron.pty.termbuf.scroll_to_bottom();
        let bytes = key_to_bytes(key);
        if !bytes.is_empty() {
            kiron.pty.write(&bytes);
        }
    }

    true
}
