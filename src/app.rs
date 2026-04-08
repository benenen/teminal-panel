use crate::agent::{panel::AddAgentForm, Agent};
use crate::config::AppConfig;
use iced::widget::{button, column, container, row, scrollable, text, text_input};
use iced::{Element, Length, Task, Theme};
use std::path::PathBuf;
use uuid::Uuid;

pub struct App {
    pub config: AppConfig,
    pub selected_agent: Option<Uuid>,
    pub add_form: AddAgentForm,
}

#[derive(Debug, Clone)]
pub enum Message {
    SelectAgent(Uuid),
    AddAgent { name: String, working_dir: String },
    RemoveAgent(Uuid),
    AgentStatusChanged(Uuid, String),
    ShowAddForm,
    HideAddForm,
    FormNameChanged(String),
    FormDirChanged(String),
    SubmitAddForm,
}

impl App {
    pub fn new() -> (Self, Task<Message>) {
        let config = AppConfig::load();

        (
            Self {
                config,
                selected_agent: None,
                add_form: Default::default(),
            },
            Task::none(),
        )
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::SelectAgent(id) => {
                self.selected_agent = Some(id);
            }
            Message::AddAgent { name, working_dir } => {
                self.add_local_agent(name, working_dir);
            }
            Message::RemoveAgent(id) => {
                self.config.agents.retain(|agent| agent.id != id);

                if self.selected_agent == Some(id) {
                    self.selected_agent = None;
                }

                self.config.save();
            }
            Message::AgentStatusChanged(_, _) => {}
            Message::ShowAddForm => {
                self.add_form.visible = true;
            }
            Message::HideAddForm => {
                self.add_form = Default::default();
            }
            Message::FormNameChanged(value) => {
                self.add_form.name = value;
            }
            Message::FormDirChanged(value) => {
                self.add_form.working_dir = value;
            }
            Message::SubmitAddForm => {
                if self.add_local_agent(
                    self.add_form.name.clone(),
                    self.add_form.working_dir.clone(),
                ) {
                    self.add_form = Default::default();
                }
            }
        }

        Task::none()
    }

    fn add_local_agent(&mut self, name: String, working_dir: String) -> bool {
        let name = name.trim().to_string();
        let working_dir = working_dir.trim().to_string();

        if name.is_empty() || working_dir.is_empty() {
            return false;
        }

        let working_dir = PathBuf::from(working_dir);
        if !working_dir.is_dir() {
            return false;
        }

        self.config.agents.push(Agent::new_local(name, working_dir));
        self.config.save();
        true
    }

    pub fn view(&self) -> Element<'_, Message> {
        row![self.view_agent_panel(), self.view_terminal_area()]
            .spacing(16)
            .padding(16)
            .into()
    }

    fn view_agent_panel(&self) -> Element<'_, Message> {
        let agent_list = self.config.agents.iter().fold(column![], |column, agent| {
            let details = column![
                text(&agent.name).size(16),
                text(agent.working_dir.display().to_string()).size(12)
            ]
            .spacing(2)
            .width(Length::Fill);

            column.push(
                row![
                    button(details)
                        .width(Length::Fill)
                        .on_press(Message::SelectAgent(agent.id)),
                    button(text("x")).on_press(Message::RemoveAgent(agent.id)),
                ]
                .spacing(6),
            )
        });

        let add_section: Element<'_, Message> = if self.add_form.visible {
            column![
                text_input("Name", &self.add_form.name)
                    .on_input(Message::FormNameChanged)
                    .on_submit(Message::SubmitAddForm),
                text_input("Directory", &self.add_form.working_dir)
                    .on_input(Message::FormDirChanged)
                    .on_submit(Message::SubmitAddForm),
                row![
                    button(text("Add")).on_press(Message::SubmitAddForm),
                    button(text("Cancel")).on_press(Message::HideAddForm),
                ]
                .spacing(6),
            ]
            .spacing(6)
            .into()
        } else {
            button(text("+ Add Agent"))
                .on_press(Message::ShowAddForm)
                .into()
        };

        container(
            column![
                text("Agents").size(24),
                scrollable(agent_list.spacing(8)).height(Length::Fill),
                add_section,
            ]
            .spacing(12),
        )
            .width(Length::Fixed(240.0))
            .height(Length::Fill)
            .into()
    }

    fn view_terminal_area(&self) -> Element<'_, Message> {
        let content = if let Some(agent) = self
            .selected_agent
            .and_then(|selected| self.config.agents.iter().find(|agent| agent.id == selected))
        {
            column![
                text(format!("Selected agent: {}", agent.name)).size(24),
                text("Terminal area placeholder")
            ]
            .spacing(8)
        } else {
            column![
                text("Select an agent to open a terminal").size(24),
                text("Terminal area placeholder")
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
            selected_agent: None,
            add_form: Default::default(),
        }
    }

    fn with_temp_config_dir<T>(f: impl FnOnce(&PathBuf) -> T) -> T {
        let _guard = env_lock().lock().expect("test env lock");
        let temp_root = std::env::temp_dir().join(format!("teminal-panel-tests-{}", uuid::Uuid::new_v4()));
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
    fn show_and_hide_add_form_updates_visibility_and_resets_fields() {
        let mut app = test_app();

        let _ = app.update(Message::ShowAddForm);
        assert!(app.add_form.visible);

        let _ = app.update(Message::FormNameChanged("Local agent".into()));
        let _ = app.update(Message::FormDirChanged("/tmp/work".into()));
        assert_eq!(app.add_form.name, "Local agent");
        assert_eq!(app.add_form.working_dir, "/tmp/work");

        let _ = app.update(Message::HideAddForm);
        assert!(!app.add_form.visible);
        assert!(app.add_form.name.is_empty());
        assert!(app.add_form.working_dir.is_empty());
    }

    #[test]
    fn submit_add_form_adds_agent_and_resets_form() {
        with_temp_config_dir(|workspace_dir: &PathBuf| {
            let mut app = test_app();
            let working_dir = workspace_dir.display().to_string();

            let _ = app.update(Message::ShowAddForm);
            let _ = app.update(Message::FormNameChanged("Local agent".into()));
            let _ = app.update(Message::FormDirChanged(working_dir.clone()));
            let _ = app.update(Message::SubmitAddForm);

            assert_eq!(app.config.agents.len(), 1);
            assert_eq!(app.config.agents[0].name, "Local agent");
            assert_eq!(app.config.agents[0].working_dir, workspace_dir.clone());
            assert!(!app.add_form.visible);
            assert!(app.add_form.name.is_empty());
            assert!(app.add_form.working_dir.is_empty());

            let persisted = AppConfig::load();
            assert_eq!(persisted.agents.len(), 1);
            assert_eq!(persisted.agents[0].name, "Local agent");
        });
    }

    #[test]
    fn submit_add_form_requires_valid_directory() {
        with_temp_config_dir(|_| {
            let mut app = test_app();

            let _ = app.update(Message::ShowAddForm);
            let _ = app.update(Message::FormNameChanged("Local agent".into()));
            let _ = app.update(Message::FormDirChanged("/tmp/missing-directory".into()));
            let _ = app.update(Message::SubmitAddForm);

            assert!(app.config.agents.is_empty());
            assert!(app.add_form.visible);
            assert!(AppConfig::load().agents.is_empty());
        });
    }

    #[test]
    fn removing_selected_agent_clears_selection() {
        with_temp_config_dir(|workspace_dir: &PathBuf| {
            let mut app = test_app();

            let _ = app.update(Message::AddAgent {
                name: "Local agent".into(),
                working_dir: workspace_dir.display().to_string(),
            });

            let agent_id = app.config.agents[0].id;
            let _ = app.update(Message::SelectAgent(agent_id));
            assert_eq!(app.selected_agent, Some(agent_id));

            let _ = app.update(Message::RemoveAgent(agent_id));
            assert!(app.config.agents.is_empty());
            assert_eq!(app.selected_agent, None);
            assert!(AppConfig::load().agents.is_empty());
        });
    }
}
