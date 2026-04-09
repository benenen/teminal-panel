use crate::config::AppConfig;
use crate::project::{panel::AddProjectForm, Project};
use crate::terminal::{settings_for_working_dir, TerminalState};
use iced::widget::{button, column, container, row, scrollable, text};
use iced::{Element, Length, Task, Theme};
use std::collections::HashMap;
use std::path::PathBuf;
use teminal_ui::components::{Button, TextInput};
use teminal_ui::containers::Modal;
use uuid::Uuid;

pub struct App {
    pub config: AppConfig,
    pub selected_project: Option<Uuid>,
    pub add_form: AddProjectForm,
    pub terminals: HashMap<Uuid, TerminalState>,
    pub next_terminal_id: u64,
}

#[derive(Debug, Clone)]
pub enum Message {
    SelectProject(Uuid),
    AddProject { name: String, working_dir: String },
    RemoveProject(Uuid),
    ProjectStatusChanged(Uuid, String),
    ShowAddProjectForm,
    HideAddProjectForm,
    FormNameChanged(String),
    FormDirChanged(String),
    ChooseProjectFolder,
    ProjectFolderSelected(Option<PathBuf>),
    SubmitAddProjectForm,
    OpenTerminal(Uuid),
    Terminal(iced_term::Event),
}

impl App {
    pub fn new() -> (Self, Task<Message>) {
        (
            Self {
                config: AppConfig::load(),
                selected_project: None,
                add_form: Default::default(),
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
            }
            Message::AddProject { name, working_dir } => {
                self.add_local_project(name, PathBuf::from(working_dir));
            }
            Message::RemoveProject(id) => {
                self.config.projects.retain(|project| project.id != id);
                self.terminals.remove(&id);

                if self.selected_project == Some(id) {
                    self.selected_project = None;
                }

                self.config.save();
            }
            Message::ProjectStatusChanged(_, _) => {}
            Message::ShowAddProjectForm => {
                self.add_form.visible = true;
            }
            Message::HideAddProjectForm => {
                self.add_form = Default::default();
            }
            Message::FormNameChanged(value) => {
                self.add_form.name = value;
            }
            Message::FormDirChanged(_value) => {}
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
                    if self.add_local_project(self.add_form.name.clone(), path) {
                        self.add_form = Default::default();
                    }
                }
            }
            Message::OpenTerminal(project_id) => {
                if self.terminals.contains_key(&project_id) {
                    return Task::none();
                }

                if let Some(project) = self
                    .config
                    .projects
                    .iter()
                    .find(|project| project.id == project_id)
                {
                    let terminal = iced_term::Terminal::new(
                        self.next_terminal_id,
                        settings_for_working_dir(&project.working_dir),
                    );
                    self.next_terminal_id += 1;
                    let widget_id = terminal.widget_id();

                    self.terminals.insert(
                        project_id,
                        TerminalState {
                            id: project_id,
                            project_id,
                            terminal,
                            title: None,
                        },
                    );

                    return iced_term::TerminalView::focus(widget_id);
                }
            }
            Message::Terminal(iced_term::Event::CommandReceived(term_id, cmd)) => {
                let mut closed_project = None;

                if let Some((project_id, terminal_state)) = self
                    .terminals
                    .iter_mut()
                    .find(|(_, terminal)| terminal.terminal.id == term_id)
                {
                    match terminal_state.terminal.update(cmd) {
                        iced_term::actions::Action::Shutdown => {
                            closed_project = Some(*project_id);
                        }
                        iced_term::actions::Action::ChangeTitle(title) => {
                            terminal_state.title = Some(title);
                        }
                        iced_term::actions::Action::Redraw | iced_term::actions::Action::Ignore => {
                        }
                    }
                }

                if let Some(project_id) = closed_project {
                    self.terminals.remove(&project_id);
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

    pub fn view(&self) -> Element<'_, Message> {
        let main_content = row![self.view_project_panel(), self.view_terminal_area()]
            .spacing(16)
            .padding(16);

        if self.add_form.visible {
            let selected_dir = self
                .add_form
                .selected_dir
                .as_ref()
                .map(|path| path.display().to_string())
                .unwrap_or_else(|| "No folder selected".into());

            let form_content = column![
                TextInput::new("Project Name", &self.add_form.name)
                    .on_input(Message::FormNameChanged)
                    .on_submit(Message::SubmitAddProjectForm)
                    .into_element(),
                row![
                    text(selected_dir).size(12),
                    Button::new("Choose Folder")
                        .on_press(Message::ChooseProjectFolder)
                        .into_element(),
                ]
                .spacing(8)
                .align_y(iced::alignment::Vertical::Center),
                row![
                    Button::new("Add")
                        .width(Length::Fill)
                        .on_press(Message::SubmitAddProjectForm)
                        .into_element(),
                    Button::new("Cancel")
                        .width(Length::Fill)
                        .on_press(Message::HideAddProjectForm)
                        .into_element(),
                ]
                .spacing(8),
            ]
            .spacing(16);

            let modal = Modal::new(form_content.into())
                .with_title("Add Project")
                .into_element();

            let overlay = container(modal)
                .width(Length::Fill)
                .height(Length::Fill)
                .style(|_| {
                    container::Style::default()
                        .background(iced::Color::from_rgba(0.0, 0.0, 0.0, 0.5))
                })
                .center_x(Length::Fill)
                .center_y(Length::Fill);

            column![
                container(main_content)
                    .width(Length::Fill)
                    .height(Length::Fill),
                overlay,
            ]
            .into()
        } else {
            main_content.into()
        }
    }

    fn view_project_panel(&self) -> Element<'_, Message> {
        let project_list = self
            .config
            .projects
            .iter()
            .fold(column![], |column, project| {
                let details = column![
                    text(&project.name).size(16),
                    text(project.working_dir.display().to_string()).size(12)
                ]
                .spacing(2)
                .width(Length::Fill);

                column.push(
                    row![
                        button(details)
                            .width(Length::Fill)
                            .on_press(Message::SelectProject(project.id)),
                        button(text("x")).on_press(Message::RemoveProject(project.id)),
                    ]
                    .spacing(6),
                )
            });

        container(
            column![
                text("Projects").size(24),
                scrollable(project_list.spacing(8)).height(Length::Fill),
                Button::new("+ Add Project")
                    .width(Length::Fill)
                    .on_press(Message::ShowAddProjectForm)
                    .into_element(),
            ]
            .spacing(12),
        )
        .width(Length::Fixed(240.0))
        .height(Length::Fill)
        .into()
    }

    fn view_terminal_area(&self) -> Element<'_, Message> {
        let content = if let Some(selected_id) = self.selected_project {
            if let Some(project) = self
                .config
                .projects
                .iter()
                .find(|project| project.id == selected_id)
            {
                if let Some(terminal) = self.terminals.get(&selected_id) {
                    let title = terminal
                        .title
                        .as_ref()
                        .filter(|title| !title.trim().is_empty())
                        .map(|title| format!("Terminal: {} [{}]", project.name, title))
                        .unwrap_or_else(|| format!("Terminal: {}", project.name));

                    column![
                        text(title).size(16),
                        container(
                            iced_term::TerminalView::show(&terminal.terminal)
                                .map(Message::Terminal)
                        )
                        .height(Length::Fill),
                    ]
                    .spacing(8)
                } else {
                    column![
                        text(format!("Project: {}", project.name)).size(24),
                        button(text("Open Terminal")).on_press(Message::OpenTerminal(selected_id)),
                    ]
                    .spacing(8)
                }
            } else {
                column![
                    text("Project not found").size(24),
                    text("Select a project to open a terminal")
                ]
                .spacing(8)
            }
        } else {
            column![
                text("Select a project to open a terminal").size(24),
                text("Terminal area placeholder"),
            ]
            .spacing(8)
        };

        container(content)
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }

    pub fn theme() -> Theme {
        Theme::Dark
    }

    pub fn subscription(&self) -> iced::Subscription<Message> {
        iced::Subscription::batch(self.terminals.values().map(|terminal| {
            iced::Subscription::run_with_id(
                terminal.terminal.id,
                iced_term::Subscription::new(terminal.terminal.id).event_stream(),
            )
            .map(Message::Terminal)
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::{App, Message};
    use crate::config::AppConfig;
    use std::path::PathBuf;
    use std::sync::{Mutex, OnceLock};

    fn env_lock() -> &'static Mutex<()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(()))
    }

    fn test_app() -> App {
        App {
            config: AppConfig::default(),
            selected_project: None,
            add_form: Default::default(),
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

            let terminal = app.terminals.get(&project_id).expect("terminal exists");
            assert_eq!(terminal.project_id, project_id);
            assert_eq!(terminal.terminal.id, 1);
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
}
