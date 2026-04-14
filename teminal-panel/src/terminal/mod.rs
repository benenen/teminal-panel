use iced::Font;
use std::collections::HashMap;
use std::path::Path;

pub struct TerminalState {
    pub terminal: iced_term::Terminal,
    pub name: String,
    pub title: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DisplayMode {
    Tabs,
    Panel,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemoteFileEntry {
    pub name: String,
    pub path: String,
    pub is_dir: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RemoteFileStatus {
    Idle,
    Loading,
    Loaded,
    Error(String),
    Unsupported(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemoteFileState {
    pub path: String,
    pub status: RemoteFileStatus,
    pub entries: Vec<RemoteFileEntry>,
}

pub struct ProjectTerminals {
    pub terminals: Vec<TerminalState>,
    pub active_index: usize,
    pub display_mode: DisplayMode,
    pub remote_files: Option<RemoteFileState>,
}

impl ProjectTerminals {
    pub fn new() -> Self {
        Self {
            terminals: Vec::new(),
            active_index: 0,
            display_mode: DisplayMode::Tabs,
            remote_files: None,
        }
    }

    pub fn active_terminal(&self) -> Option<&TerminalState> {
        self.terminals.get(self.active_index)
    }

    pub fn remove_terminal(&mut self, index: usize) {
        if index < self.terminals.len() {
            self.terminals.remove(index);
            if self.active_index >= self.terminals.len() && !self.terminals.is_empty() {
                self.active_index = self.terminals.len() - 1;
            }
        }
    }
}

pub fn settings_for_working_dir(working_dir: &Path) -> iced_term::settings::Settings {
    let mut settings = settings_for_local_shell();
    settings.backend.working_directory = Some(working_dir.to_path_buf());
    settings
}

pub fn settings_for_local_shell() -> iced_term::settings::Settings {
    iced_term::settings::Settings {
        font: iced_term::settings::FontSettings {
            size: 16.0,
            scale_factor: 1.3,
            font_type: terminal_font(),
        },
        backend: iced_term::settings::BackendSettings {
            program: default_shell(),
            env: HashMap::from([
                ("TERM".into(), "xterm-256color".into()),
                ("COLORTERM".into(), "truecolor".into()),
            ]),
            ..Default::default()
        },
        ..Default::default()
    }
}

pub fn terminal_font() -> Font {
    #[cfg(windows)]
    {
        return Font::with_name("NSimSun");
    }

    #[cfg(target_os = "macos")]
    {
        return Font::with_name("Menlo");
    }

    #[cfg(all(not(windows), not(target_os = "macos")))]
    {
        Font::MONOSPACE
    }
}

fn default_shell() -> String {
    #[cfg(windows)]
    {
        return "powershell".to_string();
    }

    #[cfg(not(windows))]
    {
        std::env::var("SHELL").unwrap_or_else(|_| "/bin/bash".to_string())
    }
}
