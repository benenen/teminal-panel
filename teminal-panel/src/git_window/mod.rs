mod git_data;
mod theme;

use self::git_data::{get_file_changes, FileChange};
use iced::{Element, Task};
use std::path::PathBuf;
use uuid::Uuid;

pub struct GitWindow {
    project_id: Uuid,
    project_name: String,
    repo_path: PathBuf,
    file_changes: Vec<FileChange>,
    selected_file: Option<PathBuf>,
    error: Option<String>,
}

#[derive(Debug, Clone)]
pub enum Message {
    SelectFile(PathBuf),
    CloseWindow,
}

impl GitWindow {
    pub fn new(project_id: Uuid, project_name: String, repo_path: PathBuf) -> (Self, Task<Message>) {
        let file_changes = match get_file_changes(&repo_path) {
            Ok(changes) => changes,
            Err(e) => {
                return (
                    Self {
                        project_id,
                        project_name,
                        repo_path,
                        file_changes: Vec::new(),
                        selected_file: None,
                        error: Some(format!("Failed to load git data: {}", e)),
                    },
                    Task::none(),
                );
            }
        };

        (
            Self {
                project_id,
                project_name,
                repo_path,
                file_changes,
                selected_file: None,
                error: None,
            },
            Task::none(),
        )
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::SelectFile(path) => {
                self.selected_file = Some(path);
                Task::none()
            }
            Message::CloseWindow => Task::none(),
        }
    }

    pub fn view(&self) -> Element<'_, Message> {
        iced::widget::text("Git Window").into()
    }
}
