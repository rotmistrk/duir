//! Append-only log for MCP bridge and listener diagnostics.
//!
//! Writes to `$XDG_RUNTIME_DIR/duir-mcp.log` (or `/tmp/duir-{uid}/duir-mcp.log`).
//! Each line: `YYYY-MM-DDTHH:MM:SS component: message`

use std::io::Write;

fn log_path() -> std::path::PathBuf {
    let dir = std::env::var("XDG_RUNTIME_DIR").unwrap_or_else(|_| {
        let uid = std::env::var("UID")
            .or_else(|_| std::env::var("EUID"))
            .unwrap_or_else(|_| "0".to_owned());
        format!("/tmp/duir-{uid}")
    });
    let dir = std::path::PathBuf::from(dir);
    let _ = std::fs::create_dir_all(&dir);
    dir.join("duir-mcp.log")
}

/// Append a log line. Silently ignores write failures.
pub fn log(component: &str, msg: &str) {
    let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(log_path()) else {
        return;
    };
    let ts = chrono::Local::now().format("%Y-%m-%dT%H:%M:%S");
    let pid = std::process::id();
    let _ = writeln!(f, "{ts} [{pid}] {component}: {msg}");
}

/// `BufRead` wrapper that logs each line read.
pub struct LoggingReader<R> {
    inner: R,
}

impl<R> LoggingReader<R> {
    pub const fn new(inner: R) -> Self {
        Self { inner }
    }
}

impl<R: std::io::Read> std::io::Read for LoggingReader<R> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        self.inner.read(buf)
    }
}

impl<R: std::io::BufRead> std::io::BufRead for LoggingReader<R> {
    fn fill_buf(&mut self) -> std::io::Result<&[u8]> {
        self.inner.fill_buf()
    }

    fn consume(&mut self, amt: usize) {
        self.inner.consume(amt);
    }

    fn read_line(&mut self, buf: &mut String) -> std::io::Result<usize> {
        let n = self.inner.read_line(buf)?;
        if n > 0 {
            let preview: String = buf.trim_end().chars().take(120).collect();
            log("mcp-recv", &preview);
        }
        Ok(n)
    }
}

/// `Write` wrapper that logs each line written.
pub struct LoggingWriter<W> {
    inner: W,
}

impl<W> LoggingWriter<W> {
    pub const fn new(inner: W) -> Self {
        Self { inner }
    }
}

impl<W: std::io::Write> std::io::Write for LoggingWriter<W> {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        if let Ok(s) = std::str::from_utf8(buf) {
            let preview: String = s.trim_end().chars().take(120).collect();
            if !preview.is_empty() {
                log("mcp-send", &preview);
            }
        }
        self.inner.write(buf)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.inner.flush()
    }
}
