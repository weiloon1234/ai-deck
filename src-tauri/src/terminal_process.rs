use crate::{
    app_core::Notify,
    cli_adapters::LaunchSpec,
    error::{AppError, Result},
    process_tree::ProcessTree,
    types::{TerminalChunk, TerminalReplay},
};
use portable_pty::{native_pty_system, CommandBuilder, MasterPty, PtySize};
use std::{
    collections::{HashMap, VecDeque},
    io::{Read, Write},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
};

const MAX_SCROLLBACK_BYTES: usize = 1024 * 1024;
struct Terminal {
    master: Mutex<Box<dyn MasterPty + Send>>,
    writer: Mutex<Box<dyn Write + Send>>,
    close_requested: AtomicBool,
    running: AtomicBool,
    output: Mutex<OutputBuffer>,
}
#[derive(Default)]
struct OutputBuffer {
    chunks: VecDeque<TerminalChunk>,
    bytes: usize,
    next_sequence: u32,
}
#[derive(Default)]
pub struct TerminalManager {
    terminals: Mutex<HashMap<String, Arc<Terminal>>>,
}

impl TerminalManager {
    pub fn spawn(
        &self,
        id: &str,
        spec: LaunchSpec,
        notify: Notify,
        on_exit: Arc<dyn Fn(u32) + Send + Sync>,
    ) -> Result<()> {
        let mut terminals = self.terminals.lock().map_err(|_| terminal_error())?;
        if terminals
            .get(id)
            .is_some_and(|t| t.running.load(Ordering::Acquire))
        {
            return Err(AppError::invalid(
                "This session already has a running process.",
            ));
        }
        let previous_sequence = terminals
            .get(id)
            .and_then(|t| t.output.lock().ok().map(|b| b.next_sequence))
            .unwrap_or(0);
        ProcessTree::check_available()?;
        let pair = native_pty_system()
            .openpty(PtySize {
                rows: 30,
                cols: 100,
                pixel_width: 0,
                pixel_height: 0,
            })
            .map_err(|_| terminal_error())?;
        let mut command = CommandBuilder::new(spec.executable);
        command.args(spec.arguments);
        command.cwd(spec.directory);
        command.env_clear();
        for (key, value) in spec.environment {
            command.env(key, value);
        }
        let mut reader = pair
            .master
            .try_clone_reader()
            .map_err(|_| terminal_error())?;
        let writer = pair.master.take_writer().map_err(|_| terminal_error())?;
        let mut child = pair
            .slave
            .spawn_command(command)
            .map_err(|_| terminal_error())?;
        drop(pair.slave);
        let mut tree = match child
            .process_id()
            .ok_or_else(terminal_error)
            .and_then(ProcessTree::capture)
        {
            Ok(tree) => tree,
            Err(error) => {
                #[cfg(unix)]
                if let Some(pid) = child.process_id() {
                    // This direct child has not been reaped, so its PID cannot
                    // have been reused. Abort a launch we cannot supervise.
                    unsafe {
                        libc::kill(pid as i32, libc::SIGKILL);
                    }
                }
                let _ = child.kill();
                let _ = child.wait();
                return Err(error);
            }
        };
        let terminal = Arc::new(Terminal {
            master: Mutex::new(pair.master),
            writer: Mutex::new(writer),
            close_requested: AtomicBool::new(false),
            running: AtomicBool::new(true),
            output: Mutex::new(OutputBuffer {
                next_sequence: previous_sequence,
                ..Default::default()
            }),
        });
        terminals.insert(id.to_owned(), terminal.clone());
        let session_id = id.to_owned();
        let output_terminal = terminal.clone();
        let process_notify = notify.clone();
        let process_session_id = session_id.clone();
        let reader_thread = std::thread::spawn(move || {
            let mut buffer = [0u8; 8192];
            let mut decoder = Utf8Decoder::default();
            let mut redactor = StreamRedactor::new(spec.redacted_values);
            while let Ok(size) = reader.read(&mut buffer) {
                if size == 0 {
                    break;
                }
                let decoded = decoder.push(&buffer[..size], false);
                let safe = redactor.push(&decoded, false);
                publish(&output_terminal, &session_id, safe, &notify);
            }
            let tail = decoder.push(&[], true);
            publish(
                &output_terminal,
                &session_id,
                redactor.push(&tail, true),
                &notify,
            );
        });
        std::thread::spawn(move || {
            let mut exit_code = None;
            let mut closing_at = None;
            let mut reported_error = false;
            loop {
                let active = tree.refresh();
                if exit_code.is_none() {
                    if let Ok(Some(status)) = child.try_wait() {
                        exit_code = Some(status.exit_code());
                    }
                }
                if terminal.close_requested.load(Ordering::Acquire) || exit_code.is_some() {
                    let since = closing_at.get_or_insert_with(std::time::Instant::now);
                    let force = since.elapsed() >= std::time::Duration::from_secs(3);
                    // An unreadable child must not prevent cleanup of other
                    // already-owned tools; each signal rechecks its identity.
                    if tree.terminate(force).is_err() && !reported_error {
                        publish(&terminal, &process_session_id, "\r\nLocal tool cleanup is unconfirmed. Keep the app open and retry Close.\r\n".into(), &process_notify);
                        reported_error = true;
                    }
                }
                if active.is_err() && !reported_error {
                    publish(&terminal, &process_session_id, "\r\nLocal process inspection is unavailable. Cleanup remains unconfirmed.\r\n".into(), &process_notify);
                    reported_error = true;
                }
                // The first scan may precede the CLI's final fork/exit. Rescan
                // after observing its exit before accepting an empty tree.
                if matches!(active, Ok(false))
                    && exit_code.is_some()
                    && matches!(tree.refresh(), Ok(false))
                {
                    break;
                }
                std::thread::sleep(std::time::Duration::from_millis(100));
            }
            let _ = reader_thread.join();
            terminal.running.store(false, Ordering::Release);
            on_exit(exit_code.unwrap_or(1));
        });
        Ok(())
    }
    pub async fn shutdown(&self) -> Result<()> {
        let ids: Vec<_> = self
            .terminals
            .lock()
            .map_err(|_| terminal_error())?
            .keys()
            .cloned()
            .collect();
        for id in &ids {
            self.close(id)?;
        }
        self.wait_for(&ids).await
    }

    pub async fn wait_for(&self, ids: &[String]) -> Result<()> {
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        while ids.iter().any(|id| self.is_running(id)) {
            if std::time::Instant::now() >= deadline {
                return Err(crate::process_tree::process_error());
            }
            tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        }
        Ok(())
    }

    fn terminal(&self, id: &str) -> Result<Arc<Terminal>> {
        self.terminals
            .lock()
            .map_err(|_| terminal_error())?
            .get(id)
            .cloned()
            .ok_or_else(|| {
                AppError::new(
                    "terminal_ended",
                    "This process is no longer attached.",
                    "Resume its CLI history or open a new session.",
                )
            })
    }
    pub fn is_running(&self, id: &str) -> bool {
        self.terminal(id)
            .is_ok_and(|t| t.running.load(Ordering::Acquire))
    }
    pub fn input(&self, id: &str, data: &str) -> Result<()> {
        if data.len() > 64 * 1024 {
            return Err(AppError::invalid("Paste at most 64 KiB at a time."));
        }
        let terminal = self.terminal(id)?;
        if !terminal.running.load(Ordering::Acquire) {
            return Err(AppError::invalid("This session has ended."));
        }
        terminal
            .writer
            .lock()
            .map_err(|_| terminal_error())?
            .write_all(data.as_bytes())
            .map_err(|_| terminal_error())?;
        Ok(())
    }
    pub fn resize(&self, id: &str, rows: u16, cols: u16) -> Result<()> {
        if !(2..=500).contains(&rows) || !(2..=1000).contains(&cols) {
            return Err(AppError::invalid("Invalid terminal dimensions."));
        }
        self.terminal(id)?
            .master
            .lock()
            .map_err(|_| terminal_error())?
            .resize(PtySize {
                rows,
                cols,
                pixel_width: 0,
                pixel_height: 0,
            })
            .map_err(|_| terminal_error())
    }
    pub fn replay(&self, id: &str) -> Result<TerminalReplay> {
        let Ok(terminal) = self.terminal(id) else {
            return Ok(TerminalReplay {
                chunks: vec![],
                running: false,
            });
        };
        let chunks = terminal
            .output
            .lock()
            .map_err(|_| terminal_error())?
            .chunks
            .iter()
            .cloned()
            .collect();
        Ok(TerminalReplay {
            chunks,
            running: terminal.running.load(Ordering::Acquire),
        })
    }
    pub fn close(&self, id: &str) -> Result<()> {
        let Ok(terminal) = self.terminal(id) else {
            return Ok(());
        };
        if !terminal.running.load(Ordering::Acquire) {
            return Ok(());
        }
        terminal.close_requested.store(true, Ordering::Release);
        Ok(())
    }
}
fn terminal_error() -> AppError {
    AppError::new(
        "terminal_process",
        "The terminal process could not complete this operation.",
        "Check the executable and project folder, then open or resume the session.",
    )
}
fn publish(terminal: &Terminal, id: &str, text: String, notify: &Notify) {
    if text.is_empty() {
        return;
    }
    if let Ok(mut output) = terminal.output.lock() {
        output.next_sequence = output.next_sequence.saturating_add(1);
        let chunk = TerminalChunk {
            session_id: id.into(),
            sequence: output.next_sequence,
            text,
        };
        output.bytes += chunk.text.len();
        output.chunks.push_back(chunk.clone());
        while output.bytes > MAX_SCROLLBACK_BYTES {
            if let Some(old) = output.chunks.pop_front() {
                output.bytes -= old.text.len();
            } else {
                break;
            }
        }
        notify(
            "terminal-output",
            serde_json::to_value(chunk).unwrap_or_default(),
        );
    }
}

#[derive(Default)]
pub struct Utf8Decoder {
    pending: Vec<u8>,
}
impl Utf8Decoder {
    pub fn push(&mut self, bytes: &[u8], finish: bool) -> String {
        self.pending.extend_from_slice(bytes);
        let mut result = String::new();
        loop {
            match std::str::from_utf8(&self.pending) {
                Ok(text) => {
                    result.push_str(text);
                    self.pending.clear();
                    break;
                }
                Err(error) => {
                    let valid = error.valid_up_to();
                    result.push_str(std::str::from_utf8(&self.pending[..valid]).unwrap());
                    self.pending.drain(..valid);
                    if let Some(length) = error.error_len() {
                        result.push('\u{fffd}');
                        self.pending.drain(..length);
                    } else {
                        if finish {
                            result.push('\u{fffd}');
                            self.pending.clear();
                        }
                        break;
                    }
                }
            }
        }
        result
    }
}
pub struct StreamRedactor {
    pending: String,
    secrets: Vec<String>,
}
impl StreamRedactor {
    pub fn new(secrets: Vec<String>) -> Self {
        Self {
            pending: String::new(),
            secrets: secrets.into_iter().filter(|s| !s.is_empty()).collect(),
        }
    }
    pub fn push(&mut self, text: &str, finish: bool) -> String {
        self.pending.push_str(text);
        for secret in &self.secrets {
            self.pending = self.pending.replace(secret, "[REDACTED]");
        }
        let keep = if finish {
            0
        } else {
            self.secrets
                .iter()
                .flat_map(|secret| {
                    (1..secret.len())
                        .filter(|&len| secret.is_char_boundary(len))
                        .map(|len| &secret[..len])
                })
                .filter(|prefix| self.pending.ends_with(prefix))
                .map(str::len)
                .max()
                .unwrap_or(0)
        };
        let tail = self.pending.split_off(self.pending.len() - keep);
        std::mem::replace(&mut self.pending, tail)
    }
}
