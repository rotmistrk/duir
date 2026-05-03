use std::io;
use std::path::PathBuf;
use std::time::Duration;

use crossterm::event::{Event, KeyCode, KeyModifiers};
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;

use duir_core::FileStorage;

use crate::app::{self, App, FocusState};
use crate::password;
use crate::render;

#[allow(clippy::too_many_lines)]
pub fn run_loop(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut App,
    storage_dir: &PathBuf,
    config: &duir_core::config::Config,
) -> io::Result<()> {
    let mut last_save = std::time::Instant::now();
    let autosave_interval = config.editor.autosave_interval_secs;

    loop {
        terminal.draw(|frame| render::render_frame(frame, app))?;

        // Process pending crypto after redraw (so "Working..." is visible)
        if let Some((password, action)) = app.pending_crypto.take() {
            app.handle_password_result(&password, action);
            continue; // redraw to show result
        }

        // Block for input, with timeout only for autosave or active kirons
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

        // Poll active kirons for new output
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
            } else {
                crate::input::handle_key(app, key);
            }
        }

        // Autosave
        if app.is_tree_focused()
            && last_save.elapsed() >= Duration::from_secs(autosave_interval)
            && app.files.iter().any(|f| f.autosave && f.is_modified())
            && let Ok(storage) = FileStorage::new(storage_dir)
        {
            app.save_all(&storage);
            last_save = std::time::Instant::now();
        }

        if app.should_quit {
            break;
        }
    }

    Ok(())
}

/// Handle overlay input (password prompt, about screen, help screen).
/// Returns true if the event was consumed by an overlay.
fn handle_overlay_input(app: &mut App, key: crossterm::event::KeyEvent) -> bool {
    // Password prompt overlay
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

    // About screen overlay
    if app.is_about_shown() {
        app.state = FocusState::Tree;
        return true;
    }

    // Help screen overlay
    if let FocusState::Help { ref mut scroll } = app.state {
        match key.code {
            KeyCode::Esc | KeyCode::Char('q') => app.state = FocusState::Tree,

            KeyCode::Down | KeyCode::Char('j') => *scroll += 1,

            KeyCode::Up | KeyCode::Char('k') => {
                *scroll = scroll.saturating_sub(1);
            }

            KeyCode::PageDown => *scroll += 20,

            KeyCode::PageUp => *scroll = scroll.saturating_sub(20),

            _ => {}
        }

        return true;
    }

    false
}

/// Handle global key bindings (Ctrl+S, Ctrl+T, Ctrl+R, kiro routing).
/// Returns true if the event was consumed.
#[allow(clippy::too_many_lines)]
fn handle_global_keys(app: &mut App, key: crossterm::event::KeyEvent, storage_dir: &PathBuf) -> bool {
    // F5 or macOS ∞ (Opt+5): toggle zoom
    if matches!(key.code, KeyCode::F(5) | KeyCode::Char('∞')) {
        app.zoomed = !app.zoomed;
        return true;
    }

    // « (macOS Opt+\) or Ctrl+\: send to kiro (works from ANY panel)
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

    // Ctrl+S: save all
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

    // Ctrl+R: capture kiro response (only when tree-focused, not in kiro PTY)
    if key.code == KeyCode::Char('r')
        && key.modifiers.contains(KeyModifiers::CONTROL)
        && !app.is_kiro_focused()
        && app.is_tree_focused()
        && app.active_kiron_for_cursor().is_some()
    {
        app.capture_kiro_response();
        return true;
    }

    // F2 or Alt+2 or macOS ™ (Opt+2): keyboard to tree, right panel unchanged
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

    // F3 or Alt+3 or macOS £ (Opt+3): focus note
    if (matches!(key.code, KeyCode::F(3))
        || matches!(key.code, KeyCode::Char('3') if key.modifiers.contains(KeyModifiers::ALT))
        || matches!(key.code, KeyCode::Char('£')))
        && !app.is_command_active()
        && !app.is_filter_active()
    {
        if !app.is_note_focused() {
            app.kiro_tab_focused = false;
            app.focus_note();
        }
        return true;
    }

    // F4 or Alt+4 or macOS ¢ (Opt+4): focus kiro (if active)
    if (matches!(key.code, KeyCode::F(4))
        || matches!(key.code, KeyCode::Char('4') if key.modifiers.contains(KeyModifiers::ALT))
        || matches!(key.code, KeyCode::Char('¢')))
        && app.active_kiron_for_cursor().is_some()
    {
        if app.is_note_focused() {
            app.save_editor();
        }
        app.state = FocusState::Kiro;
        app.kiro_tab_focused = true;
        app.clear_response_ready();
        return true;
    }

    // Ctrl+T: cycle Tree → Note → Kiro → Tree
    if key.code == KeyCode::Char('t')
        && key.modifiers.contains(KeyModifiers::CONTROL)
        && !app.is_command_active()
        && !app.is_filter_active()
        && !app.is_editing_title()
    {
        let has_kiron = app.active_kiron_for_cursor().is_some();

        if app.is_kiro_focused() {
            app.state = FocusState::Tree; // Kiro → Tree
        } else if app.is_note_focused() {
            app.save_editor();
            if has_kiron {
                app.state = FocusState::Kiro; // Note → Kiro
                app.kiro_tab_focused = true;
                app.clear_response_ready();
            } else {
                app.state = FocusState::Tree; // Note → Tree
            }
        } else {
            app.focus_note(); // Tree → Note
        }

        return true;
    }

    // Route keys to kiro PTY when focused (including Esc — kiro uses it).
    // Ctrl+S and Ctrl+T are already handled above; pass other Ctrl keys through.
    // PgUp/PgDn scroll the terminal buffer instead of being sent to the PTY.
    if app.is_kiro_focused() && app.active_kiron_for_cursor().is_some() {
        // PgUp/PgDn: scroll kiro terminal buffer
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
            // Snap to bottom on any non-scroll input
            kiron.pty.termbuf.scroll_to_bottom();
            let bytes = key_to_bytes(key);
            if !bytes.is_empty() {
                kiron.pty.write(&bytes);
            }
        }

        return true;
    }

    false
}

/// Test-only wrapper for `handle_global_keys`.
#[cfg(test)]
pub fn handle_global_keys_for_test(app: &mut App, key: crossterm::event::KeyEvent, storage_dir: &PathBuf) -> bool {
    handle_global_keys(app, key, storage_dir)
}

/// Convert a crossterm `KeyEvent` to bytes for PTY input.
pub fn key_to_bytes(key: crossterm::event::KeyEvent) -> Vec<u8> {
    match key.code {
        KeyCode::Char(c) => {
            if key.modifiers.contains(KeyModifiers::CONTROL) {
                let ctrl = (c as u8).wrapping_sub(b'a').wrapping_add(1);
                vec![ctrl]
            } else {
                let mut buf = [0u8; 4];
                let s = c.encode_utf8(&mut buf);
                s.as_bytes().to_vec()
            }
        }

        KeyCode::Enter => vec![b'\r'],

        KeyCode::Backspace => vec![0x7f],

        KeyCode::Tab => vec![b'\t'],

        KeyCode::Esc => vec![0x1b],

        KeyCode::Up => b"\x1b[A".to_vec(),

        KeyCode::Down => b"\x1b[B".to_vec(),

        KeyCode::Right => b"\x1b[C".to_vec(),

        KeyCode::Left => b"\x1b[D".to_vec(),

        KeyCode::Home => b"\x1b[H".to_vec(),

        KeyCode::End => b"\x1b[F".to_vec(),

        KeyCode::PageUp => b"\x1b[5~".to_vec(),

        KeyCode::PageDown => b"\x1b[6~".to_vec(),

        KeyCode::Delete => b"\x1b[3~".to_vec(),

        _ => Vec::new(),
    }
}
