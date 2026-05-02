// PTY backend: spawns a process in a PTY, feeds output to TermBuf.

use std::io::{Read, Write};
use std::sync::mpsc;

use portable_pty::{CommandBuilder, MasterPty, PtySize, native_pty_system};

use crate::termbuf::TermBuf;

/// A live PTY process with its terminal buffer.
pub struct PtyTab {
    master: Box<dyn MasterPty + Send>,
    writer: Box<dyn Write + Send>,
    rx: mpsc::Receiver<Vec<u8>>,
    pub termbuf: TermBuf,
    pub last_output_time: std::time::Instant,
    pub finished: bool,
}

impl PtyTab {
    pub fn spawn(
        cmd: &str,
        args: &[&str],
        cols: u16,
        rows: u16,
        cwd: &std::path::Path,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let pair = open_pty(cols, rows)?;
        spawn_command(&pair, cmd, args, cwd)?;
        let reader = pair.master.try_clone_reader()?;
        let writer = pair.master.take_writer()?;
        let rx = spawn_reader_thread(reader);

        Ok(Self {
            master: pair.master,
            writer,
            rx,
            termbuf: TermBuf::new(cols as usize, rows as usize),
            last_output_time: std::time::Instant::now(),
            finished: false,
        })
    }

    pub fn poll(&mut self) {
        loop {
            match self.rx.try_recv() {
                Ok(data) => {
                    self.termbuf.process(&data);
                    self.last_output_time = std::time::Instant::now();
                }
                Err(mpsc::TryRecvError::Empty) => break,
                Err(mpsc::TryRecvError::Disconnected) => {
                    self.finished = true;
                    break;
                }
            }
        }
        // Discard terminal query responses — the child is a TUI app
        // that manages its own terminal. Sending CPR responses back
        // creates a feedback loop (child redraws → more queries → loop).
        self.termbuf.responses.clear();
    }

    pub fn write(&mut self, data: &[u8]) {
        let _ = self.writer.write_all(data);
        let _ = self.writer.flush();
    }

    pub fn resize(&mut self, cols: u16, rows: u16) {
        let _ = self.master.resize(PtySize {
            rows,
            cols,
            pixel_width: 0,
            pixel_height: 0,
        });
        self.termbuf.resize(cols as usize, rows as usize);
    }
}

fn open_pty(cols: u16, rows: u16) -> Result<portable_pty::PtyPair, Box<dyn std::error::Error>> {
    Ok(native_pty_system().openpty(PtySize {
        rows,
        cols,
        pixel_width: 0,
        pixel_height: 0,
    })?)
}

fn spawn_command(
    pair: &portable_pty::PtyPair,
    cmd: &str,
    args: &[&str],
    cwd: &std::path::Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut builder = CommandBuilder::new(cmd);
    for arg in args {
        builder.arg(arg);
    }
    builder.env("TERM", "xterm-256color");
    builder.cwd(cwd);
    pair.slave.spawn_command(builder)?;
    Ok(())
}

fn spawn_reader_thread(mut reader: Box<dyn Read + Send>) -> mpsc::Receiver<Vec<u8>> {
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        let mut buf = [0u8; 8192];
        loop {
            match reader.read(&mut buf) {
                Ok(0) | Err(_) => break,
                Ok(n) => {
                    if tx.send(buf[..n].to_vec()).is_err() {
                        break;
                    }
                }
            }
        }
    });
    rx
}
