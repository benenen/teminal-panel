mod git_data;
mod theme;

use self::git_data::{get_file_changes, FileChange};
use iced::{Element, Length, Task};
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
        use iced::Alignment;
        use iced::widget::{column, container, row, scrollable, text};

        if let Some(error) = &self.error {
            return container(text(error).size(14).color(theme::TEXT_SECONDARY))
                .width(Length::Fill)
                .height(Length::Fill)
                .center_x(Length::Fill)
                .center_y(Length::Fill)
                .into();
        }

        let (unstaged, staged): (Vec<_>, Vec<_>) =
            self.file_changes.iter().partition(|file_change| !file_change.staged);

        let mut content = column![].spacing(16).padding(20);

        if !unstaged.is_empty() {
            let header = text("UNSTAGED CHANGES")
                .size(12)
                .color(theme::TEXT_TERTIARY);

            let mut file_list = column![].spacing(2);
            for file_change in unstaged {
                let status_text = match file_change.status {
                    git_data::FileStatus::Added => "A",
                    git_data::FileStatus::Modified => "M",
                    git_data::FileStatus::Deleted => "D",
                };

                let status_color = match file_change.status {
                    git_data::FileStatus::Added => theme::GIT_ADDED,
                    git_data::FileStatus::Modified => theme::GIT_MODIFIED,
                    git_data::FileStatus::Deleted => theme::GIT_DELETED,
                };

                let file_row = row![
                    text(status_text).size(12).color(status_color),
                    text(file_change.path.display().to_string())
                        .size(13)
                        .color(theme::TEXT_PRIMARY)
                ]
                .spacing(10)
                .align_y(Alignment::Center);

                file_list = file_list.push(file_row);
            }

            content = content.push(header).push(file_list);
        }

        if !staged.is_empty() {
            let header = text("STAGED CHANGES")
                .size(12)
                .color(theme::TEXT_TERTIARY);

            let mut file_list = column![].spacing(2);
            for file_change in staged {
                let status_text = match file_change.status {
                    git_data::FileStatus::Added => "A",
                    git_data::FileStatus::Modified => "M",
                    git_data::FileStatus::Deleted => "D",
                };

                let file_row = row![
                    text(status_text).size(12).color(theme::GIT_ADDED),
                    text(file_change.path.display().to_string())
                        .size(13)
                        .color(theme::TEXT_PRIMARY)
                ]
                .spacing(10)
                .align_y(Alignment::Center);

                file_list = file_list.push(file_row);
            }

            content = content.push(header).push(file_list);
        }

        scrollable(content).into()
    }
}
