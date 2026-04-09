use iced::Font;
use std::path::Path;
use uuid::Uuid;

pub struct TerminalState {
    pub id: Uuid,
    pub project_id: Uuid,
    pub terminal: iced_term::Terminal,
    pub title: Option<String>,
}

pub fn settings_for_working_dir(working_dir: &Path) -> iced_term::settings::Settings {
    iced_term::settings::Settings {
        font: iced_term::settings::FontSettings {
            size: 16.0,
            scale_factor: 1.3,
            font_type: terminal_font(),
        },
        backend: iced_term::settings::BackendSettings {
            shell: default_shell(),
            working_directory: Some(working_dir.to_path_buf()),
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
