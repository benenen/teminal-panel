use crate::config::AppConfig;
use crate::project::{
    panel::{AddProjectForm, ProjectConnectionKind},
    Connection, Project, SshAuth, SshService,
};
use crate::terminal::{
    settings_for_working_dir, DisplayMode, ProjectTerminals, RemoteFileEntry, RemoteFileState,
    RemoteFileStatus, TerminalState,
};
use iced::{Element, Task, Theme};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use uuid::Uuid;

#[path = "view/mod.rs"]
mod view;

pub struct App {
    pub main_window_id: iced::window::Id,
    pub config: AppConfig,
    pub selected_project: Option<Uuid>,
    pub hovered_project: Option<Uuid>,
    pub expanded_projects: HashSet<Uuid>,
    pub editing_terminal: Option<(Uuid, usize)>,
    pub add_form: AddProjectForm,
    pub overlay: Option<OverlayState>,
    pub settings_menu_open: bool,
    pub ssh_service_form: SshServiceForm,
    pub editing_ssh_service: Option<Uuid>,
    pub terminals: HashMap<Uuid, ProjectTerminals>,
    pub git_windows_by_project: HashMap<Uuid, GitWindowState>,
    pub git_window_projects_by_id: HashMap<iced::window::Id, Uuid>,
    pub next_terminal_id: u64,
}

pub struct GitWindowState {
    pub window_id: iced::window::Id,
    project_name: Option<String>,
    git_window: Option<crate::git_window::GitWindow>,
}

#[derive(Debug, Clone)]
pub enum Message {
    SelectProject(Uuid),
    #[cfg(test)]
    AddProject {
        name: String,
        working_dir: String,
    },
    RemoveProject(Uuid),
    HoverProject(Option<Uuid>),
    ShowAddProjectForm,
    HideAddProjectForm,
    FormNameChanged(String),
    FormConnectionKindChanged(ProjectConnectionKind),
    FormSshServiceSelected(Uuid),
    ChooseProjectFolder,
    ProjectFolderSelected(Option<PathBuf>),
    SubmitAddProjectForm,
    ToggleSettingsMenu,
    HideSettingsMenu,
    ShowSshServices,
    HideOverlay,
    ShowAddSshServiceForm,
    EditSshService(Uuid),
    DeleteSshService(Uuid),
    SshServiceNameChanged(String),
    SshServiceHostChanged(String),
    SshServicePortChanged(String),
    SshServiceUserChanged(String),
    SshServiceAuthTypeChanged(SshAuthType),
    SshServicePasswordChanged(String),
    SshServiceKeyPathChanged(String),
    SshServiceKeyPassphraseChanged(String),
    SubmitSshServiceForm,
    CancelSshServiceForm,
    RequestRemoteFiles(Uuid),
    RemoteFilesLoaded {
        project_id: Uuid,
        result: Result<Vec<RemoteFileEntry>, String>,
    },
    OpenTerminal(Uuid),
    OpenGitWindow(Uuid),
    GitWindow(iced::window::Id, crate::git_window::Message),
    WindowCloseRequested(iced::window::Id),
    WindowClosed(iced::window::Id),
    ToggleProjectExpanded(Uuid),
    SelectTab(Uuid, usize),
    CloseTab(Uuid, usize),
    StartRenameTerminal(Uuid, usize),
    RenameTerminal(Uuid, usize, String),
    FinishRenameTerminal,
    ToggleDisplayMode(Uuid),
    Terminal(iced_term::Event),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OverlayState {
    AddProject,
    SshServices,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SshAuthType {
    Password,
    Key,
    Agent,
}

#[derive(Debug, Clone)]
pub struct SshServiceForm {
    pub name: String,
    pub host: String,
    pub port: String,
    pub user: String,
    pub auth_type: SshAuthType,
    pub password: String,
    pub key_path: String,
    pub key_passphrase: String,
    pub error: Option<String>,
}

impl Default for SshServiceForm {
    fn default() -> Self {
        Self {
            name: String::new(),
            host: String::new(),
            port: "22".into(),
            user: String::new(),
            auth_type: SshAuthType::Agent,
            password: String::new(),
            key_path: String::new(),
            key_passphrase: String::new(),
            error: None,
        }
    }
}

impl App {
    pub fn new() -> (Self, Task<Message>) {
        let (main_window_id, open_main_window) = iced::window::open(Self::main_window_settings());

        (
            Self {
                main_window_id,
                config: AppConfig::load(),
                selected_project: None,
                hovered_project: None,
                expanded_projects: HashSet::new(),
                editing_terminal: None,
                add_form: Default::default(),
                overlay: None,
                settings_menu_open: false,
                ssh_service_form: Default::default(),
                editing_ssh_service: None,
                terminals: HashMap::new(),
                git_windows_by_project: HashMap::new(),
                git_window_projects_by_id: HashMap::new(),
                next_terminal_id: 1,
            },
            open_main_window.discard(),
        )
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::SelectProject(id) => {
                self.selected_project = Some(id);
                self.expanded_projects.insert(id);
                return self.maybe_request_remote_files(id);
            }
            #[cfg(test)]
            Message::AddProject { name, working_dir } => {
                self.add_local_project(name, PathBuf::from(working_dir));
            }
            Message::RemoveProject(id) => {
                self.config.projects.retain(|project| project.id != id);
                self.terminals.remove(&id);
                let close_git_window_task = self
                    .git_windows_by_project
                    .get(&id)
                    .map(|state| state.window_id)
                    .map(|window_id| {
                        self.close_git_window(window_id);
                        iced::window::close(window_id)
                    });

                if self.selected_project == Some(id) {
                    self.selected_project = None;
                }
                if self.hovered_project == Some(id) {
                    self.hovered_project = None;
                }

                self.config.save();

                if let Some(task) = close_git_window_task {
                    return task;
                }
            }
            Message::HoverProject(id) => {
                self.hovered_project = id;
            }
            Message::ShowAddProjectForm => {
                self.add_form = Default::default();
                self.settings_menu_open = false;
                self.overlay = Some(OverlayState::AddProject);
            }
            Message::HideAddProjectForm => {
                self.add_form = Default::default();
                if self.overlay == Some(OverlayState::AddProject) {
                    self.overlay = None;
                }
            }
            Message::FormNameChanged(value) => {
                self.add_form.name = value;
            }
            Message::FormConnectionKindChanged(kind) => {
                self.add_form.connection_kind = kind;
                if kind == ProjectConnectionKind::Local {
                    self.add_form.ssh_service_id = None;
                    if self
                        .add_form
                        .selected_dir
                        .as_ref()
                        .is_some_and(|path| !path.is_dir())
                    {
                        self.add_form.selected_dir = None;
                    }
                }
            }
            Message::FormSshServiceSelected(service_id) => {
                self.add_form.ssh_service_id = Some(service_id);
            }
            Message::ChooseProjectFolder => {
                if self.add_form.connection_kind == ProjectConnectionKind::Ssh {
                    return Task::none();
                }

                return Task::perform(
                    async {
                        rfd::AsyncFileDialog::new()
                            .pick_folder()
                            .await
                            .map(|handle| handle.path().to_path_buf())
                    },
                    Message::ProjectFolderSelected,
                );
            }
            Message::ProjectFolderSelected(selection) => {
                if let Some(path) = selection {
                    self.add_form.selected_dir = Some(path);
                }
            }
            Message::SubmitAddProjectForm => {
                if let Some(path) = self.add_form.selected_dir.clone() {
                    let added = match self.add_form.connection_kind {
                        ProjectConnectionKind::Local => {
                            self.add_local_project(self.add_form.name.clone(), path)
                        }
                        ProjectConnectionKind::Ssh => self.add_ssh_project(
                            self.add_form.name.clone(),
                            path,
                            self.add_form.ssh_service_id,
                        ),
                    };

                    if added {
                        self.add_form = Default::default();
                        self.overlay = None;
                        self.settings_menu_open = false;
                    }
                }
            }
            Message::ToggleSettingsMenu => {
                self.settings_menu_open = !self.settings_menu_open;
            }
            Message::HideSettingsMenu => {
                self.settings_menu_open = false;
            }
            Message::ShowSshServices => {
                self.add_form = Default::default();
                self.settings_menu_open = false;
                self.overlay = Some(OverlayState::SshServices);
                self.editing_ssh_service = None;
                self.ssh_service_form = Default::default();
            }
            Message::HideOverlay => {
                self.overlay = None;
                self.settings_menu_open = false;
                self.editing_ssh_service = None;
                self.ssh_service_form = Default::default();
            }
            Message::ShowAddSshServiceForm => {
                self.editing_ssh_service = None;
                self.ssh_service_form = Default::default();
            }
            Message::EditSshService(service_id) => {
                if let Some(service) = self.config.ssh_services.iter().find(|s| s.id == service_id)
                {
                    self.editing_ssh_service = Some(service_id);
                    self.ssh_service_form = SshServiceForm::from_service(service);
                }
            }
            Message::DeleteSshService(service_id) => {
                if self
                    .config
                    .projects
                    .iter()
                    .any(|project| matches!(project.connection, Connection::Ssh { service_id: id } if id == service_id))
                {
                    return Task::none();
                }

                self.config
                    .ssh_services
                    .retain(|service| service.id != service_id);
                if self.editing_ssh_service == Some(service_id) {
                    self.editing_ssh_service = None;
                    self.ssh_service_form = Default::default();
                }
                self.config.save();
            }
            Message::SshServiceNameChanged(value) => {
                self.ssh_service_form.name = value;
                self.ssh_service_form.error = None;
            }
            Message::SshServiceHostChanged(value) => {
                self.ssh_service_form.host = value;
                self.ssh_service_form.error = None;
            }
            Message::SshServicePortChanged(value) => {
                self.ssh_service_form.port = value;
                self.ssh_service_form.error = None;
            }
            Message::SshServiceUserChanged(value) => {
                self.ssh_service_form.user = value;
                self.ssh_service_form.error = None;
            }
            Message::SshServiceAuthTypeChanged(value) => {
                self.ssh_service_form.auth_type = value;
                self.ssh_service_form.error = None;
            }
            Message::SshServicePasswordChanged(value) => {
                self.ssh_service_form.password = value;
                self.ssh_service_form.error = None;
            }
            Message::SshServiceKeyPathChanged(value) => {
                self.ssh_service_form.key_path = value;
                self.ssh_service_form.error = None;
            }
            Message::SshServiceKeyPassphraseChanged(value) => {
                self.ssh_service_form.key_passphrase = value;
                self.ssh_service_form.error = None;
            }
            Message::SubmitSshServiceForm => {
                if let Some(service) = self
                    .ssh_service_form
                    .to_service(self.editing_ssh_service.unwrap_or_else(Uuid::new_v4))
                {
                    if let Some(editing_id) = self.editing_ssh_service {
                        if let Some(existing) = self
                            .config
                            .ssh_services
                            .iter_mut()
                            .find(|existing| existing.id == editing_id)
                        {
                            *existing = service;
                        }
                    } else {
                        self.config.ssh_services.push(service);
                    }
                    self.config.save();
                    self.editing_ssh_service = None;
                    self.ssh_service_form = Default::default();
                } else {
                    self.ssh_service_form.error = Some(self.ssh_service_form.validation_error());
                }
            }
            Message::CancelSshServiceForm => {
                self.editing_ssh_service = None;
                self.ssh_service_form = Default::default();
            }
            Message::RequestRemoteFiles(project_id) => {
                let Some(project) = self
                    .config
                    .projects
                    .iter()
                    .find(|project| project.id == project_id)
                else {
                    return Task::none();
                };

                let Connection::Ssh { service_id } = project.connection else {
                    return Task::none();
                };

                let path = project.working_dir.clone();
                let Some(service) = self
                    .config
                    .ssh_services
                    .iter()
                    .find(|service| service.id == service_id)
                    .cloned()
                else {
                    self.ensure_project_terminals(project_id).remote_files =
                        Some(RemoteFileState {
                            path: path.display().to_string(),
                            status: RemoteFileStatus::Error("SSH service not found".into()),
                            entries: Vec::new(),
                        });
                    return Task::none();
                };

                if matches!(service.auth, SshAuth::Password { .. }) {
                    self.ensure_project_terminals(project_id).remote_files =
                        Some(RemoteFileState {
                            path: path.display().to_string(),
                            status: RemoteFileStatus::Unsupported(
                                "Remote browsing supports SSH agent/key auth only".into(),
                            ),
                            entries: Vec::new(),
                        });
                    return Task::none();
                }

                self.set_remote_files_loading(project_id, &path);

                return Task::perform(
                    async move { crate::ssh::load_remote_entries(&service, &path) },
                    move |result| Message::RemoteFilesLoaded { project_id, result },
                );
            }
            Message::RemoteFilesLoaded { project_id, result } => {
                let remote_files = self
                    .ensure_project_terminals(project_id)
                    .remote_files
                    .get_or_insert(RemoteFileState {
                        path: String::new(),
                        status: RemoteFileStatus::Idle,
                        entries: Vec::new(),
                    });

                match result {
                    Ok(entries) => {
                        remote_files.status = RemoteFileStatus::Loaded;
                        remote_files.entries = entries;
                    }
                    Err(error) => {
                        remote_files.status = RemoteFileStatus::Error(error);
                        remote_files.entries.clear();
                    }
                }
            }
            Message::OpenTerminal(project_id) => {
                if let Some(project) = self.config.projects.iter().find(|p| p.id == project_id) {
                    let settings = match &project.connection {
                        Connection::Local => settings_for_working_dir(&project.working_dir),
                        Connection::Ssh { .. } => crate::terminal::settings_for_local_shell(),
                    };

                    let local_shell_flavor =
                        crate::terminal::local_shell_flavor_for_settings(&settings);

                    match iced_term::Terminal::new(self.next_terminal_id, settings) {
                        Ok(mut terminal) => {
                            self.next_terminal_id += 1;
                            let widget_id = terminal.widget_id().clone();

                            let project_name = project.name.clone();
                            let ssh_bootstrap_title = match project.connection {
                                Connection::Ssh { service_id } => self
                                    .config
                                    .ssh_services
                                    .iter()
                                    .find(|service| service.id == service_id)
                                    .map(|service| {
                                        let command = crate::ssh::build_terminal_bootstrap_command(
                                            service,
                                            &project.working_dir,
                                            local_shell_flavor,
                                        );
                                        let _ =
                                            terminal.handle(iced_term::Command::ProxyToBackend(
                                                iced_term::BackendCommand::Write(
                                                    format!("{}\r", command).into_bytes(),
                                                ),
                                            ));
                                        format!(
                                            "ssh: {}",
                                            service.display_remote_location(&project.working_dir)
                                        )
                                    }),
                                Connection::Local => None,
                            };

                            let project_terms = self
                                .terminals
                                .entry(project_id)
                                .or_insert_with(ProjectTerminals::new);
                            let term_num = project_terms.terminals.len() + 1;

                            project_terms.terminals.push(TerminalState {
                                terminal,
                                name: format!("{} * {}", project_name, term_num),
                                title: ssh_bootstrap_title,
                            });
                            project_terms.active_index = project_terms.terminals.len() - 1;

                            self.expanded_projects.insert(project_id);

                            return iced_term::TerminalView::focus(widget_id);
                        }
                        Err(e) => {
                            eprintln!("Failed to create terminal: {e}");
                        }
                    }
                }
            }
            Message::OpenGitWindow(project_id) => {
                let Some((project_name, working_dir, is_git_repo)) = self
                    .config
                    .projects
                    .iter()
                    .find(|project| project.id == project_id)
                    .map(|project| {
                        (
                            project.name.clone(),
                            project.working_dir.clone(),
                            project.is_git_repo,
                        )
                    })
                else {
                    return Task::none();
                };

                if !is_git_repo {
                    return Task::none();
                }

                if let Some(state) = self.git_windows_by_project.get(&project_id) {
                    return iced::window::gain_focus(state.window_id);
                }

                let (window_id, open_window) = iced::window::open(Self::git_window_settings());
                let (git_window, task) = crate::git_window::GitWindow::new(
                    project_id,
                    project_name.clone(),
                    working_dir,
                );

                self.track_git_window_with_state(
                    project_id,
                    GitWindowState {
                        window_id,
                        project_name: Some(project_name),
                        git_window: Some(git_window),
                    },
                );

                return Task::batch([
                    open_window.discard(),
                    task.map(move |message| Message::GitWindow(window_id, message)),
                ]);
            }
            Message::GitWindow(window_id, message) => {
                if let Some(git_window) = self.git_window_for_window_mut(window_id) {
                    return git_window
                        .update(message)
                        .map(move |message| Message::GitWindow(window_id, message));
                }
            }
            Message::WindowCloseRequested(window_id) => {
                if window_id == self.main_window_id {
                    return iced::exit();
                }
            }
            Message::WindowClosed(window_id) => {
                if window_id != self.main_window_id {
                    self.close_git_window(window_id);
                }
            }
            Message::ToggleProjectExpanded(id) => {
                if !self.expanded_projects.remove(&id) {
                    self.expanded_projects.insert(id);
                    return self.maybe_request_remote_files(id);
                }
            }
            Message::SelectTab(project_id, index) => {
                self.selected_project = Some(project_id);
                self.expanded_projects.insert(project_id);
                if let Some(project_terms) = self.terminals.get_mut(&project_id) {
                    if index < project_terms.terminals.len() {
                        project_terms.active_index = index;
                        let widget_id = project_terms.terminals[index].terminal.widget_id().clone();
                        return iced_term::TerminalView::focus(widget_id);
                    }
                }
            }
            Message::CloseTab(project_id, index) => {
                if let Some(project_terms) = self.terminals.get_mut(&project_id) {
                    project_terms.remove_terminal(index);
                    if project_terms.terminals.is_empty() {
                        self.terminals.remove(&project_id);
                    }
                }
                if self.editing_terminal == Some((project_id, index)) {
                    self.editing_terminal = None;
                }
            }
            Message::StartRenameTerminal(project_id, index) => {
                self.editing_terminal = Some((project_id, index));
            }
            Message::RenameTerminal(project_id, index, new_name) => {
                if let Some(project_terms) = self.terminals.get_mut(&project_id) {
                    if let Some(ts) = project_terms.terminals.get_mut(index) {
                        ts.name = new_name;
                    }
                }
            }
            Message::FinishRenameTerminal => {
                self.editing_terminal = None;
            }
            Message::ToggleDisplayMode(project_id) => {
                if let Some(project_terms) = self.terminals.get_mut(&project_id) {
                    project_terms.display_mode = match project_terms.display_mode {
                        DisplayMode::Tabs => DisplayMode::Panel,
                        DisplayMode::Panel => DisplayMode::Tabs,
                    };
                }
            }
            Message::Terminal(iced_term::Event::BackendCall(term_id, cmd)) => {
                let mut closed = None;

                for (project_id, project_terms) in self.terminals.iter_mut() {
                    if let Some((idx, ts)) = project_terms
                        .terminals
                        .iter_mut()
                        .enumerate()
                        .find(|(_, ts)| ts.terminal.id == term_id)
                    {
                        match ts.terminal.handle(iced_term::Command::ProxyToBackend(cmd)) {
                            iced_term::actions::Action::Shutdown => {
                                closed = Some((*project_id, idx));
                            }
                            iced_term::actions::Action::ChangeTitle(title) => {
                                ts.title = Some(title);
                            }
                            iced_term::actions::Action::Ignore => {}
                        }
                        break;
                    }
                }

                if let Some((project_id, idx)) = closed {
                    if let Some(project_terms) = self.terminals.get_mut(&project_id) {
                        project_terms.remove_terminal(idx);
                        if project_terms.terminals.is_empty() {
                            self.terminals.remove(&project_id);
                        }
                    }
                }
            }
        }

        Task::none()
    }

    fn add_local_project(&mut self, name: String, working_dir: PathBuf) -> bool {
        let name = name.trim().to_string();

        if name.is_empty() || !working_dir.is_dir() {
            return false;
        }

        self.config
            .projects
            .push(Project::new_local(name, working_dir));
        self.config.save();
        true
    }

    fn add_ssh_project(
        &mut self,
        name: String,
        working_dir: PathBuf,
        service_id: Option<Uuid>,
    ) -> bool {
        let name = name.trim().to_string();

        let Some(service_id) = service_id else {
            return false;
        };

        if name.is_empty() {
            return false;
        }

        if !self
            .config
            .ssh_services
            .iter()
            .any(|service| service.id == service_id)
        {
            return false;
        }

        self.config
            .projects
            .push(Project::new_ssh(name, working_dir, service_id));
        self.config.save();
        true
    }

    pub fn view(&self) -> Element<'_, Message> {
        let main_content = iced::widget::row![self.view_project_panel(), self.view_terminal_area()]
            .spacing(0)
            .padding(0);

        let mut layers: Vec<Element<'_, Message>> = vec![main_content.into()];

        if let Some(overlay) = self.overlay {
            let overlay_view = match overlay {
                OverlayState::AddProject => self.view_add_project_overlay(),
                OverlayState::SshServices => self.view_ssh_services_overlay(),
            };
            layers.push(overlay_view);
        }

        iced::widget::Stack::with_children(layers).into()
    }

    pub fn view_window(&self, window_id: iced::window::Id) -> Element<'_, Message> {
        if window_id == self.main_window_id {
            return self.view();
        }

        if let Some(git_window) = self.git_window_for_window(window_id) {
            return git_window
                .view()
                .map(move |message| Message::GitWindow(window_id, message));
        }

        iced::widget::container(iced::widget::text("Window unavailable"))
            .center_x(iced::Length::Fill)
            .center_y(iced::Length::Fill)
            .into()
    }

    pub fn title(&self, window_id: iced::window::Id) -> String {
        if window_id == self.main_window_id {
            return "teminal-panel".into();
        }

        self.git_window_state_for_window(window_id)
            .and_then(|state| state.project_name.as_ref())
            .map(|project_name| format!("Git - {project_name}"))
            .unwrap_or_else(|| "Git".into())
    }

    pub fn theme(&self, _window_id: iced::window::Id) -> Theme {
        Theme::Dark
    }

    pub fn subscription(&self) -> iced::Subscription<Message> {
        let mut subscriptions: Vec<iced::Subscription<Message>> = self
            .terminals
            .values()
            .flat_map(|pt| pt.terminals.iter())
            .map(|ts| ts.terminal.subscription().map(Message::Terminal))
            .collect();

        subscriptions.push(iced::window::close_requests().map(Message::WindowCloseRequested));
        subscriptions.push(iced::window::close_events().map(Message::WindowClosed));

        iced::Subscription::batch(subscriptions)
    }
}

pub(crate) fn project_subtitle(project: &Project, services: &[SshService]) -> String {
    match project.connection {
        Connection::Local => project.working_dir.display().to_string(),
        Connection::Ssh { service_id } => services
            .iter()
            .find(|service| service.id == service_id)
            .map(|service| service.display_remote_location(&project.working_dir))
            .unwrap_or_else(|| project.working_dir.display().to_string()),
    }
}

pub(crate) fn remote_file_status_label(status: &RemoteFileStatus) -> Option<&str> {
    match status {
        RemoteFileStatus::Loading => Some("Loading remote files..."),
        RemoteFileStatus::Error(message) | RemoteFileStatus::Unsupported(message) => Some(message),
        RemoteFileStatus::Idle | RemoteFileStatus::Loaded => None,
    }
}

#[cfg(test)]
pub(crate) fn project_subtitle_for_test(project: &Project, services: &[SshService]) -> String {
    project_subtitle(project, services)
}

#[cfg(test)]
pub(crate) fn remote_file_status_label_for_test(status: &RemoteFileStatus) -> Option<&str> {
    remote_file_status_label(status)
}

impl App {
    fn main_window_settings() -> iced::window::Settings {
        iced::window::Settings::default()
    }

    fn git_window_settings() -> iced::window::Settings {
        iced::window::Settings {
            size: iced::Size::new(1100.0, 720.0),
            min_size: Some(iced::Size::new(840.0, 560.0)),
            ..iced::window::Settings::default()
        }
    }

    pub(crate) fn can_open_git_window(&self, project_id: Uuid) -> bool {
        self.config
            .projects
            .iter()
            .find(|project| project.id == project_id)
            .is_some_and(|project| {
                project.is_git_repo && !self.git_windows_by_project.contains_key(&project_id)
            })
    }

    fn track_git_window_with_state(&mut self, project_id: Uuid, state: GitWindowState) -> bool {
        if self.git_windows_by_project.contains_key(&project_id)
            || self
                .git_window_projects_by_id
                .contains_key(&state.window_id)
        {
            return false;
        }

        self.git_window_projects_by_id
            .insert(state.window_id, project_id);
        self.git_windows_by_project.insert(project_id, state);
        true
    }

    #[cfg(test)]
    pub(crate) fn track_git_window(
        &mut self,
        project_id: Uuid,
        window_id: iced::window::Id,
    ) -> bool {
        self.track_git_window_with_state(
            project_id,
            GitWindowState {
                window_id,
                project_name: None,
                git_window: None,
            },
        )
    }

    pub(crate) fn close_git_window(&mut self, window_id: iced::window::Id) {
        if let Some(project_id) = self.git_window_projects_by_id.remove(&window_id) {
            self.git_windows_by_project.remove(&project_id);
        }
    }

    fn git_window_state_for_window(&self, window_id: iced::window::Id) -> Option<&GitWindowState> {
        let project_id = self.git_window_projects_by_id.get(&window_id)?;
        self.git_windows_by_project.get(project_id)
    }

    fn git_window_for_window(
        &self,
        window_id: iced::window::Id,
    ) -> Option<&crate::git_window::GitWindow> {
        self.git_window_state_for_window(window_id)
            .and_then(|state| state.git_window.as_ref())
    }

    fn git_window_for_window_mut(
        &mut self,
        window_id: iced::window::Id,
    ) -> Option<&mut crate::git_window::GitWindow> {
        let project_id = *self.git_window_projects_by_id.get(&window_id)?;
        self.git_windows_by_project
            .get_mut(&project_id)
            .and_then(|state| state.git_window.as_mut())
    }

    fn ensure_project_terminals(&mut self, project_id: Uuid) -> &mut ProjectTerminals {
        self.terminals
            .entry(project_id)
            .or_insert_with(ProjectTerminals::new)
    }

    fn set_remote_files_loading(&mut self, project_id: Uuid, path: &std::path::Path) {
        self.ensure_project_terminals(project_id).remote_files = Some(RemoteFileState {
            path: path.display().to_string(),
            status: RemoteFileStatus::Loading,
            entries: Vec::new(),
        });
    }

    fn maybe_request_remote_files(&mut self, project_id: Uuid) -> Task<Message> {
        let Some(project) = self
            .config
            .projects
            .iter()
            .find(|project| project.id == project_id)
        else {
            return Task::none();
        };

        if !matches!(project.connection, Connection::Ssh { .. }) {
            return Task::none();
        }

        let should_request = self
            .terminals
            .get(&project_id)
            .and_then(|state| state.remote_files.as_ref())
            .is_none();

        if should_request {
            self.update(Message::RequestRemoteFiles(project_id))
        } else {
            Task::none()
        }
    }
}

impl SshServiceForm {
    fn from_service(service: &SshService) -> Self {
        let (auth_type, password, key_path, key_passphrase) = match &service.auth {
            SshAuth::Password { password } => (
                SshAuthType::Password,
                password.clone(),
                String::new(),
                String::new(),
            ),
            SshAuth::Key { path, passphrase } => (
                SshAuthType::Key,
                String::new(),
                path.display().to_string(),
                passphrase.clone().unwrap_or_default(),
            ),
            SshAuth::Agent => (
                SshAuthType::Agent,
                String::new(),
                String::new(),
                String::new(),
            ),
        };

        Self {
            name: service.name.clone(),
            host: service.host.clone(),
            port: service.port.to_string(),
            user: service.user.clone(),
            auth_type,
            password,
            key_path,
            key_passphrase,
            error: None,
        }
    }

    fn validation_error(&self) -> String {
        if self.name.trim().is_empty() {
            return "Name is required".into();
        }
        if self.host.trim().is_empty() {
            return "Host is required".into();
        }
        if self.port.trim().is_empty() {
            return "Port is required".into();
        }
        if self.port.trim().parse::<u16>().is_err() {
            return "Port must be a valid number".into();
        }
        if self.user.trim().is_empty() {
            return "User is required".into();
        }
        match self.auth_type {
            SshAuthType::Agent => {}
            SshAuthType::Password if self.password.is_empty() => {
                return "Password is required".into();
            }
            SshAuthType::Key if self.key_path.trim().is_empty() => {
                return "Key path is required".into();
            }
            SshAuthType::Password | SshAuthType::Key => {}
        }
        String::new()
    }

    fn can_submit(&self) -> bool {
        self.validation_error().is_empty()
    }

    fn to_service(&self, id: Uuid) -> Option<SshService> {
        if !self.can_submit() {
            return None;
        }

        let auth = match self.auth_type {
            SshAuthType::Agent => SshAuth::Agent,
            SshAuthType::Password => SshAuth::Password {
                password: self.password.clone(),
            },
            SshAuthType::Key => SshAuth::Key {
                path: PathBuf::from(self.key_path.trim()),
                passphrase: if self.key_passphrase.is_empty() {
                    None
                } else {
                    Some(self.key_passphrase.clone())
                },
            },
        };

        Some(SshService {
            id,
            name: self.name.trim().into(),
            host: self.host.trim().into(),
            port: self.port.trim().parse().ok()?,
            user: self.user.trim().into(),
            auth,
        })
    }
}

#[cfg(test)]
#[path = "app_test.rs"]
mod tests;
