mod git_data;
mod graph;
mod theme;

use self::git_data::{get_commit_history, get_file_changes, CommitNode, FileChange};
use iced::{Element, Font, Length, Task};
use std::path::PathBuf;
use uuid::Uuid;

pub struct GitWindow {
    project_id: Uuid,
    project_name: String,
    repo_path: PathBuf,
    file_changes: Vec<FileChange>,
    commit_history: Vec<CommitNode>,
    selected_file: Option<PathBuf>,
    selected_diff: Option<String>,
    error: Option<String>,
}

#[derive(Debug, Clone)]
pub enum Message {
    SelectFile(PathBuf),
    CloseWindow,
}

impl GitWindow {
    pub fn new(
        project_id: Uuid,
        project_name: String,
        repo_path: PathBuf,
    ) -> (Self, Task<Message>) {
        let file_changes = match get_file_changes(&repo_path) {
            Ok(changes) => changes,
            Err(e) => {
                return (
                    Self {
                        project_id,
                        project_name,
                        repo_path,
                        file_changes: Vec::new(),
                        commit_history: Vec::new(),
                        selected_file: None,
                        selected_diff: None,
                        error: Some(format!("Failed to load git data: {}", e)),
                    },
                    Task::none(),
                );
            }
        };

        let commit_history = match get_commit_history(&repo_path, 150) {
            Ok(commits) => commits,
            Err(e) => {
                return (
                    Self {
                        project_id,
                        project_name,
                        repo_path,
                        file_changes,
                        commit_history: Vec::new(),
                        selected_file: None,
                        selected_diff: None,
                        error: Some(format!("Failed to load git history: {}", e)),
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
                commit_history,
                selected_file: None,
                selected_diff: None,
                error: None,
            },
            Task::none(),
        )
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::SelectFile(path) => {
                self.selected_diff = git_data::get_file_diff(&self.repo_path, &path).ok();
                self.selected_file = Some(path);
                Task::none()
            }
            Message::CloseWindow => Task::none(),
        }
    }

    pub fn view(&self) -> Element<'_, Message> {
        use iced::widget::{button, column, container, row, scrollable, text};
        use iced::Alignment;

        if let Some(error) = &self.error {
            return container(text(error).size(14).color(theme::TEXT_SECONDARY))
                .width(Length::Fill)
                .height(Length::Fill)
                .center_x(Length::Fill)
                .center_y(Length::Fill)
                .into();
        }

        let (unstaged, staged): (Vec<_>, Vec<_>) = self
            .file_changes
            .iter()
            .partition(|file_change| !file_change.staged);

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

                let file_path = file_change.path.clone();
                let is_selected = self.selected_file.as_ref() == Some(&file_path);
                let file_row = button(
                    row![
                        text(status_text).size(12).color(status_color),
                        text(file_path.display().to_string())
                            .size(13)
                            .color(theme::TEXT_PRIMARY)
                    ]
                    .spacing(10)
                    .align_y(Alignment::Center),
                )
                .on_press(Message::SelectFile(file_path))
                .padding([6, 8])
                .width(Length::Fill)
                .style(move |theme_value, status| {
                    let mut style = button::text(theme_value, status);
                    if is_selected {
                        style.background = Some(theme::BG_TERTIARY.into());
                    }
                    style
                });

                file_list = file_list.push(file_row);
            }

            content = content.push(header).push(file_list);
        }

        if !staged.is_empty() {
            let header = text("STAGED CHANGES").size(12).color(theme::TEXT_TERTIARY);

            let mut file_list = column![].spacing(2);
            for file_change in staged {
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

                let file_path = file_change.path.clone();
                let is_selected = self.selected_file.as_ref() == Some(&file_path);
                let file_row = button(
                    row![
                        text(status_text).size(12).color(status_color),
                        text(file_path.display().to_string())
                            .size(13)
                            .color(theme::TEXT_PRIMARY)
                    ]
                    .spacing(10)
                    .align_y(Alignment::Center),
                )
                .on_press(Message::SelectFile(file_path))
                .padding([6, 8])
                .width(Length::Fill)
                .style(move |theme_value, status| {
                    let mut style = button::text(theme_value, status);
                    if is_selected {
                        style.background = Some(theme::BG_TERTIARY.into());
                    }
                    style
                });

                file_list = file_list.push(file_row);
            }

            content = content.push(header).push(file_list);
        }

        let detail_pane: Element<'_, Message> = if let (Some(selected_file), Some(selected_diff)) =
            (&self.selected_file, &self.selected_diff)
        {
            container(
                column![
                    text(selected_file.display().to_string())
                        .size(14)
                        .color(theme::TEXT_PRIMARY),
                    scrollable(
                        text(selected_diff.clone())
                            .size(12)
                            .font(Font::MONOSPACE)
                            .color(theme::TEXT_SECONDARY)
                    )
                ]
                .spacing(12)
                .padding(20),
            )
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
        } else {
            graph::view_commit_graph(&self.commit_history)
        };

        row![
            container(scrollable(content))
                .width(Length::Fixed(300.0))
                .height(Length::Fill),
            detail_pane,
        ]
        .height(Length::Fill)
        .into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use git2::{IndexAddOption, Repository, Signature};

    fn with_temp_repo<T>(f: impl FnOnce(&std::path::Path, &Repository) -> T) -> T {
        let temp_dir =
            std::env::temp_dir().join(format!("teminal-panel-git-window-{}", Uuid::new_v4()));
        std::fs::create_dir_all(&temp_dir).expect("create temp repo dir");

        let repo = Repository::init(&temp_dir).expect("init repo");
        let result = f(&temp_dir, &repo);

        let _ = std::fs::remove_dir_all(&temp_dir);
        result
    }

    fn commit_file(
        repo: &Repository,
        repo_path: &std::path::Path,
        file_name: &str,
        contents: &str,
    ) {
        std::fs::write(repo_path.join(file_name), contents).expect("write repo file");

        let mut index = repo.index().expect("open index");
        index
            .add_all([file_name], IndexAddOption::DEFAULT, None)
            .expect("stage file");
        index.write().expect("write index");

        let tree_id = index.write_tree().expect("write tree");
        let tree = repo.find_tree(tree_id).expect("find tree");
        let signature = Signature::now("Test User", "test@example.com").expect("signature");

        repo.commit(
            Some("HEAD"),
            &signature,
            &signature,
            "Initial commit",
            &tree,
            &[],
        )
        .expect("create commit");
    }

    #[test]
    fn selecting_file_loads_diff_text_for_detail_pane() {
        with_temp_repo(|repo_path, repo| {
            commit_file(repo, repo_path, "README.md", "# test\n");
            std::fs::write(repo_path.join("README.md"), "# test\nnew line\n")
                .expect("update repo file");

            let (mut git_window, _) =
                GitWindow::new(Uuid::new_v4(), "repo".into(), repo_path.to_path_buf());

            let _ = git_window.update(Message::SelectFile(PathBuf::from("README.md")));

            assert_eq!(git_window.selected_file, Some(PathBuf::from("README.md")));
            assert!(git_window
                .selected_diff
                .as_deref()
                .is_some_and(|diff| diff.contains("+new line")));
        });
    }
}
