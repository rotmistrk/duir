mod app;
mod clipboard;
mod completer;
mod event_loop;
mod help;
mod input;
mod markdown_view;
mod note_editor;
#[allow(dead_code)]
mod note_view;
mod password;
#[allow(dead_code)]
mod pty_tab;
mod render;
mod syntax;
#[allow(dead_code)]
mod termbuf;
mod tree_view;

use std::io;
use std::path::PathBuf;

use clap::Parser;
use crossterm::execute;
use crossterm::terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode};
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;

use duir_core::{FileStorage, TodoStorage};

use app::{App, FocusState};

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

    // Ensure terminal is restored on panic
    let default_panic = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let _ = disable_raw_mode();
        let _ = execute!(io::stdout(), LeaveAlternateScreen);
        let _ = crossterm::cursor::Show;
        default_panic(info);
    }));

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(
        stdout,
        crossterm::terminal::Clear(crossterm::terminal::ClearType::All),
        crossterm::cursor::MoveTo(0, 0),
        EnterAlternateScreen
    )?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let result = event_loop::run_loop(&mut terminal, &mut app, &storage_dir, &config);

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    result
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
#[allow(clippy::indexing_slicing)] // Tests: indices are controlled by test setup
mod tests;
