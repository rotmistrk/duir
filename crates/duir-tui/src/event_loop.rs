use std::io;
use std::path::PathBuf;
use std::time::Duration;

use crossterm::event::{Event, KeyCode, KeyModifiers};
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;

use duir_core::FileStorage;

use crate::app::{self, App, FocusState};
use crate::event_helpers::{handle_file_changed, key_to_bytes, save_file_order};
use crate::password;
use crate::render;

pub fn run_loop(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut App,
    storage_dir: &PathBuf,
    config: &duir_core::config::Config,
    watcher_rx: Option<&std::sync::mpsc::Receiver<crate::file_watcher::FileChanged>>,
) -> io::Result<()> {
    let mut last_save = std::time::Instant::now();
    let autosave_interval = config.editor.autosave_interval_secs;

    loop {
        terminal.draw(|frame| render::render_frame(frame, app))?;

        if let Some((password, action)) = app.pending_crypto.take() {
            app.handle_password_result(&password, action);
            continue;
        }

        let has_pending_save = app.is_tree_focused() && app.files.iter().any(|f| f.autosave && f.is_modified());
        let has_active_kirons = !app.active_kirons.is_empty();

        let timeout = if app.pending_crypto.is_some() {
            Duration::from_millis(1)
        } else if has_active_kirons {
            Duration::from_millis(50)
        } else if has_pending_save {
            Duration::from_secs(autosave_interval)
        } else {
            Duration::from_secs(3600)
        };

        if has_active_kirons {
            app.poll_kirons();
        }

        if let Some(Event::Key(key)) = crate::input::poll_event(timeout)? {
            if handle_overlay_input(app, key) {
                continue;
            }

            if handle_global_keys(app, key, storage_dir) {
                // handled
            } else if app.is_command_active() && key.code == KeyCode::Enter {
                if let Ok(storage) = FileStorage::new(storage_dir) {
                    app.execute_command(&storage);
                }
            } else if matches!(app.state, crate::app::FocusState::Resolve(_)) && key.code == KeyCode::Enter {
                if let Ok(storage) = FileStorage::new(storage_dir) {
                    app.resolve_apply(&storage);
                }
            } else {
                crate::input::handle_key(app, key);
            }
        }

        if app.is_tree_focused()
            && last_save.elapsed() >= Duration::from_secs(autosave_interval)
            && app.files.iter().any(|f| f.autosave && f.is_modified())
            && let Ok(storage) = FileStorage::new(storage_dir)
        {
            app.save_all(&storage);
            last_save = std::time::Instant::now();
        }

        if let Some(rx) = watcher_rx {
            while let Ok(event) = rx.try_recv() {
                handle_file_changed(app, storage_dir, &event.path);
            }
        }

        if app.flags.should_quit() {
            save_file_order(app, config);
            break;
        }
    }

    Ok(())
}

/// Handle overlay input (password prompt, about screen, help screen).
fn handle_overlay_input(app: &mut App, key: crossterm::event::KeyEvent) -> bool {
    if let Some(prompt) = &mut app.password_prompt {
        match prompt.handle_key(key) {
            crate::password::PromptResult::Submitted(pw) => {
                if let Some(prompt) = app.password_prompt.take() {
                    let msg = match &prompt.callback {
                        password::PasswordAction::Decrypt { .. } => "⏳ Decrypting...",
                        password::PasswordAction::Encrypt { .. } => "⏳ Encrypting...",
                        password::PasswordAction::ChangePassword { .. } => "⏳ Re-encrypting...",
                    };
                    app.set_status(msg, app::StatusLevel::Warning);
                    app.pending_crypto = Some((pw, prompt.callback));
                }
            }
            crate::password::PromptResult::Cancelled => {
                app.password_prompt = None;
            }
            crate::password::PromptResult::Pending => {}
        }
        return true;
    }

    if app.is_about_shown() {
        app.state = FocusState::Tree;
        return true;
    }

    if crate::help::handle_help_input(app, key) {
        return true;
    }

    false
}

/// Handle global key bindings (Ctrl+S, Ctrl+T, Ctrl+R, kiro routing).
fn handle_global_keys(app: &mut App, key: crossterm::event::KeyEvent, storage_dir: &PathBuf) -> bool {
    if matches!(key.code, KeyCode::F(5) | KeyCode::Char('∞')) {
        let v = !app.flags.zoomed();
        app.flags.set_zoomed(v);
        return true;
    }

    if (matches!(key.code, KeyCode::Char('«'))
        || (key.code == KeyCode::Char('\\') && key.modifiers.contains(KeyModifiers::CONTROL)))
        && app.active_kiron_for_cursor().is_some()
    {
        if app.is_note_focused() {
            app.save_editor();
            app.state = FocusState::Tree;
        }
        app.send_to_kiro();
        return true;
    }

    if key.code == KeyCode::Char('s')
        && key.modifiers.contains(KeyModifiers::CONTROL)
        && !app.is_editing_title()
        && !app.is_filter_active()
        && !app.is_command_active()
    {
        if let Ok(storage) = FileStorage::new(storage_dir) {
            app.save_all(&storage);
        }
        return true;
    }

    if key.code == KeyCode::Char('r')
        && key.modifiers.contains(KeyModifiers::CONTROL)
        && !app.is_kiro_focused()
        && app.is_tree_focused()
        && app.active_kiron_for_cursor().is_some()
    {
        app.capture_kiro_response();
        return true;
    }

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

/// Route a key event to the active kiro PTY.
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

/// Test-only wrapper for `handle_global_keys`.
#[cfg(test)]
pub fn handle_global_keys_for_test(app: &mut App, key: crossterm::event::KeyEvent, storage_dir: &PathBuf) -> bool {
    handle_global_keys(app, key, storage_dir)
}
