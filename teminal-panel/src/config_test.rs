use super::*;
use crate::project::{Connection, SshAuth, SshService};

fn config_with_projects(projects: Vec<Project>) -> AppConfig {
    AppConfig {
        projects,
        ssh_services: Vec::new(),
    }
}

fn sample_ssh_service() -> SshService {
    SshService {
        id: uuid::Uuid::new_v4(),
        name: "Prod".into(),
        host: "example.com".into(),
        port: 22,
        user: "root".into(),
        auth: SshAuth::Agent,
    }
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

#[test]
fn config_serializes_ssh_services() {
    let service = sample_ssh_service();
    let config = AppConfig {
        projects: vec![Project::new_ssh(
            "Remote".into(),
            std::path::PathBuf::from("/srv/app"),
            service.id,
        )],
        ssh_services: vec![service],
    };

    let text = toml::to_string_pretty(&config).expect("serialize config");

    assert!(text.contains("[[ssh_services]]"));
    assert!(text.contains("service_id"));
}

#[test]
fn config_deserializes_ssh_services() {
    let compat: AppConfigCompat = toml::from_str(
        r#"
            [[ssh_services]]
            id = "00000000-0000-0000-0000-000000000010"
            name = "Prod"
            host = "example.com"
            port = 22
            user = "deploy"

            [ssh_services.auth]
            type = "agent"

            [[projects]]
            id = "00000000-0000-0000-0000-000000000001"
            name = "Remote Project"
            working_dir = "/srv/project"
            is_git_repo = false

            [projects.connection]
            type = "ssh"
            service_id = "00000000-0000-0000-0000-000000000010"
            "#,
    )
    .expect("deserialize config");

    let config = AppConfig::from_compat(compat);

    assert_eq!(config.ssh_services.len(), 1);
    assert!(matches!(config.projects[0].connection, Connection::Ssh { .. }));
}

#[test]
fn config_serializes_and_deserializes_password_ssh_services() {
    let service = SshService {
        id: uuid::Uuid::new_v4(),
        name: "Prod Password".into(),
        host: "example.com".into(),
        port: 22,
        user: "deploy".into(),
        auth: SshAuth::Password {
            password: "secret".into(),
        },
    };
    let config = AppConfig {
        projects: vec![],
        ssh_services: vec![service.clone()],
    };

    let text = toml::to_string_pretty(&config).expect("serialize config");
    let compat: AppConfigCompat = toml::from_str(&text).expect("deserialize config");
    let loaded = AppConfig::from_compat(compat);

    assert_eq!(loaded.ssh_services.len(), 1);
    assert_eq!(loaded.ssh_services[0].name, service.name);
    match &loaded.ssh_services[0].auth {
        SshAuth::Password { password } => assert_eq!(password, "secret"),
        other => panic!("expected password auth, got {other:?}"),
    }
}

#[test]
fn missing_ssh_services_defaults_to_empty() {
    let config = AppConfig::from_compat(AppConfigCompat {
        projects: vec![Project::new_local(
            "Demo".into(),
            std::path::PathBuf::from("/tmp/demo"),
        )],
        agents: vec![],
        ssh_services: vec![],
    });

    assert!(config.ssh_services.is_empty());
}

#[test]
fn config_path_defaults_to_home_config_directory() {
    let previous_xdg = std::env::var_os("XDG_CONFIG_HOME");
    std::env::remove_var("XDG_CONFIG_HOME");

    let path = AppConfig::config_path();

    match previous_xdg {
        Some(value) => std::env::set_var("XDG_CONFIG_HOME", value),
        None => std::env::remove_var("XDG_CONFIG_HOME"),
    }

    let home = dirs::home_dir().expect("home dir");
    assert_eq!(
        path,
        home.join(".config").join("teminal-panel").join("config.toml")
    );
}
