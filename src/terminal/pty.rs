use crate::terminal::model::TerminalSize;
use portable_pty::{native_pty_system, Child, CommandBuilder, MasterPty, PtySize};
use std::io::Read;
use std::path::Path;
use tokio::sync::mpsc;
use uuid::Uuid;

pub type PtyResult<T> = Result<T, Box<dyn std::error::Error + Send + Sync>>;

pub struct PtyHandle {
    pub writer: Box<dyn std::io::Write + Send>,
    pub lifecycle: PtyLifecycle,
    pub controller: Box<dyn MasterPty + Send>,
}

impl PtyHandle {
    pub fn resize(&self, size: TerminalSize) -> PtyResult<()> {
        self.controller.resize(PtySize {
            rows: size.rows,
            cols: size.cols,
            pixel_width: 0,
            pixel_height: 0,
        })?;
        Ok(())
    }
}

pub struct PtyLifecycle {
    child: Option<Box<dyn Child + Send + Sync>>,
}

impl PtyLifecycle {
    pub fn new(child: Box<dyn Child + Send + Sync>) -> Self {
        Self { child: Some(child) }
    }

    pub fn shutdown(&mut self) {
        let Some(mut child) = self.child.take() else {
            return;
        };

        match child.try_wait() {
            Ok(Some(_)) => {}
            Ok(None) | Err(_) => {
                let _ = child.kill();
                let _ = child.wait();
            }
        }
    }
}

impl Drop for PtyLifecycle {
    fn drop(&mut self) {
        self.shutdown();
    }
}

pub fn spawn_shell(
    working_dir: &Path,
    tx: mpsc::Sender<(Uuid, Vec<u8>)>,
    terminal_id: Uuid,
) -> PtyResult<PtyHandle> {
    let pty_system = native_pty_system();
    let pair = pty_system.openpty(PtySize {
        rows: 24,
        cols: 80,
        pixel_width: 0,
        pixel_height: 0,
    })?;

    let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/sh".to_string());
    let mut command = CommandBuilder::new(&shell);
    command.cwd(working_dir);

    let child = pair.slave.spawn_command(command)?;
    let controller = pair.master;
    let writer = controller.take_writer()?;
    let mut reader = controller.try_clone_reader()?;

    std::thread::spawn(move || {
        let mut buffer = [0_u8; 4096];
        loop {
            match reader.read(&mut buffer) {
                Ok(0) => break,
                Ok(bytes_read) => {
                    if tx
                        .blocking_send((terminal_id, buffer[..bytes_read].to_vec()))
                        .is_err()
                    {
                        break;
                    }
                }
                Err(_) => break,
            }
        }
    });

    Ok(PtyHandle {
        writer,
        lifecycle: PtyLifecycle::new(child),
        controller,
    })
}
