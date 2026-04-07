use crate::agent::Agent;
use crate::config::AppConfig;
use iced::widget::{button, column, container, row, text};
use iced::{Element, Length, Task, Theme};
use std::path::PathBuf;
use uuid::Uuid;

pub struct App {
    pub config: AppConfig,
    pub selected_agent: Option<Uuid>,
}

#[derive(Debug, Clone)]
pub enum Message {
    SelectAgent(Uuid),
    AddAgent { name: String, working_dir: String },
    RemoveAgent(Uuid),
    AgentStatusChanged(Uuid, String),
}

impl App {
    pub fn new() -> (Self, Task<Message>) {
        let config = AppConfig::load();

        (
            Self {
                config,
                selected_agent: None,
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
                let agent = Agent::new_local(name, PathBuf::from(working_dir));
                self.config.agents.push(agent);
                self.config.save();
            }
            Message::RemoveAgent(id) => {
                self.config.agents.retain(|agent| agent.id != id);

                if self.selected_agent == Some(id) {
                    self.selected_agent = None;
                }

                self.config.save();
            }
            Message::AgentStatusChanged(_, _) => {}
        }

        Task::none()
    }

    pub fn view(&self) -> Element<'_, Message> {
        row![self.view_agent_panel(), self.view_terminal_area()]
            .spacing(16)
            .padding(16)
            .into()
    }

    fn view_agent_panel(&self) -> Element<'_, Message> {
        let mut agent_list = column![text("Agents").size(24)].spacing(8).width(220);

        for agent in &self.config.agents {
            agent_list = agent_list.push(
                button(text(agent.name.clone()))
                    .width(Length::Fill)
                    .on_press(Message::SelectAgent(agent.id)),
            );
        }

        container(agent_list)
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
