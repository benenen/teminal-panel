use super::view::terminal::{
    panel_interaction_mode, terminal_footer_label, terminal_footer_model, PanelInteractionMode,
};
use super::{App, Message, OverlayState, SshAuthType};
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
        settings_menu_open: false,
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
fn show_and_hide_add_project_form_updates_overlay_and_resets_fields() {
    let mut app = test_app();

    let _ = app.update(Message::ShowAddProjectForm);
    assert_eq!(app.overlay, Some(OverlayState::AddProject));

    let _ = app.update(Message::FormNameChanged("Local agent".into()));
    assert_eq!(app.add_form.name, "Local agent");

    let _ = app.update(Message::HideAddProjectForm);
    assert_eq!(app.overlay, None);
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
        assert_eq!(app.overlay, None);
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
        assert_eq!(app.overlay, Some(OverlayState::AddProject));
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
fn local_git_project_retains_git_repo_flag() {
    with_temp_config_dir(|workspace_dir: &PathBuf| {
        std::fs::create_dir_all(workspace_dir.join(".git")).expect("create git dir");

        let mut app = test_app();
        let _ = app.update(Message::AddProject {
            name: "repo".into(),
            working_dir: workspace_dir.display().to_string(),
        });

        assert!(app.config.projects[0].is_git_repo);
    });
}

#[test]
fn ssh_project_does_not_expose_git_repo_flag() {
    let service = sample_ssh_service();
    let mut app = test_app();
    app.config.ssh_services.push(service.clone());
    app.add_form.connection_kind = ProjectConnectionKind::Ssh;
    app.add_form.ssh_service_id = Some(service.id);
    app.add_form.name = "remote".into();
    app.add_form.selected_dir = Some(PathBuf::from("/srv/app"));

    let _ = app.update(Message::SubmitAddProjectForm);

    assert!(!app.config.projects[0].is_git_repo);
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
        assert_eq!(
            project_terms.terminals[1].title.as_deref(),
            Some("background-shell")
        );
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
fn terminal_footer_label_in_tabs_mode_uses_terminal_count() {
    let label = terminal_footer_label(crate::terminal::DisplayMode::Tabs, 3);

    assert_eq!(label, "Tabs mode · 3 terminals");
}

#[test]
fn terminal_footer_label_in_panel_mode_uses_terminal_count() {
    let label = terminal_footer_label(crate::terminal::DisplayMode::Panel, 2);

    assert_eq!(label, "Panel mode · 2 terminals");
}

#[test]
fn terminal_footer_model_shows_git_icon_for_git_projects() {
    let footer = terminal_footer_model(crate::terminal::DisplayMode::Tabs, 3, true);

    assert!(footer.show_git_icon);
    assert_eq!(footer.label, "Tabs mode · 3 terminals");
}

#[test]
fn terminal_footer_model_hides_git_icon_for_non_git_projects() {
    let footer = terminal_footer_model(crate::terminal::DisplayMode::Panel, 2, false);

    assert!(!footer.show_git_icon);
    assert_eq!(footer.label, "Panel mode · 2 terminals");
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
fn toggle_settings_menu_updates_menu_state() {
    let mut app = test_app();

    let _ = app.update(Message::ToggleSettingsMenu);
    assert!(app.settings_menu_open);
    assert_eq!(app.overlay, None);

    let _ = app.update(Message::ToggleSettingsMenu);
    assert!(!app.settings_menu_open);
    assert_eq!(app.overlay, None);
}

#[test]
fn showing_add_project_closes_settings_menu() {
    let mut app = test_app();

    let _ = app.update(Message::ToggleSettingsMenu);
    let _ = app.update(Message::ShowAddProjectForm);

    assert!(!app.settings_menu_open);
    assert_eq!(app.overlay, Some(OverlayState::AddProject));
}

#[test]
fn showing_ssh_services_closes_settings_menu_without_using_menu_overlay() {
    let mut app = test_app();

    let _ = app.update(Message::ToggleSettingsMenu);
    let _ = app.update(Message::ShowSshServices);

    assert!(!app.settings_menu_open);
    assert_eq!(app.overlay, Some(OverlayState::SshServices));
}

#[test]
fn hiding_settings_menu_closes_menu_without_touching_modal_overlay() {
    let mut app = test_app();

    let _ = app.update(Message::ShowAddProjectForm);
    let _ = app.update(Message::ToggleSettingsMenu);
    let _ = app.update(Message::HideSettingsMenu);

    assert!(!app.settings_menu_open);
    assert_eq!(app.overlay, Some(OverlayState::AddProject));
}

#[test]
fn showing_add_project_keeps_settings_menu_closed() {
    let mut app = test_app();

    let _ = app.update(Message::ToggleSettingsMenu);
    let _ = app.update(Message::ShowAddProjectForm);

    assert!(!app.settings_menu_open);
    assert_eq!(app.overlay, Some(OverlayState::AddProject));
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
        let persisted = AppConfig::load();
        assert_eq!(persisted.ssh_services.len(), 1);
        assert_eq!(persisted.ssh_services[0].name, "Prod");
        assert_eq!(persisted.ssh_services[0].host, "example.com");
    });
}

#[test]
fn app_new_loads_persisted_ssh_services() {
    with_temp_config_dir(|_| {
        let mut app = test_app();

        let _ = app.update(Message::ShowSshServices);
        let _ = app.update(Message::SshServiceNameChanged("Prod".into()));
        let _ = app.update(Message::SshServiceHostChanged("example.com".into()));
        let _ = app.update(Message::SshServicePortChanged("22".into()));
        let _ = app.update(Message::SshServiceUserChanged("deploy".into()));
        let _ = app.update(Message::SshServiceAuthTypeChanged(SshAuthType::Agent));
        let _ = app.update(Message::SubmitSshServiceForm);

        let (reloaded, _) = App::new();
        assert_eq!(reloaded.config.ssh_services.len(), 1);
        assert_eq!(reloaded.config.ssh_services[0].name, "Prod");
        assert_eq!(reloaded.config.ssh_services[0].host, "example.com");
    });
}

#[test]
fn submit_ssh_service_form_with_missing_required_field_keeps_overlay_open() {
    let mut app = test_app();

    let _ = app.update(Message::ShowSshServices);
    let _ = app.update(Message::SshServiceNameChanged("Prod".into()));
    let _ = app.update(Message::SshServicePortChanged("22".into()));
    let _ = app.update(Message::SshServiceUserChanged("deploy".into()));
    let _ = app.update(Message::SubmitSshServiceForm);

    assert_eq!(app.overlay, Some(OverlayState::SshServices));
    assert!(app.config.ssh_services.is_empty());
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
        let _ = app.update(Message::FormConnectionKindChanged(
            ProjectConnectionKind::Ssh,
        ));
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
fn submit_add_form_creates_ssh_project_for_non_local_remote_path() {
    with_temp_config_dir(|_| {
        let mut app = test_app();
        let service = sample_ssh_service();
        let service_id = service.id;
        app.config.ssh_services.push(service);

        let _ = app.update(Message::ShowAddProjectForm);
        let _ = app.update(Message::FormNameChanged("Remote project".into()));
        let _ = app.update(Message::FormConnectionKindChanged(
            ProjectConnectionKind::Ssh,
        ));
        let _ = app.update(Message::FormSshServiceSelected(service_id));
        let _ = app.update(Message::ProjectFolderSelected(Some(PathBuf::from(
            "/srv/remote-only",
        ))));
        let _ = app.update(Message::SubmitAddProjectForm);

        assert_eq!(app.config.projects.len(), 1);
        assert_eq!(
            app.config.projects[0].working_dir,
            PathBuf::from("/srv/remote-only")
        );
    });
}

#[test]
fn choose_project_folder_is_noop_for_ssh_projects() {
    let mut app = test_app();

    let _ = app.update(Message::ShowAddProjectForm);
    let _ = app.update(Message::FormConnectionKindChanged(
        ProjectConnectionKind::Ssh,
    ));
    let _ = app.update(Message::ChooseProjectFolder);

    assert_eq!(app.add_form.selected_dir, None);
}

#[test]
fn switching_to_ssh_preserves_manual_remote_path() {
    let mut app = test_app();

    let _ = app.update(Message::ShowAddProjectForm);
    let _ = app.update(Message::ProjectFolderSelected(Some(PathBuf::from(
        "/srv/app",
    ))));
    let _ = app.update(Message::FormConnectionKindChanged(
        ProjectConnectionKind::Ssh,
    ));

    assert_eq!(app.add_form.selected_dir, Some(PathBuf::from("/srv/app")));
}

#[test]
fn switching_back_to_local_clears_non_local_remote_path() {
    let mut app = test_app();

    let _ = app.update(Message::ShowAddProjectForm);
    let _ = app.update(Message::ProjectFolderSelected(Some(PathBuf::from(
        "/srv/app",
    ))));
    let _ = app.update(Message::FormConnectionKindChanged(
        ProjectConnectionKind::Ssh,
    ));
    let _ = app.update(Message::FormConnectionKindChanged(
        ProjectConnectionKind::Local,
    ));

    assert_eq!(app.add_form.selected_dir, None);
}

#[test]
fn project_folder_selected_none_does_not_clear_remote_path() {
    let mut app = test_app();

    let _ = app.update(Message::ShowAddProjectForm);
    let _ = app.update(Message::FormConnectionKindChanged(
        ProjectConnectionKind::Ssh,
    ));
    let _ = app.update(Message::ProjectFolderSelected(Some(PathBuf::from(
        "/srv/app",
    ))));
    let _ = app.update(Message::ProjectFolderSelected(None));

    assert_eq!(app.add_form.selected_dir, Some(PathBuf::from("/srv/app")));
}

#[test]
fn submit_add_form_with_ssh_requires_service_selection() {
    with_temp_config_dir(|workspace_dir: &PathBuf| {
        let mut app = test_app();

        let _ = app.update(Message::ShowAddProjectForm);
        let _ = app.update(Message::FormNameChanged("Remote project".into()));
        let _ = app.update(Message::FormConnectionKindChanged(
            ProjectConnectionKind::Ssh,
        ));
        let _ = app.update(Message::ProjectFolderSelected(Some(workspace_dir.clone())));
        let _ = app.update(Message::SubmitAddProjectForm);

        assert!(app.config.projects.is_empty());
        assert_eq!(app.overlay, Some(OverlayState::AddProject));
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

#[test]
fn ssh_display_location_omits_default_port() {
    let service = sample_ssh_service();

    assert_eq!(service.display_destination(), "deploy@example.com");
    assert_eq!(
        service.display_remote_location(std::path::Path::new("/srv/app")),
        "deploy@example.com:/srv/app"
    );
}

#[test]
fn ssh_display_location_keeps_non_default_port() {
    let service = SshService {
        port: 2222,
        ..sample_ssh_service()
    };

    assert_eq!(
        service.display_remote_location(std::path::Path::new("/srv/app")),
        "deploy@example.com:2222:/srv/app"
    );
}

#[test]
fn ssh_terminal_bootstrap_command_uses_key_auth() {
    let service = SshService {
        port: 2222,
        auth: SshAuth::Key {
            path: std::path::PathBuf::from("/home/test/.ssh/id_ed25519"),
            passphrase: Some("secret".into()),
        },
        ..sample_ssh_service()
    };

    let command = crate::ssh::build_terminal_bootstrap_command(
        &service,
        std::path::Path::new("/srv/my app"),
        crate::terminal::LocalShellFlavor::Posix,
    );

    assert!(command.contains("ssh"));
    assert!(command.contains("-i"));
    assert!(command.contains("/home/test/.ssh/id_ed25519"));
    assert!(command.contains("'-p' '2222'"));
    assert!(command.contains("'deploy@example.com'"));
    assert!(command.contains("/srv/my app"));
    assert!(!command.contains("secret"));
}

#[test]
fn ssh_terminal_bootstrap_command_uses_cmd_quoting_for_windows() {
    let service = SshService {
        port: 2222,
        auth: SshAuth::Key {
            path: std::path::PathBuf::from(r#"C:\Users\test user\.ssh\id_ed25519"#),
            passphrase: None,
        },
        ..sample_ssh_service()
    };

    let command = crate::ssh::build_terminal_bootstrap_command(
        &service,
        std::path::Path::new("/srv/my app"),
        crate::terminal::LocalShellFlavor::Cmd,
    );

    assert!(command.contains("\"ssh\""));
    assert!(command.contains("\"-p\" \"2222\""));
    assert!(command.contains("\"-i\" \"C:\\Users\\test user\\.ssh\\id_ed25519\""));
    assert!(command.contains("\"deploy@example.com\""));
    assert!(command.contains("exec ${SHELL:-/bin/bash} -l"));
    assert!(!command.contains("'ssh'"));
}

#[test]
fn ssh_terminal_bootstrap_command_uses_powershell_quoting_for_windows() {
    let service = SshService {
        auth: SshAuth::Key {
            path: std::path::PathBuf::from(r#"C:\Users\test user\.ssh\id_ed25519"#),
            passphrase: None,
        },
        ..sample_ssh_service()
    };

    let command = crate::ssh::build_terminal_bootstrap_command(
        &service,
        std::path::Path::new("/srv/it's app"),
        crate::terminal::LocalShellFlavor::PowerShell,
    );

    assert!(command.contains("& \"ssh\""));
    assert!(command.contains("\"C:\\Users\\test user\\.ssh\\id_ed25519\""));
    assert!(command.contains("deploy@example.com"));
    assert!(command.contains("/srv/it'\\''s app"));
}

#[test]
fn ssh_remote_browse_rejects_password_auth() {
    let service = SshService {
        auth: SshAuth::Password {
            password: "secret".into(),
        },
        ..sample_ssh_service()
    };

    let result = crate::ssh::build_remote_list_command(&service, std::path::Path::new("/srv/app"));

    assert!(matches!(
        result,
        Err(crate::ssh::RemoteListCommandError::PasswordAuthUnsupported)
    ));
}

#[test]
fn ssh_terminal_bootstrap_command_quotes_key_path_with_spaces() {
    let service = SshService {
        auth: SshAuth::Key {
            path: std::path::PathBuf::from("/home/test/.ssh/my key"),
            passphrase: None,
        },
        ..sample_ssh_service()
    };

    let command = crate::ssh::build_terminal_bootstrap_command(
        &service,
        std::path::Path::new("/srv/app"),
        crate::terminal::LocalShellFlavor::Posix,
    );

    assert!(command.contains("'/home/test/.ssh/my key'"));
}

#[test]
fn ssh_shell_quote_escapes_single_quotes() {
    let quoted = crate::ssh::shell_quote_for_test(std::path::Path::new("/srv/it's app"));

    assert_eq!(quoted, "'/srv/it'\\''s app'");
}

#[test]
fn ssh_project_subtitle_uses_remote_location_format() {
    let service = sample_ssh_service();
    let project = crate::project::Project::new_ssh(
        "Remote".into(),
        std::path::PathBuf::from("/srv/project"),
        service.id,
    );

    let subtitle = super::project_subtitle_for_test(&project, &[service]);

    assert_eq!(subtitle, "deploy@example.com:/srv/project");
}

#[test]
fn ensure_project_terminals_initializes_remote_file_state() {
    let state = crate::terminal::ProjectTerminals::new();

    assert!(state.remote_files.is_none());
}

#[test]
fn request_remote_files_marks_loading_for_agent_auth() {
    let mut app = test_app();
    let service = sample_ssh_service();
    let project = crate::project::Project::new_ssh(
        "Remote".into(),
        std::path::PathBuf::from("/srv/project"),
        service.id,
    );
    let project_id = project.id;
    app.config.ssh_services.push(service);
    app.config.projects.push(project);

    let _ = app.update(Message::RequestRemoteFiles(project_id));

    let state = app
        .terminals
        .get(&project_id)
        .and_then(|state| state.remote_files.as_ref())
        .expect("remote file state");
    assert!(matches!(
        state.status,
        crate::terminal::RemoteFileStatus::Loading
    ));
}

#[test]
fn password_auth_remote_browse_is_marked_unsupported() {
    let mut app = test_app();
    let service = SshService {
        auth: SshAuth::Password {
            password: "secret".into(),
        },
        ..sample_ssh_service()
    };
    let project = crate::project::Project::new_ssh(
        "Remote".into(),
        std::path::PathBuf::from("/srv/project"),
        service.id,
    );
    let project_id = project.id;
    app.config.ssh_services.push(service);
    app.config.projects.push(project);

    let _ = app.update(Message::RequestRemoteFiles(project_id));

    let state = app
        .terminals
        .get(&project_id)
        .and_then(|state| state.remote_files.as_ref())
        .expect("remote file state");
    assert!(matches!(
        state.status,
        crate::terminal::RemoteFileStatus::Unsupported(_)
    ));
}

#[test]
fn remote_files_loaded_updates_entries() {
    let mut app = test_app();
    let project_id = Uuid::new_v4();
    app.terminals
        .insert(project_id, crate::terminal::ProjectTerminals::new());

    let _ = app.update(Message::RemoteFilesLoaded {
        project_id,
        result: Ok(vec![crate::terminal::RemoteFileEntry {
            name: "src".into(),
            path: "/srv/project/src".into(),
            is_dir: true,
        }]),
    });

    let state = app
        .terminals
        .get(&project_id)
        .and_then(|state| state.remote_files.as_ref())
        .expect("remote file state");
    assert!(matches!(
        state.status,
        crate::terminal::RemoteFileStatus::Loaded
    ));
    assert_eq!(state.entries.len(), 1);
}

#[test]
fn selecting_ssh_project_requests_remote_files() {
    let mut app = test_app();
    let service = sample_ssh_service();
    let project = crate::project::Project::new_ssh(
        "Remote".into(),
        std::path::PathBuf::from("/srv/project"),
        service.id,
    );
    let project_id = project.id;
    app.config.ssh_services.push(service);
    app.config.projects.push(project);

    let _ = app.update(Message::SelectProject(project_id));

    let state = app
        .terminals
        .get(&project_id)
        .and_then(|state| state.remote_files.as_ref())
        .expect("remote file state");
    assert!(matches!(
        state.status,
        crate::terminal::RemoteFileStatus::Loading
    ));
}

#[test]
fn missing_ssh_service_falls_back_to_plain_path_subtitle() {
    let project = crate::project::Project::new_ssh(
        "Remote".into(),
        std::path::PathBuf::from("/srv/project"),
        Uuid::new_v4(),
    );

    let subtitle = super::project_subtitle_for_test(&project, &[]);
    assert_eq!(subtitle, "/srv/project");
}

#[test]
fn remote_file_status_message_for_unsupported_state_is_readable() {
    let status = crate::terminal::RemoteFileStatus::Unsupported(
        "Remote browsing supports SSH agent/key auth only".into(),
    );
    let message = super::remote_file_status_label_for_test(&status);

    assert_eq!(
        message,
        Some("Remote browsing supports SSH agent/key auth only")
    );
}

#[test]
fn selecting_ssh_project_only_initializes_remote_files_without_terminals() {
    let mut app = test_app();
    let service = sample_ssh_service();
    let project = crate::project::Project::new_ssh(
        "Remote".into(),
        std::path::PathBuf::from("/srv/project"),
        service.id,
    );
    let project_id = project.id;
    app.config.ssh_services.push(service);
    app.config.projects.push(project);

    let _ = app.update(Message::SelectProject(project_id));

    let project_terms = app.terminals.get(&project_id).expect("terminal state");
    assert!(project_terms.terminals.is_empty());
    assert!(project_terms.remote_files.is_some());
}

#[test]
fn open_terminal_for_ssh_project_creates_terminal_state() {
    let mut app = test_app();
    let service = sample_ssh_service();
    let project = crate::project::Project::new_ssh(
        "Remote".into(),
        std::path::PathBuf::from("/srv/project"),
        service.id,
    );
    let project_id = project.id;
    app.config.ssh_services.push(service);
    app.config.projects.push(project);

    let _ = app.update(Message::OpenTerminal(project_id));

    let project_terms = app.terminals.get(&project_id).expect("terminal state");
    assert_eq!(project_terms.terminals.len(), 1);
}

#[test]
fn parse_remote_entries_reads_kind_and_name() {
    let entries = crate::ssh::parse_remote_entries_for_test("d\tsrc\nf\tCargo.toml\n", "/srv/app")
        .expect("parse entries");

    assert_eq!(entries.len(), 2);
    assert!(entries[0].is_dir);
    assert_eq!(entries[0].path, "/srv/app/src");
    assert_eq!(entries[1].name, "Cargo.toml");
}

#[test]
fn ssh_terminal_settings_do_not_require_remote_path_locally() {
    let settings = crate::terminal::settings_for_local_shell();

    assert!(settings.backend.working_directory.is_none());
}

#[test]
fn remote_file_status_message_for_loading_state_is_readable() {
    let status = crate::terminal::RemoteFileStatus::Loading;
    let message = super::remote_file_status_label_for_test(&status);

    assert_eq!(message, Some("Loading remote files..."));
}

#[test]
fn open_terminal_for_ssh_project_with_missing_local_path_creates_terminal_state() {
    let mut app = test_app();
    let service = sample_ssh_service();
    let project = crate::project::Project::new_ssh(
        "Remote".into(),
        std::path::PathBuf::from("/definitely/not/local"),
        service.id,
    );
    let project_id = project.id;
    app.config.ssh_services.push(service);
    app.config.projects.push(project);

    let _ = app.update(Message::OpenTerminal(project_id));

    let project_terms = app.terminals.get(&project_id).expect("terminal state");
    assert_eq!(project_terms.terminals.len(), 1);
}

#[test]
fn open_terminal_for_ssh_project_bootstraps_ssh_command() {
    let mut app = test_app();
    let service = sample_ssh_service();
    let project = crate::project::Project::new_ssh(
        "Remote".into(),
        std::path::PathBuf::from("/srv/project"),
        service.id,
    );
    let project_id = project.id;
    app.config.ssh_services.push(service);
    app.config.projects.push(project);

    let _ = app.update(Message::OpenTerminal(project_id));

    let project_terms = app.terminals.get(&project_id).expect("terminal state");
    let title = project_terms.terminals[0].title.as_deref();
    assert_eq!(title, Some("ssh: deploy@example.com:/srv/project"));
}

#[test]
fn open_terminal_for_local_project_does_not_set_ssh_bootstrap_title() {
    with_temp_config_dir(|workspace_dir: &PathBuf| {
        let mut app = test_app();

        let _ = app.update(Message::AddProject {
            name: "Local project".into(),
            working_dir: workspace_dir.display().to_string(),
        });

        let project_id = app.config.projects[0].id;
        let _ = app.update(Message::OpenTerminal(project_id));

        let project_terms = app.terminals.get(&project_id).expect("terminal state");
        assert_eq!(project_terms.terminals[0].title, None);
    });
}

#[test]
fn remote_file_status_message_for_loaded_state_is_none() {
    let status = crate::terminal::RemoteFileStatus::Loaded;
    let message = super::remote_file_status_label_for_test(&status);

    assert_eq!(message, None);
}

#[test]
fn remote_file_status_message_for_idle_state_is_none() {
    let status = crate::terminal::RemoteFileStatus::Idle;
    let message = super::remote_file_status_label_for_test(&status);

    assert_eq!(message, None);
}

#[test]
fn selecting_ssh_project_does_not_reload_existing_remote_files() {
    let mut app = test_app();
    let service = sample_ssh_service();
    let project = crate::project::Project::new_ssh(
        "Remote".into(),
        std::path::PathBuf::from("/srv/project"),
        service.id,
    );
    let project_id = project.id;
    app.config.ssh_services.push(service);
    app.config.projects.push(project);
    app.terminals.insert(
        project_id,
        crate::terminal::ProjectTerminals {
            terminals: Vec::new(),
            active_index: 0,
            display_mode: crate::terminal::DisplayMode::Tabs,
            remote_files: Some(crate::terminal::RemoteFileState {
                path: "/srv/project".into(),
                status: crate::terminal::RemoteFileStatus::Loaded,
                entries: vec![crate::terminal::RemoteFileEntry {
                    name: "src".into(),
                    path: "/srv/project/src".into(),
                    is_dir: true,
                }],
            }),
        },
    );

    let _ = app.update(Message::SelectProject(project_id));

    let state = app
        .terminals
        .get(&project_id)
        .and_then(|state| state.remote_files.as_ref())
        .expect("remote file state");
    assert!(matches!(
        state.status,
        crate::terminal::RemoteFileStatus::Loaded
    ));
    assert_eq!(state.entries.len(), 1);
}

#[test]
fn remote_files_loaded_error_sets_error_state() {
    let mut app = test_app();
    let project_id = Uuid::new_v4();
    app.terminals
        .insert(project_id, crate::terminal::ProjectTerminals::new());

    let _ = app.update(Message::RemoteFilesLoaded {
        project_id,
        result: Err("boom".into()),
    });

    let state = app
        .terminals
        .get(&project_id)
        .and_then(|state| state.remote_files.as_ref())
        .expect("remote file state");
    assert!(matches!(
        state.status,
        crate::terminal::RemoteFileStatus::Error(_)
    ));
}

#[test]
fn build_terminal_bootstrap_command_quotes_destination_and_remote_dir() {
    let service = SshService {
        user: "deploy user".into(),
        host: "example.com".into(),
        ..sample_ssh_service()
    };

    let remote_dir = std::path::Path::new("/srv/it's app");
    let command = crate::ssh::build_terminal_bootstrap_command(
        &service,
        remote_dir,
        crate::terminal::LocalShellFlavor::Posix,
    );

    assert!(command.contains("'deploy user@example.com'"));
    assert!(command.contains("exec ${SHELL:-/bin/bash} -l"));
    assert!(command.contains("it"));
    assert!(command.contains("app"));
}
