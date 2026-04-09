use crate::terminal::model::TerminalSize;
use portable_pty::{native_pty_system, Child, CommandBuilder, MasterPty, PtySize};
use std::ffi::OsString;
use std::io::Read;
use std::path::{Path, PathBuf};
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
    let mut command = CommandBuilder::new(windows_shell_program_from_env(std::env::var_os));

    #[cfg(not(windows))]
    let mut command = CommandBuilder::new_default_prog();

    command.cwd(working_dir);
    command
}

fn windows_shell_program_from_env(get_env: impl Fn(&str) -> Option<OsString>) -> OsString {
    get_env("COMSPEC")
        .filter(|value| !value.is_empty())
        .or_else(|| {
            windows_directory_from_env(&get_env)
                .map(|dir| dir.join("System32").join("cmd.exe").into_os_string())
        })
        .unwrap_or_else(|| OsString::from(r"C:\Windows\System32\cmd.exe"))
}

fn windows_directory_from_env(get_env: &impl Fn(&str) -> Option<OsString>) -> Option<PathBuf> {
    get_env("SystemRoot")
        .or_else(|| get_env("WINDIR"))
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
}

#[cfg(test)]
mod tests {
    use super::spawn_shell;
    use std::ffi::OsString;
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
}
