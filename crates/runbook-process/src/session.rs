//! A script running in a pseudo-terminal.
//!
//! `portable-pty` gives a real PTY on every OS: a Unix PTY pair on Linux and macOS, and a
//! ConPTY pseudoconsole on Windows. Programs therefore see a terminal, keep their colors and
//! progress bars, and accept keyboard input.
//!
//! Three threads per session, all blocking (no polling):
//! - reader: blocking reads from the PTY master;
//! - waiter: blocks in `wait()` until the child exits;
//! - forwarder: coalesces queued output into batches and delivers [`SessionEvent`]s, so the
//!   callback runs on one thread, in order, with exit delivered last (after the reader hit
//!   end of stream, so no output is lost).

use std::collections::BTreeMap;
use std::io::{Read, Write};
use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex, PoisonError};
use std::thread;

use portable_pty::{ChildKiller, CommandBuilder, MasterPty, PtySize, native_pty_system};

/// Upper bound for one output batch; keeps each IPC message small enough to stay responsive.
const MAX_BATCH: usize = 64 * 1024;
const READ_BUFFER: usize = 16 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TerminalSize {
    pub rows: u16,
    pub cols: u16,
}

impl From<TerminalSize> for PtySize {
    fn from(size: TerminalSize) -> Self {
        PtySize {
            rows: size.rows,
            cols: size.cols,
            pixel_width: 0,
            pixel_height: 0,
        }
    }
}

/// What to run and where.
#[derive(Debug, Clone)]
pub struct SpawnOptions {
    /// Executable name or path; names are looked up in `env["PATH"]`.
    pub program: String,
    pub args: Vec<String>,
    pub cwd: PathBuf,
    /// The complete environment of the process (nothing else is inherited).
    pub env: BTreeMap<String, String>,
    pub size: TerminalSize,
}

/// How the process ended.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExitInfo {
    pub code: u32,
    /// Name of the terminating signal on Unix, when killed by one.
    pub signal: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SessionEvent {
    /// Raw terminal output (may split UTF-8 sequences; xterm.js reassembles them).
    Output(Vec<u8>),
    /// The process exited. Always the last event.
    Exited(ExitInfo),
}

#[derive(Debug, thiserror::Error)]
pub enum SessionError {
    #[error("cannot open a terminal: {0}")]
    OpenPty(String),
    #[error("cannot start {program}: {message}")]
    Spawn { program: String, message: String },
    #[error("terminal I/O failed: {0}")]
    Io(String),
}

enum Message {
    Output(Vec<u8>),
    ReaderDone,
    Exited(ExitInfo),
}

/// The PTY master, shared with the forwarder so it can close the terminal once the child
/// exits (needed on Windows, see `forward`). `None` once closed.
type SharedMaster = Arc<Mutex<Option<Box<dyn MasterPty + Send>>>>;

/// A running (or finished) script attached to a PTY.
pub struct Session {
    master: SharedMaster,
    writer: Box<dyn Write + Send>,
    killer: Box<dyn ChildKiller + Send + Sync>,
    pid: Option<u32>,
}

impl Session {
    /// Starts `options.program` in a new PTY. `on_event` runs on a background thread.
    pub fn spawn(
        options: SpawnOptions,
        on_event: impl FnMut(SessionEvent) + Send + 'static,
    ) -> Result<Self, SessionError> {
        let pair = native_pty_system()
            .openpty(options.size.into())
            .map_err(|err| SessionError::OpenPty(err.to_string()))?;

        let mut command = CommandBuilder::new(&options.program);
        command.args(&options.args);
        command.cwd(&options.cwd);
        command.env_clear();
        for (key, value) in &options.env {
            command.env(key, value);
        }
        // The PTY is rendered by xterm.js, which supports 256 colors and truecolor.
        command.env("TERM", "xterm-256color");
        command.env("COLORTERM", "truecolor");

        // Linux/macOS: portable-pty runs setsid() in the child, making it the leader of a
        // new session and process group whose controlling terminal is this PTY.
        // Windows: the child is attached to a new ConPTY pseudoconsole.
        let mut child = pair
            .slave
            .spawn_command(command)
            .map_err(|err| SessionError::Spawn {
                program: options.program.clone(),
                message: err.to_string(),
            })?;
        // The parent must not keep the slave end open: on Unix the reader only sees EOF once
        // every slave descriptor is closed, i.e. when the script and its children are gone.
        drop(pair.slave);

        let reader = pair
            .master
            .try_clone_reader()
            .map_err(|err| SessionError::Io(err.to_string()))?;
        let writer = pair
            .master
            .take_writer()
            .map_err(|err| SessionError::Io(err.to_string()))?;
        let killer = child.clone_killer();
        let pid = child.process_id();

        let (tx, rx) = mpsc::channel();
        spawn_reader(reader, tx.clone());
        let waiter_tx = tx;
        thread::spawn(move || {
            let info = match child.wait() {
                Ok(status) => ExitInfo {
                    code: status.exit_code(),
                    signal: status.signal().map(str::to_owned),
                },
                Err(_) => ExitInfo {
                    code: u32::MAX,
                    signal: None,
                },
            };
            let _ = waiter_tx.send(Message::Exited(info));
        });
        let master: SharedMaster = Arc::new(Mutex::new(Some(pair.master)));
        let forwarder_master = Arc::clone(&master);
        thread::spawn(move || forward(rx, on_event, forwarder_master));

        Ok(Self {
            master,
            writer,
            killer,
            pid,
        })
    }

    /// OS process id of the script's direct child, when known.
    pub fn pid(&self) -> Option<u32> {
        self.pid
    }

    /// Sends keyboard input (already encoded by the terminal, e.g. `\r`, `\x03`).
    pub fn write(&mut self, data: &[u8]) -> Result<(), SessionError> {
        self.writer
            .write_all(data)
            .and_then(|()| self.writer.flush())
            .map_err(|err| SessionError::Io(err.to_string()))
    }

    /// Tells the PTY, and through it the program, the new terminal size. Does nothing once
    /// the terminal has been closed after the script ended.
    pub fn resize(&self, size: TerminalSize) -> Result<(), SessionError> {
        let master = self.master.lock().unwrap_or_else(PoisonError::into_inner);
        match master.as_ref() {
            Some(master) => master
                .resize(size.into())
                .map_err(|err| SessionError::Io(err.to_string())),
            None => Ok(()),
        }
    }

    /// Asks the direct child to stop.
    ///
    /// Linux/macOS: portable-pty sends SIGHUP to the child only. Most tools forward the
    /// hangup or exit and take their children with them, but not all do; terminating the
    /// whole process tree reliably is milestone M4.
    /// Windows: TerminateProcess on the child.
    pub fn kill(&mut self) -> Result<(), SessionError> {
        self.killer
            .kill()
            .map_err(|err| SessionError::Io(err.to_string()))
    }
}

impl Drop for Session {
    fn drop(&mut self) {
        // Best effort: a dropped session must not leave its script running. The master is
        // closed right after, which on Unix hangs up the PTY for the whole session too.
        let _ = self.killer.kill();
    }
}

fn spawn_reader(mut reader: Box<dyn Read + Send>, tx: Sender<Message>) {
    thread::spawn(move || {
        let mut buffer = vec![0u8; READ_BUFFER];
        loop {
            match reader.read(&mut buffer) {
                // Linux reports EIO instead of EOF once the slave side is closed; both end
                // the stream.
                Ok(0) | Err(_) => break,
                Ok(n) => {
                    if tx.send(Message::Output(buffer[..n].to_vec())).is_err() {
                        break;
                    }
                }
            }
        }
        let _ = tx.send(Message::ReaderDone);
    });
}

/// Delivers output batches, then the exit once the child has exited and the reader has
/// reached the end of the stream, so output printed right before exiting is never lost or
/// reported after the exit.
fn forward(rx: Receiver<Message>, mut on_event: impl FnMut(SessionEvent), master: SharedMaster) {
    let mut reader_done = false;
    let mut exit: Option<ExitInfo> = None;
    while let Ok(message) = rx.recv() {
        let mut batch = Vec::new();
        let mut next = Some(message);
        // Drain whatever is already queued (without waiting) into one batch.
        while let Some(message) = next.take() {
            match message {
                Message::Output(bytes) => batch.extend_from_slice(&bytes),
                Message::ReaderDone => reader_done = true,
                Message::Exited(info) => {
                    exit = Some(info);
                    close_terminal_after_exit(&master);
                }
            }
            if batch.len() < MAX_BATCH {
                next = rx.try_recv().ok();
            }
        }
        if !batch.is_empty() {
            on_event(SessionEvent::Output(batch));
        }
        if let Some(info) = exit.as_ref().filter(|_| reader_done) {
            on_event(SessionEvent::Exited(info.clone()));
            return;
        }
    }
}

/// Lets the reader reach end of stream once the child has exited.
///
/// Linux/macOS: nothing to do; the reader sees EOF (EIO on Linux) by itself once the
/// script and every child holding the PTY have exited.
/// Windows: ConPTY keeps the output pipe open until the pseudoconsole is closed. Dropping
/// the master (the last owner, the slave was dropped at spawn) calls ClosePseudoConsole,
/// which flushes pending output to the reader and then closes the pipe.
fn close_terminal_after_exit(master: &SharedMaster) {
    if cfg!(windows) {
        let closed = master.lock().unwrap_or_else(PoisonError::into_inner).take();
        drop(closed);
    }
}
