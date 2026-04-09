use crate::terminal::model::TerminalSize;
use portable_pty::{native_pty_system, Child, CommandBuilder, MasterPty, PtySize};
use std::ffi::OsString;
use std::io::Read;
use std::path::{Path, PathBuf};
use tokio::sync::mpsc;
use uuid::Uuid;

pub type PtyResult<T> = Result<T, Box<dyn std::error::Error + Send + Sync>>;
pub const DEFAULT_TERMINAL_COLS: u16 = 80;
pub const DEFAULT_TERMINAL_ROWS: u16 = 24;

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

                for _ in 0..50 {
                    match child.try_wait() {
                        Ok(Some(_)) => return,
                        Ok(None) => std::thread::sleep(std::time::Duration::from_millis(20)),
                        Err(_) => return,
                    }
                }
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
        rows: DEFAULT_TERMINAL_ROWS,
        cols: DEFAULT_TERMINAL_COLS,
        pixel_width: 0,
        pixel_height: 0,
    })?;

    let command = shell_command(working_dir);

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

fn shell_command(working_dir: &Path) -> CommandBuilder {
    #[cfg(windows)]
    let mut command = windows_shell_command();

    #[cfg(not(windows))]
    let mut command = CommandBuilder::new_default_prog();

    command.cwd(working_dir);
    command
}

#[cfg(windows)]
fn windows_shell_command() -> CommandBuilder {
    let shell = windows_shell_program_from_env(|key| std::env::var_os(key));
    let mut command = CommandBuilder::new(&shell);

    if is_cmd_exe(&shell) {
        command.args(["/Q", "/D", "/K", "chcp 65001>nul"]);
        command.env("LANG", "C.UTF-8");
        command.env("LC_ALL", "C.UTF-8");
        command.env("PYTHONUTF8", "1");
        command.env("PYTHONIOENCODING", "utf-8");
    }

    command
}

fn windows_shell_program_from_env(
    get_env: impl for<'a> Fn(&'a str) -> Option<OsString>,
) -> OsString {
    get_env("COMSPEC")
        .filter(|value| !value.is_empty())
        .or_else(|| {
            windows_directory_from_env(&get_env)
                .map(|dir| dir.join("System32").join("cmd.exe").into_os_string())
        })
        .unwrap_or_else(|| OsString::from(r"C:\Windows\System32\cmd.exe"))
}

fn windows_directory_from_env(
    get_env: &impl for<'a> Fn(&'a str) -> Option<OsString>,
) -> Option<PathBuf> {
    get_env("SystemRoot")
        .or_else(|| get_env("WINDIR"))
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
}

#[cfg(windows)]
fn is_cmd_exe(program: &OsString) -> bool {
    Path::new(program)
        .file_name()
        .and_then(|name| name.to_str())
        .map(|name| name.eq_ignore_ascii_case("cmd.exe"))
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::spawn_shell;
    use crate::terminal::model::TerminalModel;
    use std::ffi::OsString;
    use std::io::Write;
    use std::sync::{Mutex, OnceLock};
    use tokio::sync::mpsc;
    use uuid::Uuid;

    fn env_lock() -> &'static Mutex<()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(()))
    }

    struct EnvVarGuard {
        key: &'static str,
        previous: Option<OsString>,
    }

    impl EnvVarGuard {
        fn set(key: &'static str, value: &str) -> Self {
            let previous = std::env::var_os(key);
            std::env::set_var(key, value);
            Self { key, previous }
        }
    }

    impl Drop for EnvVarGuard {
        fn drop(&mut self) {
            match self.previous.take() {
                Some(value) => std::env::set_var(self.key, value),
                None => std::env::remove_var(self.key),
            }
        }
    }

    #[cfg(unix)]
    #[test]
    fn spawn_shell_ignores_invalid_shell_env_and_uses_platform_default() {
        let _guard = env_lock().lock().expect("test env lock");
        let working_dir = std::env::current_dir().expect("current dir");
        let _shell = EnvVarGuard::set("SHELL", "/definitely/missing-shell");

        let (tx, _rx) = mpsc::channel(1);
        let mut handle = spawn_shell(&working_dir, tx, Uuid::new_v4()).expect("spawn shell");

        handle.lifecycle.shutdown();
    }

    #[test]
    fn windows_shell_program_prefers_comspec() {
        let shell = super::windows_shell_program_from_env(|key| {
            (key == "COMSPEC").then(|| OsString::from(r"C:\Windows\System32\cmd.exe"))
        });

        assert_eq!(shell, OsString::from(r"C:\Windows\System32\cmd.exe"));
    }

    #[test]
    fn windows_shell_program_falls_back_to_cmd_exe() {
        let shell = super::windows_shell_program_from_env(|_| None);

        assert_eq!(shell, OsString::from(r"C:\Windows\System32\cmd.exe"));
    }

    #[test]
    fn windows_shell_program_ignores_empty_comspec() {
        let shell =
            super::windows_shell_program_from_env(|key| (key == "COMSPEC").then(OsString::new));

        assert_eq!(shell, OsString::from(r"C:\Windows\System32\cmd.exe"));
    }

    #[test]
    fn windows_shell_program_falls_back_to_system_root() {
        let shell = super::windows_shell_program_from_env(|key| {
            (key == "SystemRoot").then(|| OsString::from(r"C:\Windows"))
        });

        assert_eq!(shell, OsString::from(r"C:\Windows\System32\cmd.exe"));
    }

    #[cfg(windows)]
    #[test]
    fn spawn_shell_emits_output_after_command() {
        let working_dir = std::env::current_dir().expect("current dir");
        let runtime = tokio::runtime::Runtime::new().expect("tokio runtime");

        runtime.block_on(async {
            let terminal_id = Uuid::new_v4();
            let (tx, mut rx) = mpsc::channel(16);
            let mut handle = spawn_shell(&working_dir, tx, terminal_id).expect("spawn shell");

            handle
                .writer
                .write_all(b"echo codex-pty-test\r\n")
                .expect("write command");

            let mut output = Vec::new();
            let deadline = tokio::time::Instant::now() + tokio::time::Duration::from_secs(5);

            while tokio::time::Instant::now() < deadline {
                let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
                let Some((id, bytes)) = tokio::time::timeout(remaining, rx.recv())
                    .await
                    .expect("pty output before timeout")
                else {
                    break;
                };

                assert_eq!(id, terminal_id);
                output.extend_from_slice(&bytes);

                if String::from_utf8_lossy(&output).contains("codex-pty-test") {
                    break;
                }
            }

            handle.lifecycle.shutdown();

            assert!(
                String::from_utf8_lossy(&output).contains("codex-pty-test"),
                "pty output did not contain echoed marker: {}",
                String::from_utf8_lossy(&output)
            );
        });
    }

    #[cfg(windows)]
    #[test]
    fn spawn_shell_emits_utf8_chinese_output() {
        let working_dir = std::env::current_dir().expect("current dir");
        let runtime = tokio::runtime::Runtime::new().expect("tokio runtime");

        runtime.block_on(async {
            let terminal_id = Uuid::new_v4();
            let (tx, mut rx) = mpsc::channel(16);
            let mut handle = spawn_shell(&working_dir, tx, terminal_id).expect("spawn shell");

            handle
                .writer
                .write_all("echo 中文\r".as_bytes())
                .expect("write command");

            let mut model = TerminalModel::new(80, 24);
            let deadline = tokio::time::Instant::now() + tokio::time::Duration::from_secs(5);

            while tokio::time::Instant::now() < deadline {
                let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
                let Some((id, bytes)) = tokio::time::timeout(remaining, rx.recv())
                    .await
                    .expect("pty output before timeout")
                else {
                    break;
                };

                assert_eq!(id, terminal_id);
                model.process_output(&bytes);

                if model.visible_text_rows().join("\n").contains("中文") {
                    break;
                }
            }

            handle.lifecycle.shutdown();

            assert!(
                model.visible_text_rows().join("\n").contains("中文"),
                "shell output did not contain expected UTF-8 Chinese text: {:?}",
                model.visible_text_rows()
            );
        });
    }

    #[cfg(windows)]
    #[test]
    fn spawn_shell_emits_initial_prompt() {
        let working_dir = std::env::current_dir().expect("current dir");
        let runtime = tokio::runtime::Runtime::new().expect("tokio runtime");

        runtime.block_on(async {
            let terminal_id = Uuid::new_v4();
            let (tx, mut rx) = mpsc::channel(16);
            let mut handle = spawn_shell(&working_dir, tx, terminal_id).expect("spawn shell");

            let received = tokio::time::timeout(tokio::time::Duration::from_secs(2), rx.recv())
                .await
                .ok()
                .flatten();

            handle.lifecycle.shutdown();

            assert!(
                received.is_some(),
                "shell did not emit any initial output within timeout"
            );
        });
    }

    #[cfg(windows)]
    #[test]
    fn initial_shell_output_produces_visible_terminal_text() {
        let working_dir = std::env::current_dir().expect("current dir");
        let runtime = tokio::runtime::Runtime::new().expect("tokio runtime");

        runtime.block_on(async {
            let terminal_id = Uuid::new_v4();
            let (tx, mut rx) = mpsc::channel(16);
            let mut handle = spawn_shell(&working_dir, tx, terminal_id).expect("spawn shell");

            let mut model = TerminalModel::new(80, 24);
            let mut raw = Vec::new();
            let deadline = tokio::time::Instant::now() + tokio::time::Duration::from_secs(3);

            while tokio::time::Instant::now() < deadline {
                let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
                let Some((id, bytes)) = tokio::time::timeout(remaining, rx.recv())
                    .await
                    .expect("pty output before timeout")
                else {
                    break;
                };

                assert_eq!(id, terminal_id);
                raw.extend_from_slice(&bytes);
                model.process_output(&bytes);

                if model
                    .visible_text_rows()
                    .iter()
                    .any(|row| !row.trim().is_empty())
                {
                    break;
                }
            }

            handle.lifecycle.shutdown();

            let joined = model.visible_text_rows().join("\n");

            assert!(
                joined.trim().len() > 0,
                "initial shell output produced no visible text; raw bytes={:?}",
                raw
            );
        });
    }
}
