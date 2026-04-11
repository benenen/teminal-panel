use crate::project::{Project, SshService};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AppConfig {
    #[serde(default)]
    pub projects: Vec<Project>,
    #[serde(default)]
    pub ssh_services: Vec<SshService>,
}

impl AppConfig {
    pub fn config_path() -> PathBuf {
        let base = std::env::var_os("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .or_else(|| dirs::config_dir())
            .unwrap_or_else(|| PathBuf::from("."));

        base.join("teminal-panel").join("config.toml")
    }

    pub fn load() -> Self {
        let path = Self::config_path();
        if !path.exists() {
            return Self::default();
        }
        let content = std::fs::read_to_string(&path).unwrap_or_default();
        let compat: AppConfigCompat = toml::from_str(&content).unwrap_or_default();
        Self::from_compat(compat)
    }

    fn from_compat(compat: AppConfigCompat) -> Self {
        let projects = if compat.projects.is_empty() {
            compat.agents
        } else {
            compat.projects
        };

        Self {
            projects,
            ssh_services: compat.ssh_services,
        }
    }

    pub fn save(&self) {
        let path = Self::config_path();
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        if let Ok(content) = toml::to_string_pretty(self) {
            let _ = std::fs::write(&path, content);
        }
    }
}

#[derive(Debug, Deserialize, Default)]
struct AppConfigCompat {
    #[serde(default)]
    projects: Vec<Project>,
    #[serde(default)]
    agents: Vec<Project>,
    #[serde(default)]
    ssh_services: Vec<SshService>,
}

#[cfg(test)]
#[path = "config_test.rs"]
mod tests;
