mod app;
mod clipboard;
mod completer;
mod event_helpers;
mod event_loop;
mod file_watcher;
mod help;
mod input;
mod markdown_highlight;
mod markdown_view;
mod mcp_log;
mod note_editor;
#[allow(dead_code)]
mod note_view;
mod password;
#[allow(dead_code)]
mod pty_tab;
mod render;
mod render_note;
mod syntax;
mod tab_style;
#[allow(dead_code)]
mod termbuf;
mod tree_view;

use std::io;
use std::os::unix::net::UnixStream;
use std::path::PathBuf;

use clap::Parser;

use crossterm::execute;
use crossterm::terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode};

use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;

use duir_core::{FileStorage, TodoStorage};

use app::{App, FocusState};

#[derive(Parser)]
#[command(name = "duir", version, about = "Hierarchical todo tree manager")]
struct Cli {
    /// Directory containing .todo.json files
    #[arg(short, long)]
    dir: Option<PathBuf>,

    /// Specific files to open
    files: Vec<PathBuf>,

    /// Run as stdio-to-Unix-socket bridge for MCP
    #[arg(long)]
    mcp_connect: bool,
}

/// Parse CLI args with proper sysexits codes.
fn parse_cli() -> Cli {
    use clap::error::ErrorKind;

    match Cli::try_parse() {
        Ok(cli) => cli,
        Err(e) => match e.kind() {
            ErrorKind::DisplayHelp | ErrorKind::DisplayVersion => {
                print!("{e}");
                std::process::exit(0);
            }
            _ => {
                eprint!("{e}");
                std::process::exit(64); // EX_USAGE
            }
        },
    }
}

/// stdio-to-Unix-socket bridge for MCP.
///
/// Design: fail fast, never hang.
/// - No retries: the socket is created before this process is spawned.
/// - Timeouts on all I/O: a stuck peer cannot block us forever.
/// - Clean shutdown: when either direction closes, tear down the other.
fn run_mcp_bridge() -> io::Result<()> {
    use std::time::Duration;

    // 1. Validate environment — fail immediately if misconfigured.
    let socket_path = std::env::var("DUIR_MCP_SOCKET")
        .map_err(|_| io::Error::new(io::ErrorKind::NotFound, "DUIR_MCP_SOCKET not set"))?;

    if socket_path.is_empty() || socket_path.starts_with("${") {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("DUIR_MCP_SOCKET has invalid value: {socket_path:?}"),
        ));
    }

    mcp_log::log("bridge", &format!("connecting to {socket_path}"));

    // 2. Connect — one attempt, no retries. Socket must already exist.
    let socket =
        UnixStream::connect(&socket_path).map_err(|e| io::Error::new(e.kind(), format!("{socket_path}: {e}")))?;

    socket.set_read_timeout(Some(Duration::from_secs(300)))?;
    socket.set_write_timeout(Some(Duration::from_secs(30)))?;

    mcp_log::log("bridge", "connected, starting I/O threads");

    // 3. Bridge stdin↔socket with two threads.
    let mut sock_w = socket.try_clone()?;
    let mut sock_r = socket.try_clone()?;
    let shutdown = socket;

    let t_in = std::thread::spawn(move || {
        let n = io::copy(&mut io::stdin().lock(), &mut sock_w);
        mcp_log::log("bridge", &format!("stdin→socket ended: {n:?}"));
    });

    let t_out = std::thread::spawn(move || {
        // Wrap stdout in LineWriter so it flushes after each newline.
        let mut stdout = io::LineWriter::new(io::stdout().lock());
        let n = io::copy(&mut sock_r, &mut stdout);
        mcp_log::log("bridge", &format!("socket→stdout ended: {n:?}"));
    });

    // 4. When stdin closes (client done), shut down the socket to
    //    unblock the socket→stdout thread, then wait for both.
    let _ = t_in.join();
    mcp_log::log("bridge", "stdin→socket ended, shutting down");

    let _ = shutdown.shutdown(std::net::Shutdown::Both);
    let _ = t_out.join();

    mcp_log::log("bridge", "exiting");

    Ok(())
}

fn main() -> io::Result<()> {
    let cli = parse_cli();

    // Handle MCP bridge mode
    if cli.mcp_connect {
        if let Err(e) = run_mcp_bridge() {
            mcp_log::log("bridge", &format!("FATAL: {e}"));
            eprintln!("duir mcp-bridge: {e}");

            let code = match e.kind() {
                io::ErrorKind::InvalidInput => 78,                                // EX_CONFIG
                io::ErrorKind::NotFound | io::ErrorKind::ConnectionRefused => 69, // EX_UNAVAILABLE
                _ => 74,                                                          // EX_IOERR
            };

            std::process::exit(code);
        }
        return Ok(());
    }

    // Load configuration and determine storage directory
    let config = duir_core::config::Config::load();
    let storage_dir = cli.dir.clone().unwrap_or_else(|| config.storage.central.clone());

    // Initialize application state
    let mut app = App::new();
    app.flags.set_autosave_global(config.editor.autosave);
    app.note_panel_pct = config.ui.note_panel_pct;

    // Load files from storage or CLI arguments
    if cli.files.is_empty() {
        let central = config.storage.central.clone();
        for dir in &config.storage_dirs() {
            let source = if dir == &central {
                app::FileSource::Central
            } else {
                app::FileSource::Local
            };
            if let Ok(storage) = FileStorage::new(dir)
                && let Ok(names) = storage.list()
            {
                for name in &names {
                    match storage.load(name) {
                        Ok(data) => {
                            let mtime = storage.mtime(name);
                            app.add_file_with_source(name.clone(), data, source);
                            if let Some(f) = app.files.last_mut() {
                                f.disk_mtime = mtime;
                            }
                        }
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

    // Apply saved file order
    if !app.files.is_empty() {
        let state = duir_core::config::AppState::load();
        if !state.file_order.is_empty() {
            app.apply_file_order(&state.file_order);
        }
    }

    // Handle first-run experience
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

    // Set up terminal
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

    // Start file watcher
    let watcher_rx = file_watcher::spawn(&config.storage_dirs());

    // Run the main event loop
    let result = event_loop::run_loop(&mut terminal, &mut app, &storage_dir, &config, watcher_rx.as_ref());

    // Restore terminal state
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    result
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
#[allow(clippy::indexing_slicing)] // Tests: indices are controlled by test setup
mod tests;
