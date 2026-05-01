mod app;
mod clipboard;
mod completer;
mod help;
mod input;
mod markdown_view;
mod note_editor;
mod note_view;
mod password;
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

use app::{App, Focus};
use note_view::NoteView;
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

    // Initialize editor for the first item
    app.load_editor();

    if first_run {
        app.show_about = true;
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
            let tree_title = match (app.has_unsaved(), !app.filter_text.is_empty() && !app.filter_active) {
                (true, true) => format!(" Tree (*) [/{}] ", app.filter_text),
                (true, false) => " Tree (*) ".to_owned(),
                (false, true) => format!(" Tree [/{}] ", app.filter_text),
                (false, false) => " Tree ".to_owned(),
            };
            let tree_border_style = if app.focus == Focus::Tree {
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
            let focused = app.focus == Focus::Note;
            if let Some(editor) = &mut app.editor {
                let has_cmdline = matches!(
                    editor.mode,
                    crate::note_editor::EditorMode::Command | crate::note_editor::EditorMode::Search
                );
                if has_cmdline {
                    let note_chunks = Layout::default()
                        .direction(Direction::Vertical)
                        .constraints([Constraint::Min(3), Constraint::Length(1)])
                        .split(content_chunks[1]);
                    editor.set_block(" Note", focused);
                    editor.render(frame, note_chunks[0]);
                    let cmd_line = editor.status_line();
                    frame.render_widget(Paragraph::new(cmd_line), note_chunks[1]);
                } else {
                    editor.set_block(" Note", focused);
                    editor.render(frame, content_chunks[1]);
                }
            } else {
                let note_content = app.current_note();
                let note_border_style = if focused {
                    Style::default().add_modifier(Modifier::BOLD)
                } else {
                    Style::default()
                };
                let note_block = Block::default()
                    .title(" Note ")
                    .borders(Borders::ALL)
                    .border_style(note_border_style);
                let note_widget = NoteView::new(&note_content).block(note_block).scroll(0);
                frame.render_widget(note_widget, content_chunks[1]);
            }

            // Status bar
            let status = build_status_line(app);
            frame.render_widget(Paragraph::new(status), main_chunks[1]);

            // Command palette popup (above status bar)
            if app.command_active && !app.completer.matches.is_empty() {
                render_palette(frame, &app.completer, main_chunks[1]);
            }

            // Overlays
            if app.show_about {
                help::render_about(frame, size);
            }
            if app.show_help {
                help::render_help(frame, size, app.help_scroll);
            }
            if let Some(prompt) = &app.password_prompt {
                prompt.render(frame, size);
            }
        })?;

        // Process pending crypto after redraw (so "Working..." is visible)
        if let Some(password) = app.pending_crypto.take() {
            app.handle_password_result(&password);
        }

        if let Some(Event::Key(key)) = input::poll_event(Duration::from_millis(100))? {
            // Handle overlay input first
            if let Some(prompt) = &mut app.password_prompt {
                match prompt.handle_key(key) {
                    crate::password::PromptResult::Submitted(password) => {
                        app.pending_crypto = Some(password);
                        app.set_status("⏳ Working...", app::StatusLevel::Warning);
                        // Don't clear password_prompt yet — continue to redraw,
                        // then process crypto on next iteration
                        app.password_prompt = None;
                        continue;
                    }
                    crate::password::PromptResult::Cancelled => {
                        app.password_prompt = None;
                    }
                    crate::password::PromptResult::Pending => {}
                }
                continue;
            }
            if app.show_about {
                app.show_about = false;
                continue;
            }
            if app.show_help {
                match key.code {
                    KeyCode::Esc | KeyCode::Char('q') => app.show_help = false,
                    KeyCode::Down | KeyCode::Char('j') => app.help_scroll += 1,
                    KeyCode::Up | KeyCode::Char('k') => {
                        app.help_scroll = app.help_scroll.saturating_sub(1);
                    }
                    KeyCode::PageDown => app.help_scroll += 20,
                    KeyCode::PageUp => app.help_scroll = app.help_scroll.saturating_sub(20),
                    _ => {}
                }
                continue;
            }

            if key.code == KeyCode::Char('s')
                && key.modifiers.contains(KeyModifiers::CONTROL)
                && !app.editing_title
                && !app.filter_active
                && !app.command_active
            {
                if let Ok(storage) = FileStorage::new(storage_dir) {
                    app.save_all(&storage);
                }
            } else if app.command_active && key.code == KeyCode::Enter {
                // Execute command with storage access
                if let Ok(storage) = FileStorage::new(storage_dir) {
                    app.execute_command(&storage);
                }
            } else {
                input::handle_key(app, key);
            }
        }

        // Autosave — debounced, only when tree focused, at most every 5 seconds
        if app.focus == Focus::Tree
            && last_save.elapsed() > Duration::from_secs(autosave_interval)
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
    if app.command_active {
        Line::from(vec![
            Span::raw(":"),
            Span::styled(
                format!("{}▏", app.command_buffer),
                Style::default().add_modifier(Modifier::BOLD),
            ),
        ])
    } else if app.filter_active {
        Line::from(vec![
            Span::raw("Filter: "),
            Span::styled(
                format!("{}▏", app.filter_text),
                Style::default().add_modifier(Modifier::BOLD),
            ),
            Span::raw("  [Enter] apply  [Esc] cancel"),
        ])
    } else if app.editing_title {
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
    use app::{App, Focus, StatusLevel};
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
        app.load_editor();
        app.focus = Focus::Note;
        assert!(app.editor.is_some());
        assert_eq!(app.editor.as_ref().unwrap().content(), "branch1 note");
    }

    #[test]
    fn tab_back_saves_editor_to_model() {
        let mut app = make_app_with_tree();
        app.cursor = 1;
        app.load_editor();
        if let Some(editor) = &mut app.editor {
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
        app.load_editor();
        if let Some(editor) = &mut app.editor {
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
        app.handle_password_result("pass");
        assert!(app.files[0].data.items[0].cipher.is_some());
        assert!(app.files[0].data.items[0].items.is_empty());

        app.cursor = 1;
        app.expand_current();
        app.handle_password_result("pass");
        assert!(app.files[0].data.items[0].unlocked);
        assert_eq!(app.files[0].data.items[0].items.len(), 2);
        assert_eq!(app.files[0].data.items[0].note, "branch1 note");
    }

    #[test]
    fn decrypt_wrong_password_no_corruption() {
        let mut app = make_app_with_tree();
        app.cursor = 1;
        app.cmd_encrypt();
        app.handle_password_result("correct");
        let cipher = app.files[0].data.items[0].cipher.clone();

        app.cursor = 1;
        app.expand_current();
        app.handle_password_result("wrong");
        assert_eq!(app.files[0].data.items[0].cipher, cipher);
        assert!(app.files[0].data.items[0].items.is_empty());
        assert_eq!(app.status_level, StatusLevel::Error);
    }

    #[test]
    fn collapse_encrypted_relocks() {
        let mut app = make_app_with_tree();
        app.cursor = 1;
        app.cmd_encrypt();
        app.handle_password_result("pass");
        app.cursor = 1;
        app.expand_current();
        app.handle_password_result("pass");
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
        app.handle_password_result("pass");
        app.cmd_decrypt();
        assert_eq!(app.status_level, StatusLevel::Warning);
        assert!(app.files[0].data.items[0].cipher.is_some());
    }

    #[test]
    fn save_reencrypts_unlocked() {
        let mut app = make_app_with_tree();
        app.cursor = 1;
        app.cmd_encrypt();
        app.handle_password_result("pass");
        app.cursor = 1;
        app.expand_current();
        app.handle_password_result("pass");

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
        app.load_editor();
        app.save_editor();
        app.cmd_collapse();
        let content = app.editor.as_ref().unwrap().content();
        assert!(content.contains("duir:collapsed"));
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
        app.filter_text = "Child 1.1".to_owned();
        app.apply_filter();
        assert!(app.rows.len() < total);
    }

    #[test]
    fn filter_clear_restores() {
        let mut app = make_app_with_tree();
        let total = app.rows.len();
        app.filter_text = "Child 1.1".to_owned();
        app.apply_filter();
        app.filter_text.clear();
        app.apply_filter();
        assert_eq!(app.rows.len(), total);
    }

    #[test]
    fn new_sibling_starts_editing() {
        let mut app = make_app_with_tree();
        app.cursor = 1;
        app.new_sibling();
        assert!(app.editing_title);
        assert_eq!(app.files[0].data.items.len(), 4);
    }

    #[test]
    fn new_child_starts_editing() {
        let mut app = make_app_with_tree();
        app.cursor = 1;
        let old = app.files[0].data.items[0].items.len();
        app.new_child();
        assert!(app.editing_title);
        assert_eq!(app.files[0].data.items[0].items.len(), old + 1);
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
}
