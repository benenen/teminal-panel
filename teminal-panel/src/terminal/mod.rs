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

pub struct ProjectTerminals {
    pub terminals: Vec<TerminalState>,
    pub active_index: usize,
    pub display_mode: DisplayMode,
}

impl ProjectTerminals {
    pub fn new() -> Self {
        Self {
            terminals: Vec::new(),
            active_index: 0,
            display_mode: DisplayMode::Tabs,
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
    iced_term::settings::Settings {
        font: iced_term::settings::FontSettings {
            size: 16.0,
            scale_factor: 1.3,
            font_type: terminal_font(),
        },
        backend: iced_term::settings::BackendSettings {
            program: default_shell(),
            working_directory: Some(working_dir.to_path_buf()),
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

    #[cfg(not(windows))]
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
        return std::env::var("SHELL").unwrap_or_else(|_| "/bin/bash".to_string());
    }
}
