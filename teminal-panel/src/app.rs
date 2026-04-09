use crate::config::AppConfig;
use crate::project::{panel::AddProjectForm, Project};
use crate::terminal::{settings_for_working_dir, DisplayMode, ProjectTerminals, TerminalState};
use iced::widget::{button, column, container, row, scrollable, text};
use iced::{Element, Length, Task, Theme};
use iced_fonts::bootstrap;
use std::collections::HashMap;
use std::path::PathBuf;
use teminal_ui::components::{Button, TextInput};
use teminal_ui::containers::Modal;
use uuid::Uuid;

pub struct App {
    pub config: AppConfig,
    pub selected_project: Option<Uuid>,
    pub add_form: AddProjectForm,
    pub terminals: HashMap<Uuid, ProjectTerminals>,
    pub next_terminal_id: u64,
}

#[derive(Debug, Clone)]
pub enum Message {
    SelectProject(Uuid),
    AddProject { name: String, working_dir: String },
    RemoveProject(Uuid),
    ShowAddProjectForm,
    HideAddProjectForm,
    FormNameChanged(String),
    ChooseProjectFolder,
    ProjectFolderSelected(Option<PathBuf>),
    SubmitAddProjectForm,
    OpenTerminal(Uuid),
    SelectTab(Uuid, usize),
    CloseTab(Uuid, usize),
    ToggleDisplayMode(Uuid),
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
            Message::ShowAddProjectForm => {
                self.add_form.visible = true;
            }
            Message::HideAddProjectForm => {
                self.add_form = Default::default();
            }
            Message::FormNameChanged(value) => {
                self.add_form.name = value;
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
                    if self.add_local_project(self.add_form.name.clone(), path) {
                        self.add_form = Default::default();
                    }
                }
            }
            Message::OpenTerminal(project_id) => {
                if let Some(project) = self
                    .config
                    .projects
                    .iter()
                    .find(|p| p.id == project_id)
                {
                    match iced_term::Terminal::new(
                        self.next_terminal_id,
                        settings_for_working_dir(&project.working_dir),
                    ) {
                        Ok(terminal) => {
                            self.next_terminal_id += 1;
                            let widget_id = terminal.widget_id().clone();

                            let project_terms = self
                                .terminals
                                .entry(project_id)
                                .or_insert_with(ProjectTerminals::new);

                            project_terms.terminals.push(TerminalState {
                                terminal,
                                title: None,
                            });
                            project_terms.active_index =
                                project_terms.terminals.len() - 1;

                            return iced_term::TerminalView::focus(widget_id);
                        }
                        Err(e) => {
                            eprintln!("Failed to create terminal: {e}");
                        }
                    }
                }
            }
            Message::SelectTab(project_id, index) => {
                if let Some(project_terms) = self.terminals.get_mut(&project_id) {
                    if index < project_terms.terminals.len() {
                        project_terms.active_index = index;
                        let widget_id = project_terms.terminals[index]
                            .terminal
                            .widget_id()
                            .clone();
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
                        match ts
                            .terminal
                            .handle(iced_term::Command::ProxyToBackend(cmd))
                        {
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

    pub fn view(&self) -> Element<'_, Message> {
        let main_content = row![self.view_project_panel(), self.view_terminal_area()]
            .spacing(0)
            .padding(0);

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
                    button(bootstrap::folder_plus().size(14))
                        .on_press(Message::ChooseProjectFolder)
                        .padding([4, 8])
                        .style(button::secondary),
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
            .fold(column![], |col, project| {
                let is_selected = self.selected_project == Some(project.id);

                let term_count = self
                    .terminals
                    .get(&project.id)
                    .map(|pt| pt.terminals.len())
                    .unwrap_or(0);

                let name_row = row![
                    bootstrap::folder().size(14),
                    text(&project.name).size(13),
                ]
                .spacing(8)
                .align_y(iced::alignment::Vertical::Center);

                let path_text = text(project.working_dir.display().to_string())
                    .size(11)
                    .color(iced::Color::from_rgb(0.5, 0.5, 0.5));

                let mut details = column![name_row, path_text].spacing(2).width(Length::Fill);

                if term_count > 0 {
                    details = details.push(
                        row![
                            bootstrap::terminal().size(11),
                            text(format!("{term_count}")).size(11),
                        ]
                        .spacing(4)
                        .align_y(iced::alignment::Vertical::Center),
                    );
                }

                let item_style = if is_selected {
                    button::primary
                } else {
                    button::secondary
                };

                col.push(
                    row![
                        button(details)
                            .width(Length::Fill)
                            .style(item_style)
                            .padding([8, 10])
                            .on_press(Message::SelectProject(project.id)),
                        button(bootstrap::x_lg().size(12))
                            .on_press(Message::RemoveProject(project.id))
                            .padding([8, 8])
                            .style(button::text),
                    ]
                    .spacing(2),
                )
            });

        let add_btn = button(
            row![
                bootstrap::plus_lg().size(14),
                text("New Project").size(13),
            ]
            .spacing(6)
            .align_y(iced::alignment::Vertical::Center),
        )
        .width(Length::Fill)
        .padding([8, 10])
        .style(button::secondary)
        .on_press(Message::ShowAddProjectForm);

        container(
            column![
                container(
                    text("Projects")
                        .size(13)
                        .color(iced::Color::from_rgb(0.6, 0.6, 0.6)),
                )
                .padding([12, 10]),
                scrollable(project_list.spacing(2).padding([0, 6])).height(Length::Fill),
                container(add_btn).padding([6, 6]),
            ]
            .spacing(0),
        )
        .width(Length::Fixed(220.0))
        .height(Length::Fill)
        .style(|_| {
            container::Style::default()
                .background(iced::Color::from_rgb(0.1, 0.1, 0.1))
                .border(iced::Border {
                    color: iced::Color::from_rgb(0.18, 0.18, 0.18),
                    width: 1.0,
                    radius: 0.into(),
                })
        })
        .into()
    }

    fn view_terminal_area(&self) -> Element<'_, Message> {
        let content = if let Some(selected_id) = self.selected_project {
            if let Some(project) = self
                .config
                .projects
                .iter()
                .find(|p| p.id == selected_id)
            {
                if let Some(project_terms) = self.terminals.get(&selected_id) {
                    self.view_terminals(selected_id, &project.name, project_terms)
                } else {
                    self.view_empty_project(selected_id, &project.name)
                }
            } else {
                column![text("Project not found").size(14)].into()
            }
        } else {
            container(
                column![
                    bootstrap::terminal_fill().size(48).color(iced::Color::from_rgb(0.25, 0.25, 0.25)),
                    text("Select a project")
                        .size(14)
                        .color(iced::Color::from_rgb(0.4, 0.4, 0.4)),
                ]
                .spacing(12)
                .align_x(iced::alignment::Horizontal::Center),
            )
            .width(Length::Fill)
            .height(Length::Fill)
            .center_x(Length::Fill)
            .center_y(Length::Fill)
            .into()
        };

        container(content)
            .width(Length::Fill)
            .height(Length::Fill)
            .style(|_| {
                container::Style::default()
                    .background(iced::Color::from_rgb(0.08, 0.08, 0.08))
            })
            .into()
    }

    fn view_empty_project<'a>(&self, project_id: Uuid, name: &str) -> Element<'a, Message> {
        container(
            column![
                bootstrap::terminal_plus()
                    .size(48)
                    .color(iced::Color::from_rgb(0.3, 0.3, 0.3)),
                text(name.to_string()).size(16),
                button(
                    row![
                        bootstrap::terminal_plus().size(14),
                        text("Open Terminal").size(13),
                    ]
                    .spacing(6)
                    .align_y(iced::alignment::Vertical::Center),
                )
                .on_press(Message::OpenTerminal(project_id))
                .padding([8, 16])
                .style(button::primary),
            ]
            .spacing(12)
            .align_x(iced::alignment::Horizontal::Center),
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .center_x(Length::Fill)
        .center_y(Length::Fill)
        .into()
    }

    fn view_terminals<'a>(
        &'a self,
        project_id: Uuid,
        _project_name: &str,
        project_terms: &'a ProjectTerminals,
    ) -> Element<'a, Message> {
        let tab_bar = self.view_tab_bar(project_id, project_terms);

        let terminal_content: Element<'_, Message> = match project_terms.display_mode {
            DisplayMode::Tabs => {
                if let Some(ts) = project_terms.active_terminal() {
                    iced_term::TerminalView::show(&ts.terminal).map(Message::Terminal)
                } else {
                    text("No terminal").into()
                }
            }
            DisplayMode::Panel => {
                let panels =
                    project_terms
                        .terminals
                        .iter()
                        .fold(row![], |r, ts| {
                            r.push(
                                container(
                                    iced_term::TerminalView::show(&ts.terminal)
                                        .map(Message::Terminal),
                                )
                                .width(Length::Fill)
                                .height(Length::Fill),
                            )
                        })
                        .spacing(1);
                panels.into()
            }
        };

        column![
            tab_bar,
            container(terminal_content)
                .width(Length::Fill)
                .height(Length::Fill),
        ]
        .spacing(0)
        .into()
    }

    fn view_tab_bar<'a>(
        &self,
        project_id: Uuid,
        project_terms: &ProjectTerminals,
    ) -> Element<'a, Message> {
        let mut tabs = row![].spacing(1);

        for (i, ts) in project_terms.terminals.iter().enumerate() {
            let is_active = i == project_terms.active_index;
            let label = ts
                .title
                .as_ref()
                .filter(|t| !t.trim().is_empty())
                .cloned()
                .unwrap_or_else(|| format!("Terminal {}", i + 1));

            let tab_label = row![
                bootstrap::terminal().size(12),
                text(label).size(12),
                button(bootstrap::x_lg().size(10))
                    .on_press(Message::CloseTab(project_id, i))
                    .padding([2, 4])
                    .style(button::text),
            ]
            .spacing(6)
            .align_y(iced::alignment::Vertical::Center);

            let tab_style = if is_active {
                button::primary
            } else {
                button::secondary
            };

            tabs = tabs.push(
                button(tab_label)
                    .on_press(Message::SelectTab(project_id, i))
                    .padding([6, 12])
                    .style(tab_style),
            );
        }

        let add_tab = button(bootstrap::plus_lg().size(12))
            .on_press(Message::OpenTerminal(project_id))
            .padding([6, 8])
            .style(button::text);

        let mode_icon = match project_terms.display_mode {
            DisplayMode::Tabs => bootstrap::layout_split().size(14),
            DisplayMode::Panel => bootstrap::layout_text_window().size(14),
        };
        let mode_btn = button(mode_icon)
            .on_press(Message::ToggleDisplayMode(project_id))
            .padding([6, 8])
            .style(button::text);

        container(
            row![
                scrollable(tabs).direction(scrollable::Direction::Horizontal(
                    scrollable::Scrollbar::default(),
                )),
                add_tab,
                mode_btn,
            ]
            .spacing(4)
            .align_y(iced::alignment::Vertical::Center),
        )
        .width(Length::Fill)
        .padding([2, 4])
        .style(|_| {
            container::Style::default()
                .background(iced::Color::from_rgb(0.12, 0.12, 0.12))
                .border(iced::Border {
                    color: iced::Color::from_rgb(0.18, 0.18, 0.18),
                    width: 1.0,
                    radius: 0.into(),
                })
        })
        .into()
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
