use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};

use crate::app::{App, FocusState};

/// Handle a key event, returning true if the app should repaint.
pub fn handle_key(app: &mut App, key: KeyEvent) -> bool {
    match &app.state {
        FocusState::Command { .. } => handle_command_key(app, key),
        FocusState::Filter { .. } => handle_filter_key(app, key),
        FocusState::EditingTitle { .. } => handle_edit_key(app, key),
        FocusState::Tree => handle_tree_key(app, key),
        FocusState::Note { .. } => handle_note_key(app, key),
        FocusState::Help { .. } | FocusState::About => false,
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

/// Path-taking commands: after these prefixes, Tab completes file paths.
const PATH_COMMANDS: &[&str] = &["import ", "export ", "open ", "o ", "w ", "write ", "saveas "];

fn complete_command_or_path(app: &mut App, reverse: bool) {
    let FocusState::Command { ref mut buffer, .. } = app.state else {
        return;
    };

    // Check if we're in a path-completing context
    let needs_path = PATH_COMMANDS.iter().any(|prefix| buffer.starts_with(prefix));

    if needs_path {
        let split_pos = buffer.find(' ').unwrap_or(buffer.len()) + 1;
        let (cmd_prefix, path_part) = buffer.split_at(split_pos.min(buffer.len()));
        let completions = crate::completer::complete_path(path_part);
        if completions.is_empty() {
            return;
        }
        let current_path = path_part.to_owned();
        let cmd_prefix = cmd_prefix.to_owned();
        let idx = completions.iter().position(|c| *c == current_path);
        let next_idx = if reverse {
            idx.map_or(completions.len() - 1, |i| {
                if i == 0 { completions.len() - 1 } else { i - 1 }
            })
        } else {
            idx.map_or(0, |i| (i + 1) % completions.len())
        };
        *buffer = format!("{cmd_prefix}{}", completions[next_idx]);
    } else {
        app.completer.update(buffer);
        let completion = if reverse {
            app.completer.prev()
        } else {
            app.completer.next()
        };
        if let Some(c) = completion
            && let FocusState::Command { ref mut buffer, .. } = app.state
        {
            c.clone_into(buffer);
        }
    }
}
fn handle_filter_key(app: &mut App, key: KeyEvent) -> bool {
    match key.code {
        KeyCode::Esc => {
            // Revert to saved filter state
            let saved = if let FocusState::Filter { ref saved, .. } = app.state {
                saved.clone()
            } else {
                String::new()
            };
            app.filter_committed_text = saved;
            app.state = FocusState::Tree;
            if app.filter_committed_text.is_empty() {
                app.filter_committed_exclude = false;
                app.status_message.clear();
                app.rebuild_rows();
            } else {
                app.apply_filter();
            }
            true
        }
        KeyCode::Enter => {
            let text = if let FocusState::Filter { ref text, .. } = app.state {
                text.clone()
            } else {
                String::new()
            };
            if let Some(rest) = text.strip_prefix('!') {
                app.filter_committed_exclude = true;
                app.filter_committed_text = rest.to_owned();
            } else {
                app.filter_committed_exclude = false;
                app.filter_committed_text = text;
            }
            app.state = FocusState::Tree;
            if app.filter_committed_text.is_empty() {
                app.status_message.clear();
                app.rebuild_rows();
            } else {
                app.apply_filter();
            }
            true
        }
        KeyCode::Backspace => {
            if let FocusState::Filter { ref mut text, .. } = app.state {
                text.pop();
            }
            app.apply_filter_live();
            true
        }
        KeyCode::Char(c) => {
            if let FocusState::Filter { ref mut text, .. } = app.state {
                text.push(c);
            }
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
            app.state = FocusState::Tree;
            app.completer.matches.clear();
            true
        }
        KeyCode::Enter => {
            if let FocusState::Command { ref buffer, .. } = app.state {
                let cmd = buffer.trim().to_owned();
                if !cmd.is_empty() {
                    app.command_history.push(cmd);
                }
            }
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
                let cur_idx = if let FocusState::Command { history_index, .. } = &app.state {
                    *history_index
                } else {
                    None
                };
                let idx = cur_idx.map_or(app.command_history.len() - 1, |i| i.saturating_sub(1));
                if let FocusState::Command {
                    ref mut buffer,
                    ref mut history_index,
                } = app.state
                {
                    *history_index = Some(idx);
                    buffer.clone_from(&app.command_history[idx]);
                    app.completer.update(buffer);
                }
            }
            true
        }
        KeyCode::Down => {
            if let FocusState::Command {
                ref mut buffer,
                ref mut history_index,
            } = app.state
            {
                if let Some(idx) = *history_index {
                    if idx + 1 < app.command_history.len() {
                        *history_index = Some(idx + 1);
                        buffer.clone_from(&app.command_history[idx + 1]);
                    } else {
                        *history_index = None;
                        buffer.clear();
                    }
                }
                app.completer.update(buffer);
            }
            true
        }
        KeyCode::Backspace => {
            if let FocusState::Command { ref mut buffer, .. } = app.state {
                if buffer.is_empty() {
                    app.state = FocusState::Tree;
                    app.completer.matches.clear();
                } else {
                    buffer.pop();
                    app.completer.update(buffer);
                    app.completer.reset_selection();
                }
            }
            true
        }
        KeyCode::Char(c) => {
            if let FocusState::Command {
                ref mut buffer,
                ref mut history_index,
            } = app.state
            {
                *history_index = None;
                buffer.push(c);
                app.completer.update(buffer);
                app.completer.reset_selection();
            }
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
