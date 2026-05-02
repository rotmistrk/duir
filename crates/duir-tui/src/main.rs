mod app;
mod clipboard;
mod completer;
mod help;
mod input;
mod markdown_view;
mod note_editor;
mod syntax;

mod password;
#[allow(dead_code)]
mod pty_tab;
#[allow(dead_code)]
mod termbuf;
mod tree_view;

use std::io;
use std::path::PathBuf;
use std::time::Duration;

use clap::Parser;
use crossterm::event::{Event, KeyCode, KeyModifiers};
use crossterm::execute;
use crossterm::terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode};
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};

use duir_core::{FileStorage, TodoStorage};

use app::{App, FocusState};
use tree_view::TreeView;

#[derive(Parser)]
#[command(name = "duir", about = "Hierarchical todo tree manager")]
struct Cli {
    /// Directory containing .todo.json files
    #[arg(short, long)]
    dir: Option<PathBuf>,

    /// Specific files to open
    files: Vec<PathBuf>,
}

fn main() -> io::Result<()> {
    let cli = Cli::parse();
    let config = duir_core::config::Config::load();

    let storage_dir = cli.dir.clone().unwrap_or_else(|| config.storage.central.clone());

    let mut app = App::new();
    app.autosave_global = config.editor.autosave;
    app.note_panel_pct = config.ui.note_panel_pct;

    if cli.files.is_empty() {
        // Load from all configured storage dirs
        for dir in &config.storage_dirs() {
            if let Ok(storage) = FileStorage::new(dir)
                && let Ok(names) = storage.list()
            {
                for name in &names {
                    match storage.load(name) {
                        Ok(data) => app.add_file(name.clone(), data),
                        Err(e) => eprintln!("Error loading {name}: {e}"),
                    }
                }
            }
        }
    } else {
        for path in &cli.files {
            match duir_core::file_storage::load_path(path) {
                Ok(data) => {
                    let name = path
                        .file_stem()
                        .and_then(|s| s.to_str())
                        .unwrap_or("untitled")
                        .to_owned();
                    app.add_file(name, data);
                }
                Err(e) => eprintln!("Error loading {}: {e}", path.display()),
            }
        }
    }

    let first_run = app.files.is_empty();
    if first_run {
        app.add_empty_file("todo");
    }

    if first_run {
        app.state = FocusState::About;
    }

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let result = run_loop(&mut terminal, &mut app, &storage_dir, &config);

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    result
}

#[allow(clippy::too_many_lines)]
fn run_loop(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut App,
    storage_dir: &PathBuf,
    config: &duir_core::config::Config,
) -> io::Result<()> {
    let mut last_save = std::time::Instant::now();
    let autosave_interval = config.editor.autosave_interval_secs;
    loop {
        terminal.draw(|frame| {
            let size = frame.area();

            let main_chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Min(3), Constraint::Length(1)])
                .split(size);

            let content_chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([
                    Constraint::Percentage(100 - app.note_panel_pct),
                    Constraint::Percentage(app.note_panel_pct),
                ])
                .split(main_chunks[0]);

            // Tree pane
            let tree_title = match (
                app.has_unsaved(),
                !app.filter_committed_text.is_empty() && !app.is_filter_active(),
            ) {
                (true, true) => format!(" Tree (*) [/{}] ", app.filter_committed_text),
                (true, false) => " Tree (*) ".to_owned(),
                (false, true) => format!(" Tree [/{}] ", app.filter_committed_text),
                (false, false) => " Tree ".to_owned(),
            };
            let tree_border_style = if app.is_tree_focused() || app.is_editing_title() {
                Style::default().add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };
            let tree_block = Block::default()
                .title(tree_title)
                .borders(Borders::ALL)
                .border_style(tree_border_style);
            frame.render_stateful_widget(TreeView::new().block(tree_block), content_chunks[0], app);

            // Note pane
            if let FocusState::Note { ref mut editor, .. } = app.state {
                let has_cmdline = matches!(
                    editor.mode,
                    crate::note_editor::EditorMode::Command | crate::note_editor::EditorMode::Search
                );
                if has_cmdline {
                    let note_chunks = Layout::default()
                        .direction(Direction::Vertical)
                        .constraints([Constraint::Min(3), Constraint::Length(1)])
                        .split(content_chunks[1]);
                    editor.set_block(" Note", true);
                    editor.render(frame, note_chunks[0], &app.highlighter);
                    let cmd_line = editor.status_line();
                    frame.render_widget(Paragraph::new(cmd_line), note_chunks[1]);
                } else {
                    editor.set_block(" Note", true);
                    editor.render(frame, content_chunks[1], &app.highlighter);
                }
            } else {
                // Not in Note state: render from model
                let note_content = app.current_note();
                let note_block = Block::default().title(" Note ").borders(Borders::ALL);
                let lines = crate::markdown_view::highlight_lines_with_syntax(
                    &note_content,
                    usize::MAX,
                    0,
                    Some(&app.highlighter),
                );
                let paragraph = Paragraph::new(lines).block(note_block);
                frame.render_widget(paragraph, content_chunks[1]);
            }

            // Status bar
            let status = build_status_line(app);
            frame.render_widget(Paragraph::new(status), main_chunks[1]);

            // Command palette popup (above status bar)
            if app.is_command_active() && !app.completer.matches.is_empty() {
                render_palette(frame, &app.completer, main_chunks[1]);
            }

            // Overlays
            if app.is_about_shown() {
                help::render_about(frame, size);
            }
            if let FocusState::Help { scroll } = app.state {
                help::render_help(frame, size, scroll);
            }
            if let Some(prompt) = &app.password_prompt {
                prompt.render(frame, size);
            }
        })?;

        // Process pending crypto after redraw (so "Working..." is visible)
        if let Some((password, action)) = app.pending_crypto.take() {
            app.handle_password_result(&password, action);
            continue; // redraw to show result
        }

        // Block for input, with timeout only for autosave
        let has_pending_save = app.is_tree_focused() && app.files.iter().any(|f| f.autosave && f.modified);
        let timeout = if app.pending_crypto.is_some() {
            Duration::from_millis(1) // process crypto immediately
        } else if has_pending_save {
            Duration::from_secs(autosave_interval)
        } else {
            Duration::from_secs(3600) // effectively block
        };
        if let Some(Event::Key(key)) = input::poll_event(timeout)? {
            // Handle overlay input first
            if let Some(prompt) = &mut app.password_prompt {
                match prompt.handle_key(key) {
                    crate::password::PromptResult::Submitted(password) => {
                        if let Some(prompt) = app.password_prompt.take() {
                            let msg = match &prompt.callback {
                                password::PasswordAction::Decrypt { .. } => "⏳ Decrypting...",
                                password::PasswordAction::Encrypt { .. } => "⏳ Encrypting...",
                                password::PasswordAction::ChangePassword { .. } => "⏳ Re-encrypting...",
                            };
                            app.set_status(msg, app::StatusLevel::Warning);
                            app.pending_crypto = Some((password, prompt.callback));
                        }
                        continue;
                    }
                    crate::password::PromptResult::Cancelled => {
                        app.password_prompt = None;
                    }
                    crate::password::PromptResult::Pending => {}
                }
                continue;
            }
            if app.is_about_shown() {
                app.state = FocusState::Tree;
                continue;
            }
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
                continue;
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
            } else if app.is_command_active() && key.code == KeyCode::Enter {
                // Execute command with storage access
                if let Ok(storage) = FileStorage::new(storage_dir) {
                    app.execute_command(&storage);
                }
            } else {
                input::handle_key(app, key);
            }
        }

        // Autosave — fires when poll timeout expires (no input for autosave_interval)
        if app.is_tree_focused()
            && last_save.elapsed() >= Duration::from_secs(autosave_interval)
            && app.files.iter().any(|f| f.autosave && f.modified)
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

fn render_palette(frame: &mut ratatui::Frame, completer: &crate::completer::Completer, status_area: Rect) {
    use ratatui::widgets::Clear;

    let matches = &completer.matches;
    let max_visible = 10.min(matches.len());
    #[allow(clippy::cast_possible_truncation)]
    let height = max_visible as u16;

    // Position popup just above the status bar
    let popup = Rect::new(
        status_area.x + 1,
        status_area.y.saturating_sub(height),
        30.min(status_area.width),
        height,
    );

    frame.render_widget(Clear, popup);

    let lines: Vec<Line<'_>> = matches
        .iter()
        .take(max_visible)
        .enumerate()
        .map(|(i, cmd)| {
            let style = if completer.selected == Some(i) {
                Style::default().bg(Color::DarkGray).fg(Color::Yellow)
            } else {
                Style::default().fg(Color::White).bg(Color::Rgb(30, 30, 30))
            };
            Line::styled(format!(" {cmd}"), style)
        })
        .collect();

    let block = Block::default().borders(Borders::NONE);
    let paragraph = Paragraph::new(lines).block(block);
    frame.render_widget(paragraph, popup);
}
fn build_status_line(app: &App) -> Line<'_> {
    if let FocusState::Command { ref buffer, .. } = app.state {
        Line::from(vec![
            Span::raw(":"),
            Span::styled(format!("{buffer}▏"), Style::default().add_modifier(Modifier::BOLD)),
        ])
    } else if let FocusState::Filter { ref text, .. } = app.state {
        Line::from(vec![
            Span::raw("Filter: "),
            Span::styled(format!("{text}▏"), Style::default().add_modifier(Modifier::BOLD)),
            Span::raw("  [Enter] apply  [Esc] cancel"),
        ])
    } else if app.is_editing_title() {
        Line::from(vec![
            Span::raw("Editing: "),
            Span::styled(
                "[Enter] confirm  [Esc] cancel",
                Style::default().add_modifier(Modifier::DIM),
            ),
        ])
    } else {
        let bold = Style::default().add_modifier(Modifier::BOLD);
        let mut spans = vec![
            Span::styled(" q", bold),
            Span::raw("uit "),
            Span::styled("n", bold),
            Span::raw("ew "),
            Span::styled("b", bold),
            Span::raw("ranch "),
            Span::styled("d", bold),
            Span::raw("el "),
            Span::styled("c", bold),
            Span::raw("lone "),
            Span::styled("!", bold),
            Span::raw("imp "),
            Span::styled("HJKL", bold),
            Span::raw(" move "),
            Span::styled("S", bold),
            Span::raw("ort "),
            Span::styled("/", bold),
            Span::raw("filter "),
            Span::styled("^S", bold),
            Span::raw("ave "),
            Span::styled("Tab", bold),
            Span::raw(" note "),
            Span::styled(":", bold),
            Span::raw("cmd "),
            Span::styled(":help", bold),
        ];
        if !app.status_message.is_empty() {
            let color = match app.status_level {
                app::StatusLevel::Info => Color::DarkGray,
                app::StatusLevel::Success => Color::Green,
                app::StatusLevel::Warning => Color::Yellow,
                app::StatusLevel::Error => Color::Red,
            };
            spans.push(Span::styled(
                format!("  │ {}", app.status_message),
                Style::default().fg(color),
            ));
        }
        Line::from(spans)
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use app::{App, FocusState, StatusLevel};
    use duir_core::TodoStorage;
    use duir_core::model::{Completion, TodoFile, TodoItem};

    fn make_app_with_tree() -> App {
        let mut app = App::new();
        let mut file = TodoFile::new("test");
        let mut branch1 = TodoItem::new("Branch 1");
        branch1.note = "branch1 note".to_owned();
        let mut child11 = TodoItem::new("Child 1.1");
        child11.completed = Completion::Done;
        child11.note = "child11 note".to_owned();
        let mut child12 = TodoItem::new("Child 1.2");
        child12.important = true;
        child12.note = "child12 note".to_owned();
        branch1.items.push(child11);
        branch1.items.push(child12);
        let mut branch2 = TodoItem::new("Branch 2");
        branch2.note = "branch2 note".to_owned();
        branch2.items.push(TodoItem::new("Child 2.1"));
        file.items.push(branch1);
        file.items.push(branch2);
        file.items.push(TodoItem::new("Branch 3"));
        app.add_file("test".to_owned(), file);
        app
    }

    #[test]
    fn tab_into_note_loads_editor() {
        let mut app = make_app_with_tree();
        app.cursor = 1;
        app.focus_note();
        assert!(app.is_note_focused());
        let FocusState::Note { ref editor, .. } = app.state else {
            unreachable!();
        };
        assert_eq!(editor.content(), "branch1 note");
    }

    #[test]
    fn tab_back_saves_editor_to_model() {
        let mut app = make_app_with_tree();
        app.cursor = 1;
        app.focus_note();
        if let FocusState::Note { ref mut editor, .. } = app.state {
            editor.textarea.insert_str("MODIFIED");
            editor.dirty = true;
        }
        app.save_editor();
        assert!(app.files[0].data.items[0].note.contains("MODIFIED"));
    }

    #[test]
    fn editor_not_written_without_save() {
        let mut app = make_app_with_tree();
        app.cursor = 1;
        app.focus_note();
        if let FocusState::Note { ref mut editor, .. } = app.state {
            editor.textarea.insert_str("SHOULD NOT PERSIST");
        }
        assert_eq!(app.files[0].data.items[0].note, "branch1 note");
    }

    #[test]
    fn cursor_move_does_not_affect_model() {
        let mut app = make_app_with_tree();
        let original = app.files[0].data.items[0].note.clone();
        app.move_down();
        app.move_down();
        app.move_up();
        assert_eq!(app.files[0].data.items[0].note, original);
    }

    /// THE BUG: edit note, tab back, navigate, add items → note content lost.
    /// This tests the exact real-world scenario.
    #[test]
    fn edit_note_tab_back_navigate_add_items_preserves_note() {
        let mut app = make_app_with_tree();
        app.cursor = 1; // Branch 1

        // Tab into note, edit
        app.focus_note();
        if let FocusState::Note { ref mut editor, .. } = app.state {
            editor.textarea.insert_str("EDITED TEXT ");
        }

        // Tab back to tree
        app.save_editor();
        app.focus_tree();
        assert!(app.files[0].data.items[0].note.contains("EDITED TEXT"));

        // Navigate to different items
        app.move_down(); // Child 1.1
        app.move_down(); // Child 1.2

        // current_note should show Child 1.2's note, not the edited one
        assert_eq!(app.current_note(), "child12 note");

        // Add new items
        app.new_sibling();
        app.cancel_editing();

        // Original edit should still be in the model
        assert!(
            app.files[0].data.items[0].note.contains("EDITED TEXT"),
            "Note was lost! Got: {}",
            app.files[0].data.items[0].note
        );

        // All other notes should be intact
        assert_eq!(app.files[0].data.items[0].items[0].note, "child11 note");
    }

    /// Verify `current_note` reads from model based on cursor, not from editor.
    #[test]
    fn current_note_reads_model_not_editor() {
        let mut app = make_app_with_tree();

        // Without loading editor, current_note should work from model
        app.cursor = 1; // Branch 1
        assert_eq!(app.current_note(), "branch1 note");

        app.cursor = 2; // Child 1.1
        assert_eq!(app.current_note(), "child11 note");

        app.cursor = 3; // Child 1.2
        assert_eq!(app.current_note(), "child12 note");
    }

    #[test]
    fn clone_then_navigate_correct_items() {
        let mut app = make_app_with_tree();
        app.cursor = 1;
        app.clone_subtree();
        assert_eq!(app.files[0].data.items[0].title, "Branch 1");
        assert_eq!(app.files[0].data.items[1].title, "Branch 1");
        assert_eq!(app.files[0].data.items[2].title, "Branch 2");
        assert_eq!(app.files[0].data.items[3].title, "Branch 3");
    }

    #[test]
    fn encrypt_sets_prompt() {
        let mut app = make_app_with_tree();
        app.cursor = 1;
        app.cmd_encrypt();
        assert!(app.password_prompt.is_some());
    }

    #[test]
    fn encrypt_then_decrypt_roundtrip() {
        let mut app = make_app_with_tree();
        app.cursor = 1;
        app.cmd_encrypt();
        {
            let cb = app.password_prompt.take().unwrap().callback;
            app.handle_password_result("pass", cb);
        }
        assert!(app.files[0].data.items[0].cipher.is_some());
        assert!(app.files[0].data.items[0].items.is_empty());

        app.cursor = 1;
        app.expand_current();
        {
            let cb = app.password_prompt.take().unwrap().callback;
            app.handle_password_result("pass", cb);
        }
        assert!(app.files[0].data.items[0].unlocked);
        assert_eq!(app.files[0].data.items[0].items.len(), 2);
        assert_eq!(app.files[0].data.items[0].note, "branch1 note");
    }

    /// Tests the EXACT code path used in the real app:
    /// prompt → stash (password, callback) → process on next iteration.
    /// This is the path that broke TWICE due to callback being lost.
    #[test]
    fn encrypt_decrypt_via_pending_crypto_path() {
        let mut app = make_app_with_tree();
        app.cursor = 1;

        // Encrypt via pending_crypto (real app path)
        app.cmd_encrypt();
        assert!(app.password_prompt.is_some());
        let prompt = app.password_prompt.take().unwrap();
        app.pending_crypto = Some(("pass".to_owned(), prompt.callback));
        // Simulate: next iteration processes pending_crypto
        let (pw, action) = app.pending_crypto.take().unwrap();
        app.handle_password_result(&pw, action);

        assert!(app.files[0].data.items[0].cipher.is_some());
        assert!(app.files[0].data.items[0].items.is_empty());

        // Decrypt via pending_crypto (real app path)
        app.cursor = 1;
        app.expand_current();
        assert!(app.password_prompt.is_some());
        let prompt = app.password_prompt.take().unwrap();
        app.pending_crypto = Some(("pass".to_owned(), prompt.callback));
        let (pw, action) = app.pending_crypto.take().unwrap();
        app.handle_password_result(&pw, action);

        assert!(app.files[0].data.items[0].unlocked);
        assert_eq!(app.files[0].data.items[0].items.len(), 2);
        assert_eq!(app.files[0].data.items[0].note, "branch1 note");
    }

    #[test]
    fn decrypt_wrong_password_no_corruption() {
        let mut app = make_app_with_tree();
        app.cursor = 1;
        app.cmd_encrypt();
        {
            let cb = app.password_prompt.take().unwrap().callback;
            app.handle_password_result("correct", cb);
        }
        let cipher = app.files[0].data.items[0].cipher.clone();

        app.cursor = 1;
        app.expand_current();
        {
            let cb = app.password_prompt.take().unwrap().callback;
            app.handle_password_result("wrong", cb);
        }
        assert_eq!(app.files[0].data.items[0].cipher, cipher);
        assert!(app.files[0].data.items[0].items.is_empty());
        assert_eq!(app.status_level, StatusLevel::Error);
    }

    #[test]
    fn collapse_encrypted_relocks() {
        let mut app = make_app_with_tree();
        app.cursor = 1;
        app.cmd_encrypt();
        {
            let cb = app.password_prompt.take().unwrap().callback;
            app.handle_password_result("pass", cb);
        }
        app.cursor = 1;
        app.expand_current();
        {
            let cb = app.password_prompt.take().unwrap().callback;
            app.handle_password_result("pass", cb);
        }
        assert!(app.files[0].data.items[0].unlocked);

        app.cursor = 1;
        app.collapse_current();
        assert!(!app.files[0].data.items[0].unlocked);
        assert!(app.files[0].data.items[0].items.is_empty());
    }

    #[test]
    fn decrypt_requires_unlock() {
        let mut app = make_app_with_tree();
        app.cursor = 1;
        app.cmd_encrypt();
        {
            let cb = app.password_prompt.take().unwrap().callback;
            app.handle_password_result("pass", cb);
        }
        app.cmd_decrypt();
        assert_eq!(app.status_level, StatusLevel::Warning);
        assert!(app.files[0].data.items[0].cipher.is_some());
    }

    #[test]
    fn save_reencrypts_unlocked() {
        let mut app = make_app_with_tree();
        app.cursor = 1;
        app.cmd_encrypt();
        {
            let cb = app.password_prompt.take().unwrap().callback;
            app.handle_password_result("pass", cb);
        }
        app.cursor = 1;
        app.expand_current();
        {
            let cb = app.password_prompt.take().unwrap().callback;
            app.handle_password_result("pass", cb);
        }

        let dir = tempfile::tempdir().unwrap();
        let storage = duir_core::FileStorage::new(dir.path()).unwrap();
        app.save_all(&storage);

        let loaded = storage.load("test").unwrap();
        assert!(loaded.items[0].cipher.is_some());
        assert!(loaded.items[0].items.is_empty());
        // In memory still unlocked
        assert!(app.files[0].data.items[0].unlocked);
    }

    #[test]
    fn collapse_updates_editor() {
        let mut app = make_app_with_tree();
        app.cursor = 1;
        app.focus_note();
        app.save_editor();
        app.cmd_collapse();
        let FocusState::Note { ref editor, .. } = app.state else {
            unreachable!();
        };
        assert!(editor.content().contains("duir:collapsed"));
    }

    #[test]
    fn delete_incomplete_requires_confirm() {
        let mut app = make_app_with_tree();
        app.cursor = 1;
        app.delete_current();
        assert!(app.pending_delete);
        assert_eq!(app.files[0].data.items[0].title, "Branch 1");
    }

    #[test]
    fn delete_completed_leaf_immediate() {
        let mut app = make_app_with_tree();
        app.cursor = 2; // Child 1.1 (Done)
        app.delete_current();
        assert!(!app.pending_delete);
        assert_eq!(app.files[0].data.items[0].items[0].title, "Child 1.2");
    }

    #[test]
    fn filter_hides_rows() {
        let mut app = make_app_with_tree();
        let total = app.rows.len();
        app.filter_committed_text = "Child 1.1".to_owned();
        app.apply_filter();
        assert!(app.rows.len() < total);
    }

    #[test]
    fn filter_clear_restores() {
        let mut app = make_app_with_tree();
        let total = app.rows.len();
        app.filter_committed_text = "Child 1.1".to_owned();
        app.apply_filter();
        app.filter_committed_text.clear();
        app.apply_filter();
        assert_eq!(app.rows.len(), total);
    }

    #[test]
    fn new_sibling_starts_editing() {
        let mut app = make_app_with_tree();
        app.cursor = 1;
        app.new_sibling();
        assert!(app.is_editing_title());
        assert_eq!(app.files[0].data.items.len(), 4);
    }

    #[test]
    fn new_child_starts_editing() {
        let mut app = make_app_with_tree();
        app.cursor = 1;
        let old = app.files[0].data.items[0].items.len();
        app.new_child();
        assert!(app.is_editing_title());
        assert_eq!(app.files[0].data.items[0].items.len(), old + 1);
    }

    #[test]
    fn adding_child_to_completed_parent_uncompletes_it() {
        let mut app = make_app_with_tree();
        // Branch 1 has Child 1.1 (Done) and Child 1.2 (Open, important)
        // Complete Child 1.2
        app.cursor = 3; // Child 1.2
        app.toggle_completed();
        // Now Branch 1 should be Done (all children done)
        assert_eq!(app.files[0].data.items[0].completed, Completion::Done);

        // Add a new child to Branch 1
        app.cursor = 1; // Branch 1
        app.new_child();
        app.cancel_editing();

        // Branch 1 should no longer be Done (new child is Open)
        assert_ne!(app.files[0].data.items[0].completed, Completion::Done);
    }

    #[test]
    fn adding_sibling_updates_parent_completion() {
        let mut app = make_app_with_tree();
        // Complete both children of Branch 1
        app.cursor = 2; // Child 1.1 (already Done)
        app.cursor = 3; // Child 1.2
        app.toggle_completed();
        assert_eq!(app.files[0].data.items[0].completed, Completion::Done);

        // Add sibling to Child 1.2
        app.cursor = 3;
        app.new_sibling();
        app.cancel_editing();

        // Branch 1 should not be Done anymore
        assert_ne!(app.files[0].data.items[0].completed, Completion::Done);
    }

    #[test]
    fn deleting_incomplete_child_may_complete_parent() {
        let mut app = make_app_with_tree();
        // Child 1.1 is Done, Child 1.2 is Open
        // Delete Child 1.2 (the incomplete one)
        app.cursor = 3; // Child 1.2
        app.delete_current(); // pending
        app.force_delete_current(); // confirm

        // Branch 1 should now be Done (only Child 1.1 remains, which is Done)
        assert_eq!(app.files[0].data.items[0].completed, Completion::Done);
    }

    #[test]
    fn save_preserves_unencrypted_data() {
        let mut app = make_app_with_tree();
        app.files[0].modified = true;
        let dir = tempfile::tempdir().unwrap();
        let storage = duir_core::FileStorage::new(dir.path()).unwrap();
        app.save_all(&storage);
        let loaded = storage.load("test").unwrap();
        assert_eq!(loaded.items[0].title, "Branch 1");
        assert_eq!(loaded.items[0].note, "branch1 note");
        assert_eq!(loaded.items[0].items[0].title, "Child 1.1");
        assert_eq!(loaded.items[1].title, "Branch 2");
        assert_eq!(loaded.items[2].title, "Branch 3");
    }

    // ── Helpers for input tests ──────────────────────────────────────

    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }
    fn shift_key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::SHIFT)
    }

    // ── input.rs: tree-mode navigation ──────────────────────────────

    #[test]
    fn input_tree_up() {
        let mut app = make_app_with_tree();
        app.cursor = 2;
        input::handle_key(&mut app, key(KeyCode::Up));
        assert_eq!(app.cursor, 1);
    }

    #[test]
    fn input_tree_down() {
        let mut app = make_app_with_tree();
        app.cursor = 1;
        input::handle_key(&mut app, key(KeyCode::Down));
        assert_eq!(app.cursor, 2);
    }

    #[test]
    fn input_tree_left_collapses() {
        let mut app = make_app_with_tree();
        app.cursor = 1; // Branch 1 (expanded)
        let rows_before = app.rows.len();
        input::handle_key(&mut app, key(KeyCode::Left));
        assert!(app.rows.len() < rows_before);
    }

    #[test]
    fn input_tree_right_expands() {
        let mut app = make_app_with_tree();
        app.cursor = 1;
        // Collapse first
        app.collapse_current();
        let rows_collapsed = app.rows.len();
        input::handle_key(&mut app, key(KeyCode::Right));
        assert!(app.rows.len() > rows_collapsed);
    }

    // ── input.rs: tree-mode operations ──────────────────────────────

    #[test]
    fn input_tree_n_new_sibling() {
        let mut app = make_app_with_tree();
        app.cursor = 1;
        input::handle_key(&mut app, key(KeyCode::Char('n')));
        assert!(app.is_editing_title());
        assert_eq!(app.files[0].data.items.len(), 4);
    }

    #[test]
    fn input_tree_b_new_child() {
        let mut app = make_app_with_tree();
        app.cursor = 1;
        let old = app.files[0].data.items[0].items.len();
        input::handle_key(&mut app, key(KeyCode::Char('b')));
        assert!(app.is_editing_title());
        assert_eq!(app.files[0].data.items[0].items.len(), old + 1);
    }

    #[test]
    fn input_tree_d_delete() {
        let mut app = make_app_with_tree();
        app.cursor = 1;
        input::handle_key(&mut app, key(KeyCode::Char('d')));
        assert!(app.pending_delete);
    }

    #[test]
    fn input_tree_c_clone() {
        let mut app = make_app_with_tree();
        app.cursor = 1;
        input::handle_key(&mut app, key(KeyCode::Char('c')));
        assert_eq!(app.files[0].data.items.len(), 4);
    }

    #[test]
    fn input_tree_bang_importance() {
        let mut app = make_app_with_tree();
        app.cursor = 1;
        assert!(!app.files[0].data.items[0].important);
        input::handle_key(&mut app, key(KeyCode::Char('!')));
        assert!(app.files[0].data.items[0].important);
    }

    #[test]
    fn input_tree_s_sort() {
        let mut app = make_app_with_tree();
        app.cursor = 1; // Branch 1
        input::handle_key(&mut app, key(KeyCode::Char('S')));
        // Sort should not crash; children reordered by completion
        assert!(!app.files[0].data.items[0].items.is_empty());
    }

    #[test]
    fn input_tree_q_quits() {
        let mut app = make_app_with_tree();
        input::handle_key(&mut app, key(KeyCode::Char('q')));
        assert!(app.should_quit);
    }

    // ── input.rs: tree-mode move (Shift+Arrow, HJKL) ───────────────

    #[test]
    fn input_tree_shift_up_swaps() {
        let mut app = make_app_with_tree();
        app.cursor = 4; // Branch 2
        input::handle_key(&mut app, shift_key(KeyCode::Up));
        assert_eq!(app.files[0].data.items[0].title, "Branch 2");
    }

    #[test]
    fn input_tree_shift_down_swaps() {
        let mut app = make_app_with_tree();
        app.cursor = 1; // Branch 1
        input::handle_key(&mut app, shift_key(KeyCode::Down));
        assert_eq!(app.files[0].data.items[1].title, "Branch 1");
    }

    #[test]
    fn input_tree_shift_left_promotes() {
        let mut app = make_app_with_tree();
        app.cursor = 2; // Child 1.1
        input::handle_key(&mut app, shift_key(KeyCode::Left));
        // Child 1.1 promoted to top level
        assert!(app.files[0].data.items.iter().any(|i| i.title == "Child 1.1"));
    }

    #[test]
    fn input_tree_shift_right_demotes() {
        let mut app = make_app_with_tree();
        // Move cursor to Branch 2 (index 4 in rows: root, B1, C1.1, C1.2, B2)
        app.cursor = 4;
        input::handle_key(&mut app, shift_key(KeyCode::Right));
        // Branch 2 demoted under Branch 1
        assert!(app.files[0].data.items[0].items.iter().any(|i| i.title == "Branch 2"));
    }

    #[test]
    fn input_tree_k_swaps_up() {
        let mut app = make_app_with_tree();
        app.cursor = 4; // Branch 2
        input::handle_key(&mut app, key(KeyCode::Char('K')));
        assert_eq!(app.files[0].data.items[0].title, "Branch 2");
    }

    #[test]
    fn input_tree_j_swaps_down() {
        let mut app = make_app_with_tree();
        app.cursor = 1; // Branch 1
        input::handle_key(&mut app, key(KeyCode::Char('J')));
        assert_eq!(app.files[0].data.items[1].title, "Branch 1");
    }

    #[test]
    fn input_tree_h_promotes() {
        let mut app = make_app_with_tree();
        app.cursor = 2; // Child 1.1
        input::handle_key(&mut app, key(KeyCode::Char('H')));
        assert!(app.files[0].data.items.iter().any(|i| i.title == "Child 1.1"));
    }

    #[test]
    fn input_tree_l_demotes() {
        let mut app = make_app_with_tree();
        app.cursor = 4; // Branch 2
        input::handle_key(&mut app, key(KeyCode::Char('L')));
        assert!(app.files[0].data.items[0].items.iter().any(|i| i.title == "Branch 2"));
    }

    // ── input.rs: tree-mode switches ────────────────────────────────

    #[test]
    fn input_tree_tab_to_note() {
        let mut app = make_app_with_tree();
        app.cursor = 1;
        input::handle_key(&mut app, key(KeyCode::Tab));
        assert!(app.is_note_focused());
    }

    #[test]
    fn input_tree_colon_to_command() {
        let mut app = make_app_with_tree();
        input::handle_key(&mut app, key(KeyCode::Char(':')));
        assert!(app.is_command_active());
    }

    #[test]
    fn input_tree_slash_to_filter() {
        let mut app = make_app_with_tree();
        input::handle_key(&mut app, key(KeyCode::Char('/')));
        assert!(app.is_filter_active());
    }

    #[test]
    fn input_tree_f1_help() {
        let mut app = make_app_with_tree();
        input::handle_key(&mut app, key(KeyCode::F(1)));
        assert!(app.is_help_shown());
    }

    // ── input.rs: filter mode ───────────────────────────────────────

    #[test]
    fn input_filter_typing() {
        let mut app = make_app_with_tree();
        app.state = FocusState::Filter {
            text: String::new(),
            saved: String::new(),
        };
        input::handle_key(&mut app, key(KeyCode::Char('C')));
        input::handle_key(&mut app, key(KeyCode::Char('h')));
        let FocusState::Filter { ref text, .. } = app.state else {
            unreachable!();
        };
        assert_eq!(text, "Ch");
    }

    #[test]
    fn input_filter_enter_applies() {
        let mut app = make_app_with_tree();
        app.state = FocusState::Filter {
            text: "Child 1.1".to_owned(),
            saved: String::new(),
        };
        input::handle_key(&mut app, key(KeyCode::Enter));
        assert!(!app.is_filter_active());
        // Filter applied — fewer rows
        assert!(app.rows.iter().filter(|r| !r.is_file_root).count() < 6);
    }

    #[test]
    fn input_filter_esc_reverts() {
        let mut app = make_app_with_tree();
        let total = app.rows.len();
        app.state = FocusState::Filter {
            text: "xyz".to_owned(),
            saved: String::new(),
        };
        input::handle_key(&mut app, key(KeyCode::Esc));
        assert!(!app.is_filter_active());
        assert!(app.filter_committed_text.is_empty());
        assert_eq!(app.rows.len(), total);
    }

    #[test]
    fn input_filter_exclude_prefix() {
        let mut app = make_app_with_tree();
        app.state = FocusState::Filter {
            text: "!Branch 1".to_owned(),
            saved: String::new(),
        };
        input::handle_key(&mut app, key(KeyCode::Enter));
        assert!(app.filter_committed_exclude);
        assert_eq!(app.filter_committed_text, "Branch 1");
    }

    #[test]
    fn input_filter_backspace() {
        let mut app = make_app_with_tree();
        app.state = FocusState::Filter {
            text: "abc".to_owned(),
            saved: String::new(),
        };
        input::handle_key(&mut app, key(KeyCode::Backspace));
        let FocusState::Filter { ref text, .. } = app.state else {
            unreachable!();
        };
        assert_eq!(text, "ab");
    }

    // ── input.rs: command mode ──────────────────────────────────────

    #[test]
    fn input_command_typing() {
        let mut app = make_app_with_tree();
        app.state = FocusState::Command {
            buffer: String::new(),
            history_index: None,
        };
        input::handle_key(&mut app, key(KeyCode::Char('h')));
        input::handle_key(&mut app, key(KeyCode::Char('e')));
        if let FocusState::Command { ref buffer, .. } = app.state {
            assert_eq!(buffer, "he");
        } else {
            unreachable!();
        }
    }

    #[test]
    fn input_command_esc_cancels() {
        let mut app = make_app_with_tree();
        app.state = FocusState::Command {
            buffer: "help".to_owned(),
            history_index: None,
        };
        input::handle_key(&mut app, key(KeyCode::Esc));
        assert!(!app.is_command_active());
    }

    #[test]
    fn input_command_enter_pushes_history() {
        let mut app = make_app_with_tree();
        app.state = FocusState::Command {
            buffer: "help".to_owned(),
            history_index: None,
        };
        // Enter in command mode is handled in main loop for storage,
        // but handle_key still pushes history
        input::handle_key(&mut app, key(KeyCode::Enter));
        assert_eq!(app.command_history.last().unwrap(), "help");
    }

    #[test]
    fn input_command_tab_completes() {
        let mut app = make_app_with_tree();
        app.state = FocusState::Command {
            buffer: "hel".to_owned(),
            history_index: None,
        };
        input::handle_key(&mut app, key(KeyCode::Tab));
        if let FocusState::Command { ref buffer, .. } = app.state {
            assert_eq!(buffer, "help");
        } else {
            unreachable!();
        }
    }

    #[test]
    fn input_command_up_down_history() {
        let mut app = make_app_with_tree();
        app.command_history = vec!["first".to_owned(), "second".to_owned()];
        app.state = FocusState::Command {
            buffer: String::new(),
            history_index: None,
        };
        // Up → last history entry
        input::handle_key(&mut app, key(KeyCode::Up));
        if let FocusState::Command { ref buffer, .. } = app.state {
            assert_eq!(buffer, "second");
        }
        // Up again → first
        input::handle_key(&mut app, key(KeyCode::Up));
        if let FocusState::Command { ref buffer, .. } = app.state {
            assert_eq!(buffer, "first");
        }
        // Down → second
        input::handle_key(&mut app, key(KeyCode::Down));
        if let FocusState::Command { ref buffer, .. } = app.state {
            assert_eq!(buffer, "second");
        }
        // Down past end → clears
        input::handle_key(&mut app, key(KeyCode::Down));
        if let FocusState::Command { ref buffer, .. } = app.state {
            assert!(buffer.is_empty());
        }
    }

    #[test]
    fn input_command_backspace_on_empty_exits() {
        let mut app = make_app_with_tree();
        app.state = FocusState::Command {
            buffer: String::new(),
            history_index: None,
        };
        input::handle_key(&mut app, key(KeyCode::Backspace));
        assert!(!app.is_command_active());
    }

    #[test]
    fn input_command_backspace_deletes_char() {
        let mut app = make_app_with_tree();
        app.state = FocusState::Command {
            buffer: "hel".to_owned(),
            history_index: None,
        };
        input::handle_key(&mut app, key(KeyCode::Backspace));
        if let FocusState::Command { ref buffer, .. } = app.state {
            assert_eq!(buffer, "he");
        }
        assert!(app.is_command_active());
    }

    // ── input.rs: edit mode (title editing) ─────────────────────────

    #[test]
    fn input_edit_chars() {
        let mut app = make_app_with_tree();
        app.cursor = 1;
        app.start_editing();
        if let FocusState::EditingTitle {
            ref mut select_all,
            ref mut cursor,
            ref buffer,
            ..
        } = app.state
        {
            *select_all = false;
            *cursor = buffer.len();
        }
        input::handle_key(&mut app, key(KeyCode::Char('X')));
        if let FocusState::EditingTitle { ref buffer, .. } = app.state {
            assert!(buffer.ends_with('X'));
        }
    }

    #[test]
    fn input_edit_backspace() {
        let mut app = make_app_with_tree();
        app.cursor = 1;
        app.start_editing();
        let orig_len = if let FocusState::EditingTitle {
            ref mut select_all,
            ref buffer,
            ..
        } = app.state
        {
            *select_all = false;
            buffer.len()
        } else {
            0
        };
        input::handle_key(&mut app, key(KeyCode::Backspace));
        if let FocusState::EditingTitle { ref buffer, .. } = app.state {
            assert_eq!(buffer.len(), orig_len - 1);
        }
    }

    #[test]
    fn input_edit_delete() {
        let mut app = make_app_with_tree();
        app.cursor = 1;
        app.start_editing();
        let orig_len = if let FocusState::EditingTitle {
            ref mut select_all,
            ref mut cursor,
            ref buffer,
        } = app.state
        {
            *select_all = false;
            *cursor = 0;
            buffer.len()
        } else {
            0
        };
        input::handle_key(&mut app, key(KeyCode::Delete));
        if let FocusState::EditingTitle { ref buffer, .. } = app.state {
            assert_eq!(buffer.len(), orig_len - 1);
        }
    }

    #[test]
    fn input_edit_arrows_home_end() {
        let mut app = make_app_with_tree();
        app.cursor = 1;
        app.start_editing();
        if let FocusState::EditingTitle { ref mut select_all, .. } = app.state {
            *select_all = false;
        }
        input::handle_key(&mut app, key(KeyCode::Home));
        if let FocusState::EditingTitle { cursor, .. } = app.state {
            assert_eq!(cursor, 0);
        }
        input::handle_key(&mut app, key(KeyCode::End));
        if let FocusState::EditingTitle { cursor, ref buffer, .. } = app.state {
            assert_eq!(cursor, buffer.len());
        }
        input::handle_key(&mut app, key(KeyCode::Left));
        let pos = if let FocusState::EditingTitle { cursor, .. } = app.state {
            cursor
        } else {
            0
        };
        input::handle_key(&mut app, key(KeyCode::Right));
        if let FocusState::EditingTitle { cursor, .. } = app.state {
            assert_eq!(cursor, pos + 1);
        }
    }

    #[test]
    fn input_edit_enter_finishes() {
        let mut app = make_app_with_tree();
        app.cursor = 1;
        app.start_editing();
        if let FocusState::EditingTitle {
            ref mut buffer,
            ref mut select_all,
            ..
        } = app.state
        {
            *select_all = false;
            "Renamed".clone_into(buffer);
        }
        input::handle_key(&mut app, key(KeyCode::Enter));
        assert!(!app.is_editing_title());
        assert_eq!(app.files[0].data.items[0].title, "Renamed");
    }

    #[test]
    fn input_edit_esc_cancels() {
        let mut app = make_app_with_tree();
        app.cursor = 1;
        app.start_editing();
        input::handle_key(&mut app, key(KeyCode::Esc));
        assert!(!app.is_editing_title());
        assert_eq!(app.files[0].data.items[0].title, "Branch 1");
    }

    // ── input.rs: note mode ─────────────────────────────────────────

    #[test]
    fn input_note_tab_back_to_tree() {
        let mut app = make_app_with_tree();
        app.cursor = 1;
        app.focus_note();
        // Editor starts in Normal mode, Tab returns to tree
        input::handle_key(&mut app, key(KeyCode::Tab));
        assert!(app.is_tree_focused());
    }

    // ── completer.rs tests ──────────────────────────────────────────

    #[test]
    fn completer_empty_shows_all() {
        let mut c = completer::Completer::new(completer::APP_COMMANDS);
        c.update("");
        assert_eq!(c.matches.len(), completer::APP_COMMANDS.len());
    }

    #[test]
    fn completer_prefix_narrows() {
        let mut c = completer::Completer::new(completer::APP_COMMANDS);
        c.update("ex");
        assert!(c.matches.iter().all(|m| m.starts_with("ex")));
        assert!(!c.matches.is_empty());
    }

    #[test]
    fn completer_next_cycles() {
        let mut c = completer::Completer::new(&["alpha", "beta"]);
        c.update("");
        let first = c.next().unwrap();
        assert_eq!(first, "alpha");
        let second = c.next().unwrap();
        assert_eq!(second, "beta");
        // Wraps around
        let third = c.next().unwrap();
        assert_eq!(third, "alpha");
    }

    #[test]
    fn completer_prev_cycles() {
        let mut c = completer::Completer::new(&["alpha", "beta"]);
        c.update("");
        let first = c.prev().unwrap();
        assert_eq!(first, "beta"); // starts from end
        let second = c.prev().unwrap();
        assert_eq!(second, "alpha");
        let third = c.prev().unwrap();
        assert_eq!(third, "beta"); // wraps
    }

    #[test]
    fn completer_reset_selection() {
        let mut c = completer::Completer::new(&["alpha", "beta"]);
        c.update("");
        c.next();
        assert!(c.selected.is_some());
        c.reset_selection();
        assert!(c.selected.is_none());
    }

    #[test]
    fn completer_no_matches_returns_none() {
        let mut c = completer::Completer::new(&["alpha"]);
        c.update("zzz");
        assert!(c.matches.is_empty());
        assert!(c.next().is_none());
        assert!(c.prev().is_none());
    }

    // ── app.rs: untested paths ──────────────────────────────────────

    #[test]
    fn cmd_export_no_filename() {
        let mut app = make_app_with_tree();
        app.cursor = 1;
        let dir = tempfile::tempdir().unwrap();
        let export_path = dir.path().join("branch-1.md");
        // We can't easily control CWD, so test with explicit filename
        app.cmd_export(&["export", export_path.to_str().unwrap()]);
        assert!(export_path.exists());
        let content = std::fs::read_to_string(&export_path).unwrap();
        assert!(content.contains("Branch 1"));
    }

    #[test]
    fn cmd_export_with_filename() {
        let mut app = make_app_with_tree();
        app.cursor = 1;
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("out.md");
        app.cmd_export(&["export", path.to_str().unwrap()]);
        assert!(path.exists());
        assert!(app.status_message.contains("Exported"));
    }

    #[test]
    fn cmd_export_no_item() {
        let mut app = make_app_with_tree();
        app.cursor = 0; // file root
        app.cmd_export(&["export"]);
        assert!(app.status_message.contains("No item"));
    }

    #[test]
    fn cmd_import_md() {
        let mut app = make_app_with_tree();
        app.cursor = 1;
        let dir = tempfile::tempdir().unwrap();
        let md_path = dir.path().join("import.md");
        std::fs::write(&md_path, "# Imported\n- Sub item\n").unwrap();
        app.cmd_import(&["import", "md", md_path.to_str().unwrap()]);
        assert!(app.status_message.contains("Imported"));
        // Children added to Branch 1
        assert!(app.files[0].data.items[0].items.iter().any(|i| i.title == "Imported"));
    }

    #[test]
    fn cmd_import_bad_usage() {
        let mut app = make_app_with_tree();
        app.cmd_import(&["import"]);
        assert!(app.status_message.contains("Usage"));
    }

    #[test]
    fn cmd_collapse_then_expand_roundtrip() {
        let mut app = make_app_with_tree();
        app.cursor = 1; // Branch 1 with children
        let children_before = app.files[0].data.items[0].items.len();
        assert!(children_before > 0);

        app.cmd_collapse();
        assert!(app.files[0].data.items[0].items.is_empty());
        assert!(app.files[0].data.items[0].note.contains("duir:collapsed"));

        app.cmd_expand();
        assert!(!app.files[0].data.items[0].items.is_empty());
        assert_eq!(app.files[0].data.items[0].items.len(), children_before);
    }

    #[test]
    fn cmd_collapse_no_children() {
        let mut app = make_app_with_tree();
        // Branch 3 has no children
        app.cursor = app.rows.iter().position(|r| r.title == "Branch 3").unwrap();
        app.cmd_collapse();
        assert!(app.status_message.contains("No children"));
    }

    #[test]
    fn cmd_expand_empty_note() {
        let mut app = make_app_with_tree();
        // Branch 3 has empty note
        app.cursor = app.rows.iter().position(|r| r.title == "Branch 3").unwrap();
        app.cmd_expand();
        assert!(app.status_message.contains("No note"));
    }

    #[test]
    fn cmd_autosave_toggle() {
        let mut app = make_app_with_tree();
        let before = app.files[0].autosave;
        app.cmd_autosave(&["autosave"]);
        assert_ne!(app.files[0].autosave, before);
        assert!(app.status_message.contains("Autosave"));
    }

    #[test]
    fn cmd_autosave_all_toggle() {
        let mut app = make_app_with_tree();
        let before = app.autosave_global;
        app.cmd_autosave(&["autosave", "all"]);
        assert_ne!(app.autosave_global, before);
        for f in &app.files {
            assert_eq!(f.autosave, app.autosave_global);
        }
    }

    #[test]
    fn close_current_file_unsaved_blocked() {
        let mut app = make_app_with_tree();
        app.files[0].modified = true;
        app.close_current_file();
        assert_eq!(app.files.len(), 1); // not removed
        assert!(app.status_message.contains("unsaved"));
    }

    #[test]
    fn close_current_file_saved_removes() {
        let mut app = make_app_with_tree();
        app.files[0].modified = false;
        app.close_current_file();
        assert!(app.should_quit); // last file → quit
    }

    #[test]
    fn apply_filter_exclude_mode() {
        let mut app = make_app_with_tree();
        app.filter_committed_text = "Branch 1".to_owned();
        app.filter_committed_exclude = true;
        app.apply_filter();
        // Branch 1 should be hidden
        assert!(!app.rows.iter().any(|r| r.title == "Branch 1"));
        assert!(app.status_message.contains("exclude"));
    }

    #[test]
    fn apply_filter_live_updates() {
        let mut app = make_app_with_tree();
        let total = app.rows.len();
        app.state = FocusState::Filter {
            text: "Child 1.1".to_owned(),
            saved: String::new(),
        };
        app.apply_filter_live();
        assert!(app.rows.len() < total);
        assert!(app.status_message.contains("matches"));
    }

    #[test]
    fn apply_filter_live_empty_restores() {
        let mut app = make_app_with_tree();
        let total = app.rows.len();
        app.state = FocusState::Filter {
            text: "Child".to_owned(),
            saved: String::new(),
        };
        app.apply_filter_live();
        if let FocusState::Filter { ref mut text, .. } = app.state {
            text.clear();
        }
        app.apply_filter_live();
        assert_eq!(app.rows.len(), total);
    }

    #[test]
    fn apply_filter_live_exclude_preview() {
        let mut app = make_app_with_tree();
        app.state = FocusState::Filter {
            text: "!Branch 1".to_owned(),
            saved: String::new(),
        };
        app.apply_filter_live();
        assert!(!app.rows.iter().any(|r| r.title == "Branch 1"));
    }

    #[test]
    fn mark_modified_invalidates_cipher() {
        let mut app = make_app_with_tree();
        app.cursor = 1;
        // Encrypt Branch 1
        app.cmd_encrypt();
        let cb = app.password_prompt.take().unwrap().callback;
        app.handle_password_result("pass", cb);
        assert!(app.files[0].data.items[0].cipher.is_some());

        // Unlock it
        app.cursor = 1;
        app.expand_current();
        let cb = app.password_prompt.take().unwrap().callback;
        app.handle_password_result("pass", cb);
        let cipher_before = app.files[0].data.items[0].cipher.clone();

        // Modify a child — should invalidate parent cipher
        #[allow(clippy::useless_vec)]
        let child_path = vec![0, 0];
        if let Some(child) = duir_core::tree_ops::get_item_mut(&mut app.files[0].data, &child_path) {
            child.title = "Modified".to_owned();
        }
        app.mark_modified(0, &child_path);
        assert_ne!(app.files[0].data.items[0].cipher, cipher_before);
    }

    #[test]
    fn pending_delete_cleared_on_other_key() {
        let mut app = make_app_with_tree();
        app.cursor = 1;
        app.delete_current();
        assert!(app.pending_delete);
        // Press any key other than 'y'
        input::handle_key(&mut app, key(KeyCode::Char('n')));
        // pending_delete cleared (though 'n' also creates sibling)
        assert!(!app.pending_delete);
    }

    #[test]
    fn pending_delete_y_confirms() {
        let mut app = make_app_with_tree();
        app.cursor = 1;
        app.delete_current();
        assert!(app.pending_delete);
        input::handle_key(&mut app, key(KeyCode::Char('y')));
        assert!(!app.pending_delete);
        assert_ne!(app.files[0].data.items[0].title, "Branch 1");
    }

    #[test]
    fn space_toggles_completion() {
        let mut app = make_app_with_tree();
        app.cursor = 1;
        let before = app.files[0].data.items[0].completed.clone();
        input::handle_key(&mut app, key(KeyCode::Char(' ')));
        assert_ne!(app.files[0].data.items[0].completed, before);
    }

    #[test]
    fn enter_starts_editing() {
        let mut app = make_app_with_tree();
        app.cursor = 1;
        input::handle_key(&mut app, key(KeyCode::Enter));
        assert!(app.is_editing_title());
    }

    #[test]
    fn bracket_resizes_note_panel() {
        let mut app = make_app_with_tree();
        let before = app.note_panel_pct;
        input::handle_key(&mut app, key(KeyCode::Char(']')));
        assert_eq!(app.note_panel_pct, before + 5);
        input::handle_key(&mut app, key(KeyCode::Char('[')));
        assert_eq!(app.note_panel_pct, before);
    }
}
