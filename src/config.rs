use crate::agent::Agent as Project;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AppConfig {
    #[serde(default)]
    #[serde(rename = "projects")]
    pub agents: Vec<Project>,
}

impl AppConfig {
    pub fn config_path() -> PathBuf {
        dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("teminal-panel")
            .join("config.toml")
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

        Self { agents: projects }
    }

    pub fn projects(&self) -> &[Project] {
        &self.agents
    }

    pub fn projects_mut(&mut self) -> &mut Vec<Project> {
        &mut self.agents
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn legacy_agents_toml_deserializes_into_projects() {
        let compat: AppConfigCompat = toml::from_str(
            r#"
            [[agents]]
            id = "00000000-0000-0000-0000-000000000001"
            name = "Legacy Project"
            working_dir = "/tmp/project"
            is_git_repo = false

            [agents.connection]
            type = "local"
            "#,
        )
        .expect("deserialize config");

        let config = AppConfig::from_compat(compat);

        assert_eq!(config.projects().len(), 1);
        assert_eq!(config.projects()[0].name, "Legacy Project");
    }

    #[test]
    fn serializing_config_emits_projects_and_not_agents() {
        let mut config = AppConfig::default();
        config.projects_mut().push(Project::new_local(
            "Demo".into(),
            std::path::PathBuf::from("/tmp/demo"),
        ));

        let text = toml::to_string_pretty(&config).expect("serialize config");

        assert!(text.contains("[[projects]]"));
        assert!(!text.contains("[[agents]]"));
    }
}
