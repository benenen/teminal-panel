use super::{App, Message, OverlayState, SshAuthType};
use super::view::terminal::{panel_interaction_mode, PanelInteractionMode};
use crate::config::AppConfig;
use crate::project::{panel::ProjectConnectionKind, Connection, SshAuth, SshService};
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};
use uuid::Uuid;

fn env_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

fn sample_ssh_service() -> SshService {
    SshService {
        id: Uuid::new_v4(),
        name: "Prod".into(),
        host: "example.com".into(),
        port: 22,
        user: "deploy".into(),
        auth: SshAuth::Agent,
    }
}

fn test_app() -> App {
    App {
        config: AppConfig::default(),
        selected_project: None,
        hovered_project: None,
        expanded_projects: std::collections::HashSet::new(),
        editing_terminal: None,
        add_form: Default::default(),
        overlay: None,
        ssh_service_form: Default::default(),
        editing_ssh_service: None,
        terminals: std::collections::HashMap::new(),
        next_terminal_id: 1,
    }
}

fn with_temp_config_dir<T>(f: impl FnOnce(&PathBuf) -> T) -> T {
    let _guard = env_lock().lock().expect("test env lock");
    let temp_root =
        std::env::temp_dir().join(format!("teminal-panel-tests-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&temp_root).expect("create temp config root");
    let workspace_dir = temp_root.join("workspace");
    std::fs::create_dir_all(&workspace_dir).expect("create temp workspace dir");

    let previous = std::env::var_os("XDG_CONFIG_HOME");
    std::env::set_var("XDG_CONFIG_HOME", &temp_root);

    let result = f(&workspace_dir);

    match previous {
        Some(value) => std::env::set_var("XDG_CONFIG_HOME", value),
        None => std::env::remove_var("XDG_CONFIG_HOME"),
    }

    let _ = std::fs::remove_dir_all(temp_root);
    result
}

#[test]
fn show_and_hide_add_project_form_updates_visibility_and_resets_fields() {
    let mut app = test_app();

    let _ = app.update(Message::ShowAddProjectForm);
    assert!(app.add_form.visible);

    let _ = app.update(Message::FormNameChanged("Local agent".into()));
    assert_eq!(app.add_form.name, "Local agent");

    let _ = app.update(Message::HideAddProjectForm);
    assert!(!app.add_form.visible);
    assert!(app.add_form.name.is_empty());
    assert_eq!(app.add_form.selected_dir, None);
}

#[test]
fn submit_add_form_adds_project_and_resets_form() {
    with_temp_config_dir(|workspace_dir: &PathBuf| {
        let mut app = test_app();

        let _ = app.update(Message::ShowAddProjectForm);
        let _ = app.update(Message::FormNameChanged("Local agent".into()));
        let _ = app.update(Message::ProjectFolderSelected(Some(workspace_dir.clone())));
        let _ = app.update(Message::SubmitAddProjectForm);

        assert_eq!(app.config.projects.len(), 1);
        assert_eq!(app.config.projects[0].name, "Local agent");
        assert_eq!(app.config.projects[0].working_dir, workspace_dir.clone());
        assert!(!app.add_form.visible);
        assert_eq!(app.add_form.selected_dir, None);

        let persisted = AppConfig::load();
        assert_eq!(persisted.projects.len(), 1);
        assert_eq!(persisted.projects[0].name, "Local agent");
    });
}

#[test]
fn submit_add_form_requires_valid_directory() {
    with_temp_config_dir(|_| {
        let mut app = test_app();

        let _ = app.update(Message::ShowAddProjectForm);
        let _ = app.update(Message::FormNameChanged("Local agent".into()));
        let _ = app.update(Message::ProjectFolderSelected(Some(PathBuf::from(
            "/tmp/missing-directory",
        ))));
        let _ = app.update(Message::SubmitAddProjectForm);

        assert!(app.config.projects.is_empty());
        assert!(app.add_form.visible);
        assert!(AppConfig::load().projects.is_empty());
    });
}

#[test]
fn removing_selected_project_clears_selection() {
    with_temp_config_dir(|workspace_dir: &PathBuf| {
        let mut app = test_app();

        let _ = app.update(Message::AddProject {
            name: "Local agent".into(),
            working_dir: workspace_dir.display().to_string(),
        });

        let project_id = app.config.projects[0].id;
        let _ = app.update(Message::SelectProject(project_id));
        assert_eq!(app.selected_project, Some(project_id));

        let _ = app.update(Message::RemoveProject(project_id));
        assert!(app.config.projects.is_empty());
        assert_eq!(app.selected_project, None);
        assert!(AppConfig::load().projects.is_empty());
    });
}

#[test]
fn open_terminal_without_matching_project_is_noop() {
    let mut app = test_app();
    let _ = app.update(Message::OpenTerminal(uuid::Uuid::new_v4()));
    assert!(app.terminals.is_empty());
}

#[test]
fn open_terminal_creates_iced_term_state() {
    with_temp_config_dir(|workspace_dir: &PathBuf| {
        let mut app = test_app();

        let _ = app.update(Message::AddProject {
            name: "Local project".into(),
            working_dir: workspace_dir.display().to_string(),
        });

        let project_id = app.config.projects[0].id;
        let _ = app.update(Message::OpenTerminal(project_id));

        let project_terms = app.terminals.get(&project_id).expect("terminals exist");
        assert_eq!(project_terms.terminals.len(), 1);
        assert_eq!(project_terms.terminals[0].terminal.id, 1);
        assert_eq!(project_terms.active_index, 0);
    });
}

#[test]
fn open_multiple_terminals_for_same_project() {
    with_temp_config_dir(|workspace_dir: &PathBuf| {
        let mut app = test_app();

        let _ = app.update(Message::AddProject {
            name: "Local project".into(),
            working_dir: workspace_dir.display().to_string(),
        });

        let project_id = app.config.projects[0].id;
        let _ = app.update(Message::OpenTerminal(project_id));
        let _ = app.update(Message::OpenTerminal(project_id));

        let project_terms = app.terminals.get(&project_id).expect("terminals exist");
        assert_eq!(project_terms.terminals.len(), 2);
        assert_eq!(project_terms.active_index, 1);
        assert_eq!(project_terms.terminals[0].name, "Local project * 1");
        assert_eq!(project_terms.terminals[1].name, "Local project * 2");
    });
}

#[test]
fn select_tab_changes_active_index() {
    with_temp_config_dir(|workspace_dir: &PathBuf| {
        let mut app = test_app();

        let _ = app.update(Message::AddProject {
            name: "Local project".into(),
            working_dir: workspace_dir.display().to_string(),
        });

        let project_id = app.config.projects[0].id;
        let _ = app.update(Message::OpenTerminal(project_id));
        let _ = app.update(Message::OpenTerminal(project_id));
        let _ = app.update(Message::SelectTab(project_id, 0));

        let project_terms = app.terminals.get(&project_id).expect("terminals exist");
        assert_eq!(project_terms.active_index, 0);
    });
}

#[test]
fn backend_events_do_not_override_user_selected_tab() {
    with_temp_config_dir(|workspace_dir: &PathBuf| {
        let mut app = test_app();

        let _ = app.update(Message::AddProject {
            name: "Local project".into(),
            working_dir: workspace_dir.display().to_string(),
        });

        let project_id = app.config.projects[0].id;
        let _ = app.update(Message::OpenTerminal(project_id));
        let _ = app.update(Message::OpenTerminal(project_id));
        let _ = app.update(Message::SelectTab(project_id, 0));

        let background_terminal_id = app
            .terminals
            .get(&project_id)
            .and_then(|project_terms| project_terms.terminals.get(1))
            .map(|terminal_state| terminal_state.terminal.id)
            .expect("second terminal exists");

        let _ = app.update(Message::Terminal(iced_term::Event::BackendCall(
            background_terminal_id,
            iced_term::BackendCommand::ProcessAlacrittyEvent(iced_term::AlacrittyEvent::Title(
                "background-shell".into(),
            )),
        )));

        let project_terms = app.terminals.get(&project_id).expect("terminals exist");
        assert_eq!(project_terms.active_index, 0);
        assert_eq!(project_terms.terminals[1].title.as_deref(), Some("background-shell"));
    });
}

#[test]
fn inactive_panel_clicks_activate_panel_before_terminal_interaction() {
    assert_eq!(
        panel_interaction_mode(false),
        PanelInteractionMode::ClickToActivate
    );
    assert_eq!(panel_interaction_mode(true), PanelInteractionMode::Direct);
}

#[test]
fn close_tab_removes_terminal() {
    with_temp_config_dir(|workspace_dir: &PathBuf| {
        let mut app = test_app();

        let _ = app.update(Message::AddProject {
            name: "Local project".into(),
            working_dir: workspace_dir.display().to_string(),
        });

        let project_id = app.config.projects[0].id;
        let _ = app.update(Message::OpenTerminal(project_id));
        let _ = app.update(Message::OpenTerminal(project_id));
        let _ = app.update(Message::CloseTab(project_id, 0));

        let project_terms = app.terminals.get(&project_id).expect("terminals exist");
        assert_eq!(project_terms.terminals.len(), 1);
    });
}

#[test]
fn close_last_tab_removes_project_terminals() {
    with_temp_config_dir(|workspace_dir: &PathBuf| {
        let mut app = test_app();

        let _ = app.update(Message::AddProject {
            name: "Local project".into(),
            working_dir: workspace_dir.display().to_string(),
        });

        let project_id = app.config.projects[0].id;
        let _ = app.update(Message::OpenTerminal(project_id));
        let _ = app.update(Message::CloseTab(project_id, 0));

        assert!(!app.terminals.contains_key(&project_id));
    });
}

#[test]
fn rename_terminal_updates_name() {
    with_temp_config_dir(|workspace_dir: &PathBuf| {
        let mut app = test_app();

        let _ = app.update(Message::AddProject {
            name: "Local project".into(),
            working_dir: workspace_dir.display().to_string(),
        });

        let project_id = app.config.projects[0].id;
        let _ = app.update(Message::OpenTerminal(project_id));
        let _ = app.update(Message::RenameTerminal(project_id, 0, "api-shell".into()));

        let project_terms = app.terminals.get(&project_id).expect("terminals exist");
        assert_eq!(project_terms.terminals[0].name, "api-shell");
    });
}

#[test]
fn project_folder_selected_none_preserves_existing_selection() {
    let mut app = test_app();

    let _ = app.update(Message::ShowAddProjectForm);
    let _ = app.update(Message::ProjectFolderSelected(Some(
        std::path::PathBuf::from("/tmp/demo"),
    )));
    let _ = app.update(Message::ProjectFolderSelected(None));

    assert_eq!(
        app.add_form.selected_dir,
        Some(std::path::PathBuf::from("/tmp/demo"))
    );
}

#[test]
fn toggle_settings_menu_updates_overlay_state() {
    let mut app = test_app();

    let _ = app.update(Message::ToggleSettingsMenu);
    assert_eq!(app.overlay, Some(OverlayState::SettingsMenu));

    let _ = app.update(Message::ToggleSettingsMenu);
    assert_eq!(app.overlay, None);
}

#[test]
fn show_ssh_services_opens_modal_and_resets_form() {
    let mut app = test_app();
    app.ssh_service_form.name = "stale".into();

    let _ = app.update(Message::ShowSshServices);

    assert_eq!(app.overlay, Some(OverlayState::SshServices));
    assert!(app.ssh_service_form.name.is_empty());
    assert_eq!(app.editing_ssh_service, None);
}

#[test]
fn submit_ssh_service_form_adds_service_and_persists() {
    with_temp_config_dir(|_| {
        let mut app = test_app();

        let _ = app.update(Message::ShowSshServices);
        let _ = app.update(Message::SshServiceNameChanged("Prod".into()));
        let _ = app.update(Message::SshServiceHostChanged("example.com".into()));
        let _ = app.update(Message::SshServicePortChanged("22".into()));
        let _ = app.update(Message::SshServiceUserChanged("deploy".into()));
        let _ = app.update(Message::SshServiceAuthTypeChanged(SshAuthType::Agent));
        let _ = app.update(Message::SubmitSshServiceForm);

        assert_eq!(app.config.ssh_services.len(), 1);
        assert_eq!(app.config.ssh_services[0].name, "Prod");
        assert_eq!(AppConfig::load().ssh_services.len(), 1);
    });
}

#[test]
fn edit_ssh_service_updates_existing_entry() {
    with_temp_config_dir(|_| {
        let mut app = test_app();
        let service = sample_ssh_service();
        let service_id = service.id;
        app.config.ssh_services.push(service);

        let _ = app.update(Message::EditSshService(service_id));
        let _ = app.update(Message::SshServiceHostChanged("new.example.com".into()));
        let _ = app.update(Message::SubmitSshServiceForm);

        assert_eq!(app.config.ssh_services[0].host, "new.example.com");
    });
}

#[test]
fn delete_referenced_ssh_service_is_blocked() {
    with_temp_config_dir(|workspace_dir: &PathBuf| {
        let mut app = test_app();
        let service = sample_ssh_service();
        let service_id = service.id;
        app.config.ssh_services.push(service);
        app.config.projects.push(crate::project::Project::new_ssh(
            "Remote".into(),
            workspace_dir.clone(),
            service_id,
        ));

        let _ = app.update(Message::DeleteSshService(service_id));

        assert_eq!(app.config.ssh_services.len(), 1);
    });
}

#[test]
fn submit_add_form_creates_ssh_project_when_service_selected() {
    with_temp_config_dir(|workspace_dir: &PathBuf| {
        let mut app = test_app();
        let service = sample_ssh_service();
        let service_id = service.id;
        app.config.ssh_services.push(service);

        let _ = app.update(Message::ShowAddProjectForm);
        let _ = app.update(Message::FormNameChanged("Remote project".into()));
        let _ = app.update(Message::FormConnectionKindChanged(ProjectConnectionKind::Ssh));
        let _ = app.update(Message::FormSshServiceSelected(service_id));
        let _ = app.update(Message::ProjectFolderSelected(Some(workspace_dir.clone())));
        let _ = app.update(Message::SubmitAddProjectForm);

        assert_eq!(app.config.projects.len(), 1);
        assert!(matches!(
            app.config.projects[0].connection,
            Connection::Ssh { service_id: id } if id == service_id
        ));
        assert_eq!(AppConfig::load().projects.len(), 1);
    });
}

#[test]
fn submit_add_form_with_ssh_requires_service_selection() {
    with_temp_config_dir(|workspace_dir: &PathBuf| {
        let mut app = test_app();

        let _ = app.update(Message::ShowAddProjectForm);
        let _ = app.update(Message::FormNameChanged("Remote project".into()));
        let _ = app.update(Message::FormConnectionKindChanged(ProjectConnectionKind::Ssh));
        let _ = app.update(Message::ProjectFolderSelected(Some(workspace_dir.clone())));
        let _ = app.update(Message::SubmitAddProjectForm);

        assert!(app.config.projects.is_empty());
        assert!(app.add_form.visible);
    });
}

#[test]
fn ssh_password_auth_requires_password() {
    let mut app = test_app();

    let _ = app.update(Message::SshServiceNameChanged("Prod".into()));
    let _ = app.update(Message::SshServiceHostChanged("example.com".into()));
    let _ = app.update(Message::SshServicePortChanged("22".into()));
    let _ = app.update(Message::SshServiceUserChanged("deploy".into()));
    let _ = app.update(Message::SshServiceAuthTypeChanged(SshAuthType::Password));
    let _ = app.update(Message::SubmitSshServiceForm);

    assert!(app.config.ssh_services.is_empty());
}

#[test]
fn ssh_key_auth_stores_key_path_and_passphrase() {
    with_temp_config_dir(|_| {
        let mut app = test_app();

        let _ = app.update(Message::SshServiceNameChanged("Prod".into()));
        let _ = app.update(Message::SshServiceHostChanged("example.com".into()));
        let _ = app.update(Message::SshServicePortChanged("22".into()));
        let _ = app.update(Message::SshServiceUserChanged("deploy".into()));
        let _ = app.update(Message::SshServiceAuthTypeChanged(SshAuthType::Key));
        let _ = app.update(Message::SshServiceKeyPathChanged("~/.ssh/id_rsa".into()));
        let _ = app.update(Message::SshServiceKeyPassphraseChanged("secret".into()));
        let _ = app.update(Message::SubmitSshServiceForm);

        assert!(matches!(
            app.config.ssh_services[0].auth,
            SshAuth::Key { .. }
        ));
    });
}
