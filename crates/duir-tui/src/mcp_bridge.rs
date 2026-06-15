//! stdio ↔ Unix socket bridge for MCP.
//!
//! When duir is invoked with `--mcp-connect`, this runs instead of the TUI.
//! Bridges stdin ↔ socket using two threads.

use std::io::{self, BufRead, BufReader, Error, ErrorKind, Write};
use std::os::unix::net::UnixStream;
use std::thread;
use std::time::Duration;

/// Run the MCP bridge. Reads `DUIR_MCP_SOCKET` env var for socket path.
///
/// # Errors
/// Returns `io::Error` on connection or I/O failure.
pub fn run() -> io::Result<()> {
    let socket_path =
        std::env::var("DUIR_MCP_SOCKET").map_err(|_| Error::new(ErrorKind::NotFound, "DUIR_MCP_SOCKET not set"))?;
    if socket_path.is_empty() || socket_path.starts_with("${") {
        return Err(Error::new(
            ErrorKind::InvalidInput,
            format!("DUIR_MCP_SOCKET has invalid value: {socket_path:?}"),
        ));
    }

    let socket = UnixStream::connect(&socket_path).map_err(|e| Error::new(e.kind(), format!("{socket_path}: {e}")))?;
    socket.set_write_timeout(Some(Duration::from_secs(30)))?;

    let mut writer = socket.try_clone()?;
    let reader = BufReader::new(socket);

    // Thread: socket → stdout
    let handle = thread::spawn(move || {
        let stdout = io::stdout();
        let mut out = stdout.lock();
        for line in reader.lines() {
            match line {
                Ok(l) => {
                    if writeln!(out, "{l}").is_err() || out.flush().is_err() {
                        break;
                    }
                }
                Err(_) => break,
            }
        }
    });

    // Main thread: stdin → socket
    let stdin = io::stdin();
    for line in stdin.lock().lines() {
        match line {
            Ok(l) => {
                if writeln!(writer, "{l}").is_err() || writer.flush().is_err() {
                    break;
                }
            }
            Err(_) => break,
        }
    }

    let _ = handle.join();
    Ok(())
}
