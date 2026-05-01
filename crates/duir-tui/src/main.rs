mod app;
mod completer;
mod help;
mod input;
mod note_editor;
mod note_view;
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
use ratatui::layout::{Constraint, Direction, Layout};
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
    app.sync_editor();

    if first_run {
        app.show_about = true;
    }

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let result = run_loop(&mut terminal, &mut app, &storage_dir);

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
) -> io::Result<()> {
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
            let tree_title = if app.has_unsaved() { " Tree (*) " } else { " Tree " };
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

            // Overlays
            if app.show_about {
                help::render_about(frame, size);
            }
            if app.show_help {
                help::render_help(frame, size, app.help_scroll);
            }
        })?;

        if let Some(Event::Key(key)) = input::poll_event(Duration::from_millis(100))? {
            // Handle overlay input first
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

        // Autosave tick
        if let Ok(storage) = FileStorage::new(storage_dir) {
            for file in &mut app.files {
                if file.autosave && file.modified {
                    if let Err(e) = storage.save(&file.name, &file.data) {
                        app.status_message = format!("Autosave error: {e}");
                    } else {
                        file.modified = false;
                    }
                }
            }
        }

        if app.should_quit {
            break;
        }
    }
    Ok(())
}

fn build_status_line(app: &App) -> Line<'_> {
    if app.command_active {
        let mut spans = vec![
            Span::raw(":"),
            Span::styled(
                format!("{}▏", app.command_buffer),
                Style::default().add_modifier(Modifier::BOLD),
            ),
        ];
        // Show completion matches
        if !app.completer.matches.is_empty() {
            spans.push(Span::raw("  "));
            for (i, m) in app.completer.matches.iter().enumerate() {
                let style = if app.completer.selected == Some(i) {
                    Style::default().bg(Color::DarkGray).fg(Color::Yellow)
                } else {
                    Style::default().fg(Color::DarkGray)
                };
                spans.push(Span::styled(format!(" {m} "), style));
            }
        }
        Line::from(spans)
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
            spans.push(Span::styled(
                format!("  │ {}", app.status_message),
                Style::default().add_modifier(Modifier::DIM),
            ));
        }
        Line::from(spans)
    }
}
