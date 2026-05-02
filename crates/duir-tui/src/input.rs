use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};

use crate::app::{App, Focus};

/// Handle a key event, returning true if the app should repaint.
pub fn handle_key(app: &mut App, key: KeyEvent) -> bool {
    // Command mode
    if app.command_active {
        return handle_command_key(app, key);
    }

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
            app.load_editor();
            app.focus = Focus::Note;
            true
        }
        (KeyCode::Char('/'), false) => {
            app.filter_active = true;
            app.filter_saved = app.filter_text.clone();
            // Keep current filter text for editing
            true
        }
        (KeyCode::Char(':'), false) => {
            app.command_active = true;
            app.command_buffer.clear();
            app.completer.update("");
            true
        }
        (KeyCode::F(1), false) => {
            app.show_help = true;
            app.help_scroll = 0;
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
    // Tab in normal mode returns to tree
    if let Some(editor) = &app.editor
        && editor.mode == crate::note_editor::EditorMode::Normal
        && key.code == KeyCode::Tab
    {
        app.save_editor();
        app.focus = Focus::Tree;
        return true;
    }

    app.editor.as_mut().is_some_and(|editor| editor.handle_key(key))
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
        KeyCode::Left => {
            app.edit_select_all = false;
            if app.edit_cursor > 0 {
                app.edit_cursor -= 1;
            }
            true
        }
        KeyCode::Right => {
            app.edit_select_all = false;
            if app.edit_cursor < app.edit_buffer.len() {
                app.edit_cursor += 1;
            }
            true
        }
        KeyCode::Home => {
            app.edit_select_all = false;
            app.edit_cursor = 0;
            true
        }
        KeyCode::End => {
            app.edit_select_all = false;
            app.edit_cursor = app.edit_buffer.len();
            true
        }
        KeyCode::Backspace => {
            if app.edit_select_all {
                app.edit_buffer.clear();
                app.edit_cursor = 0;
                app.edit_select_all = false;
            } else if app.edit_cursor > 0 {
                app.edit_buffer.remove(app.edit_cursor - 1);
                app.edit_cursor -= 1;
            }
            true
        }
        KeyCode::Delete => {
            if app.edit_select_all {
                app.edit_buffer.clear();
                app.edit_cursor = 0;
                app.edit_select_all = false;
            } else if app.edit_cursor < app.edit_buffer.len() {
                app.edit_buffer.remove(app.edit_cursor);
            }
            true
        }
        KeyCode::Char(c) => {
            if app.edit_select_all {
                app.edit_buffer.clear();
                app.edit_cursor = 0;
                app.edit_select_all = false;
            }
            app.edit_buffer.insert(app.edit_cursor, c);
            app.edit_cursor += 1;
            true
        }
        _ => false,
    }
}

/// Path-taking commands: after these prefixes, Tab completes file paths.
const PATH_COMMANDS: &[&str] = &["import ", "export ", "open ", "o ", "w ", "write ", "saveas "];

fn complete_command_or_path(app: &mut App, reverse: bool) {
    // Check if we're in a path-completing context
    let needs_path = PATH_COMMANDS
        .iter()
        .any(|prefix| app.command_buffer.starts_with(prefix));

    if needs_path {
        // Extract the command prefix and the path part
        let split_pos = app.command_buffer.find(' ').unwrap_or(app.command_buffer.len()) + 1;
        let (cmd_prefix, path_part) = app.command_buffer.split_at(split_pos.min(app.command_buffer.len()));
        let completions = crate::completer::complete_path(path_part);
        if completions.is_empty() {
            return;
        }
        // Cycle through completions
        let current_path = path_part.to_owned();
        let idx = completions.iter().position(|c| *c == current_path);
        let next_idx = if reverse {
            idx.map_or(completions.len() - 1, |i| {
                if i == 0 { completions.len() - 1 } else { i - 1 }
            })
        } else {
            idx.map_or(0, |i| (i + 1) % completions.len())
        };
        app.command_buffer = format!("{cmd_prefix}{}", completions[next_idx]);
    } else {
        app.completer.update(&app.command_buffer);
        let completion = if reverse {
            app.completer.prev()
        } else {
            app.completer.next()
        };
        if let Some(c) = completion {
            app.command_buffer = c.to_owned();
        }
    }
}
fn handle_filter_key(app: &mut App, key: KeyEvent) -> bool {
    match key.code {
        KeyCode::Esc => {
            // Revert to saved filter state
            app.filter_active = false;
            app.filter_text.clone_from(&app.filter_saved);
            if app.filter_text.is_empty() {
                app.filter_exclude = false;
                app.status_message.clear();
                app.rebuild_rows();
            } else {
                app.apply_filter();
            }
            true
        }
        KeyCode::Enter => {
            app.filter_active = false;
            if let Some(rest) = app.filter_text.strip_prefix('!') {
                app.filter_exclude = true;
                app.filter_text = rest.to_owned();
            } else {
                app.filter_exclude = false;
            }
            if app.filter_text.is_empty() {
                app.status_message.clear();
                app.rebuild_rows();
            } else {
                app.apply_filter();
            }
            true
        }
        KeyCode::Backspace => {
            app.filter_text.pop();
            app.apply_filter_live();
            true
        }
        KeyCode::Char(c) => {
            app.filter_text.push(c);
            app.apply_filter_live();
            true
        }
        _ => false,
    }
}

/// Returns true if the command needs storage access (`execute_command` should be called).
fn handle_command_key(app: &mut App, key: KeyEvent) -> bool {
    match key.code {
        KeyCode::Esc => {
            app.command_active = false;
            app.command_buffer.clear();
            app.command_history_index = None;
            app.completer.matches.clear();
            true
        }
        KeyCode::Enter => {
            let cmd = app.command_buffer.trim().to_owned();
            if !cmd.is_empty() {
                app.command_history.push(cmd);
            }
            app.command_history_index = None;
            app.completer.matches.clear();
            true
        }
        KeyCode::Tab => {
            complete_command_or_path(app, false);
            true
        }
        KeyCode::BackTab => {
            complete_command_or_path(app, true);
            true
        }
        KeyCode::Up => {
            if !app.command_history.is_empty() {
                let idx = app
                    .command_history_index
                    .map_or(app.command_history.len() - 1, |i| i.saturating_sub(1));
                app.command_history_index = Some(idx);
                app.command_buffer.clone_from(&app.command_history[idx]);
                app.completer.update(&app.command_buffer);
            }
            true
        }
        KeyCode::Down => {
            if let Some(idx) = app.command_history_index {
                if idx + 1 < app.command_history.len() {
                    app.command_history_index = Some(idx + 1);
                    app.command_buffer.clone_from(&app.command_history[idx + 1]);
                } else {
                    app.command_history_index = None;
                    app.command_buffer.clear();
                }
            }
            app.completer.update(&app.command_buffer);
            true
        }
        KeyCode::Backspace => {
            if app.command_buffer.is_empty() {
                app.command_active = false;
                app.command_history_index = None;
                app.completer.matches.clear();
            } else {
                app.command_buffer.pop();
                app.completer.update(&app.command_buffer);
                app.completer.reset_selection();
            }
            true
        }
        KeyCode::Char(c) => {
            app.command_history_index = None;
            app.command_buffer.push(c);
            app.completer.update(&app.command_buffer);
            app.completer.reset_selection();
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
