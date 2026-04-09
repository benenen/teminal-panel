use super::*;

fn config_with_projects(projects: Vec<Project>) -> AppConfig {
    AppConfig { projects }
}

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

    assert_eq!(config.projects.len(), 1);
    assert_eq!(config.projects[0].name, "Legacy Project");
}

#[test]
fn serializing_config_emits_projects_and_not_agents() {
    let config = config_with_projects(vec![Project::new_local(
        "Demo".into(),
        std::path::PathBuf::from("/tmp/demo"),
    )]);

    let text = toml::to_string_pretty(&config).expect("serialize config");

    assert!(text.contains("[[projects]]"));
    assert!(!text.contains("[[agents]]"));
}
