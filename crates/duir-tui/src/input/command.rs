use crossterm::event::{KeyCode, KeyEvent};

use crate::app::{App, FocusState};

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
        *buffer = completions
            .get(next_idx)
            .map_or_else(String::new, |c| format!("{cmd_prefix}{c}"));
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
pub(super) fn handle_filter_key(app: &mut App, key: KeyEvent) -> bool {
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
                app.flags.set_filter_committed_exclude(false);
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
                app.flags.set_filter_committed_exclude(true);
                app.filter_committed_text = rest.to_owned();
            } else {
                app.flags.set_filter_committed_exclude(false);
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
pub(super) fn handle_command_key(app: &mut App, key: KeyEvent) -> bool {
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
                    if let Some(entry) = app.command_history.get(idx) {
                        buffer.clone_from(entry);
                    }
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
                        if let Some(entry) = app.command_history.get(idx + 1) {
                            buffer.clone_from(entry);
                        }
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
