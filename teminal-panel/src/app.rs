use crate::config::AppConfig;
use crate::project::{
    panel::{AddProjectForm, ProjectConnectionKind},
    Connection, Project, SshAuth, SshService,
};
use crate::terminal::{settings_for_working_dir, DisplayMode, ProjectTerminals, TerminalState};
use iced::{Element, Task, Theme};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use uuid::Uuid;

#[path = "view/mod.rs"]
mod view;

pub struct App {
    pub config: AppConfig,
    pub selected_project: Option<Uuid>,
    pub hovered_project: Option<Uuid>,
    pub expanded_projects: HashSet<Uuid>,
    pub editing_terminal: Option<(Uuid, usize)>,
    pub add_form: AddProjectForm,
    pub overlay: Option<OverlayState>,
    pub ssh_service_form: SshServiceForm,
    pub editing_ssh_service: Option<Uuid>,
    pub terminals: HashMap<Uuid, ProjectTerminals>,
    pub next_terminal_id: u64,
}

#[derive(Debug, Clone)]
pub enum Message {
    SelectProject(Uuid),
    #[cfg(test)]
    AddProject { name: String, working_dir: String },
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
    OpenTerminal(Uuid),
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
    SettingsMenu,
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
        }
    }
}

impl App {
    pub fn new() -> (Self, Task<Message>) {
        (
            Self {
                config: AppConfig::load(),
                selected_project: None,
                hovered_project: None,
                expanded_projects: HashSet::new(),
                editing_terminal: None,
                add_form: Default::default(),
                overlay: None,
                ssh_service_form: Default::default(),
                editing_ssh_service: None,
                terminals: HashMap::new(),
                next_terminal_id: 1,
            },
            Task::none(),
        )
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::SelectProject(id) => {
                self.selected_project = Some(id);
                self.expanded_projects.insert(id);
            }
            #[cfg(test)]
            Message::AddProject { name, working_dir } => {
                self.add_local_project(name, PathBuf::from(working_dir));
            }
            Message::RemoveProject(id) => {
                self.config.projects.retain(|project| project.id != id);
                self.terminals.remove(&id);

                if self.selected_project == Some(id) {
                    self.selected_project = None;
                }
                if self.hovered_project == Some(id) {
                    self.hovered_project = None;
                }

                self.config.save();
            }
            Message::HoverProject(id) => {
                self.hovered_project = id;
            }
            Message::ShowAddProjectForm => {
                self.overlay = None;
                self.add_form.visible = true;
            }
            Message::HideAddProjectForm => {
                self.add_form = Default::default();
            }
            Message::FormNameChanged(value) => {
                self.add_form.name = value;
            }
            Message::FormConnectionKindChanged(kind) => {
                self.add_form.connection_kind = kind;
                if kind == ProjectConnectionKind::Local {
                    self.add_form.ssh_service_id = None;
                }
            }
            Message::FormSshServiceSelected(service_id) => {
                self.add_form.ssh_service_id = Some(service_id);
            }
            Message::ChooseProjectFolder => {
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
                    }
                }
            }
            Message::ToggleSettingsMenu => {
                self.add_form.visible = false;
                self.overlay = if self.overlay == Some(OverlayState::SettingsMenu) {
                    None
                } else {
                    Some(OverlayState::SettingsMenu)
                };
            }
            Message::ShowSshServices => {
                self.add_form.visible = false;
                self.overlay = Some(OverlayState::SshServices);
                self.editing_ssh_service = None;
                self.ssh_service_form = Default::default();
            }
            Message::HideOverlay => {
                self.overlay = None;
                self.editing_ssh_service = None;
                self.ssh_service_form = Default::default();
            }
            Message::ShowAddSshServiceForm => {
                self.editing_ssh_service = None;
                self.ssh_service_form = Default::default();
            }
            Message::EditSshService(service_id) => {
                if let Some(service) = self.config.ssh_services.iter().find(|s| s.id == service_id) {
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

                self.config.ssh_services.retain(|service| service.id != service_id);
                if self.editing_ssh_service == Some(service_id) {
                    self.editing_ssh_service = None;
                    self.ssh_service_form = Default::default();
                }
                self.config.save();
            }
            Message::SshServiceNameChanged(value) => {
                self.ssh_service_form.name = value;
            }
            Message::SshServiceHostChanged(value) => {
                self.ssh_service_form.host = value;
            }
            Message::SshServicePortChanged(value) => {
                self.ssh_service_form.port = value;
            }
            Message::SshServiceUserChanged(value) => {
                self.ssh_service_form.user = value;
            }
            Message::SshServiceAuthTypeChanged(value) => {
                self.ssh_service_form.auth_type = value;
            }
            Message::SshServicePasswordChanged(value) => {
                self.ssh_service_form.password = value;
            }
            Message::SshServiceKeyPathChanged(value) => {
                self.ssh_service_form.key_path = value;
            }
            Message::SshServiceKeyPassphraseChanged(value) => {
                self.ssh_service_form.key_passphrase = value;
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
                }
            }
            Message::CancelSshServiceForm => {
                self.editing_ssh_service = None;
                self.ssh_service_form = Default::default();
            }
            Message::OpenTerminal(project_id) => {
                if let Some(project) = self.config.projects.iter().find(|p| p.id == project_id) {
                    match iced_term::Terminal::new(
                        self.next_terminal_id,
                        settings_for_working_dir(&project.working_dir),
                    ) {
                        Ok(terminal) => {
                            self.next_terminal_id += 1;
                            let widget_id = terminal.widget_id().clone();

                            let project_name = project.name.clone();
                            let project_terms = self
                                .terminals
                                .entry(project_id)
                                .or_insert_with(ProjectTerminals::new);
                            let term_num = project_terms.terminals.len() + 1;

                            project_terms.terminals.push(TerminalState {
                                terminal,
                                name: format!("{} * {}", project_name, term_num),
                                title: None,
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
            Message::ToggleProjectExpanded(id) => {
                if !self.expanded_projects.remove(&id) {
                    self.expanded_projects.insert(id);
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

        if name.is_empty() || !working_dir.is_dir() {
            return false;
        }

        if !self.config.ssh_services.iter().any(|service| service.id == service_id) {
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

        if self.add_form.visible {
            let overlay = self.view_add_project_overlay();
            iced::widget::column![main_content, overlay].into()
        } else if let Some(overlay) = self.overlay {
            let overlay_view = match overlay {
                OverlayState::SettingsMenu => self.view_settings_menu_overlay(),
                OverlayState::SshServices => self.view_ssh_services_overlay(),
            };
            iced::widget::column![main_content, overlay_view].into()
        } else {
            main_content.into()
        }
    }

    pub fn theme(&self) -> Theme {
        Theme::Dark
    }

    pub fn subscription(&self) -> iced::Subscription<Message> {
        iced::Subscription::batch(
            self.terminals
                .values()
                .flat_map(|pt| pt.terminals.iter())
                .map(|ts| ts.terminal.subscription().map(Message::Terminal)),
        )
    }
}

impl SshServiceForm {
    fn from_service(service: &SshService) -> Self {
        let (auth_type, password, key_path, key_passphrase) = match &service.auth {
            SshAuth::Password(password) => (
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
        }
    }

    fn to_service(&self, id: Uuid) -> Option<SshService> {
        let name = self.name.trim();
        let host = self.host.trim();
        let user = self.user.trim();
        let port = self.port.trim().parse::<u16>().ok()?;

        if name.is_empty() || host.is_empty() || user.is_empty() {
            return None;
        }

        let auth = match self.auth_type {
            SshAuthType::Agent => SshAuth::Agent,
            SshAuthType::Password => {
                if self.password.is_empty() {
                    return None;
                }
                SshAuth::Password(self.password.clone())
            }
            SshAuthType::Key => {
                if self.key_path.trim().is_empty() {
                    return None;
                }
                SshAuth::Key {
                    path: PathBuf::from(self.key_path.trim()),
                    passphrase: if self.key_passphrase.is_empty() {
                        None
                    } else {
                        Some(self.key_passphrase.clone())
                    },
                }
            }
        };

        Some(SshService {
            id,
            name: name.into(),
            host: host.into(),
            port,
            user: user.into(),
            auth,
        })
    }
}

#[cfg(test)]
#[path = "app_test.rs"]
mod tests;
