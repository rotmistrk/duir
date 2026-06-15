//! stdio ↔ Unix socket bridge for MCP.
//!
//! When duir is invoked with `--mcp-connect`, this runs instead of the TUI.
//! Bridges stdin ↔ socket using two threads with `io::copy`.

use std::io::{self, Error, ErrorKind, LineWriter};
use std::net::Shutdown;
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

    let mut sock_w = socket.try_clone()?;
    let mut sock_r = socket.try_clone()?;
    let shutdown = socket;

    let t_in = thread::spawn(move || {
        let _ = io::copy(&mut io::stdin().lock(), &mut sock_w);
    });

    let t_out = thread::spawn(move || {
        let mut stdout = LineWriter::new(io::stdout().lock());
        let _ = io::copy(&mut sock_r, &mut stdout);
    });

    let _ = t_in.join();
    let _ = shutdown.shutdown(Shutdown::Both);
    let _ = t_out.join();
    Ok(())
}
