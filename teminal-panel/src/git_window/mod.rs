mod detail;
mod git_data;
mod graph;
mod theme;

use self::git_data::{
    classify_file_content, get_base_file_content, get_commit_history, get_file_changes,
    get_worktree_file_content, CommitNode, FileChange, FileContentKind,
};
use iced::{widget::text_editor, Element, Length, Task};
use std::path::{Path, PathBuf};
use uuid::Uuid;

pub struct GitWindow {
    project_id: Uuid,
    project_name: String,
    repo_path: PathBuf,
    file_changes: Vec<FileChange>,
    commit_history: Vec<CommitNode>,
    selected_detail: Option<SelectedFileDetail>,
    error: Option<String>,
}

struct SelectedFileDetail {
    path: PathBuf,
    status: git_data::FileStatus,
    staged: bool,
    content_kind: FileContentKind,
    base_text: Option<String>,
    worktree_text: Option<String>,
    draft: Option<text_editor::Content>,
    dirty: bool,
    diff: Option<String>,
    detail_error: Option<String>,
}

impl SelectedFileDetail {
    fn error(selection: &FileSelection, message: String) -> Self {
        Self {
            path: selection.path.clone(),
            status: selection.status.clone(),
            staged: selection.staged,
            content_kind: FileContentKind::Text,
            base_text: None,
            worktree_text: None,
            draft: None,
            dirty: false,
            diff: None,
            detail_error: Some(message),
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct FileSelection {
    path: PathBuf,
    status: git_data::FileStatus,
    staged: bool,
}

#[derive(Debug, Clone)]
pub enum Message {
    SelectFile(FileSelection),
    EditSelectedFile(text_editor::Action),
    ApplySelectedFile,
    DiscardSelectedFile,
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
                        selected_detail: None,
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
                        selected_detail: None,
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
                selected_detail: None,
                error: None,
            },
            Task::none(),
        )
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::SelectFile(selection) => {
                self.selected_detail =
                    Some(load_selected_file_detail(&self.repo_path, &selection));
                Task::none()
            }
            Message::EditSelectedFile(action) => {
                if let Some(detail) = self.selected_detail.as_mut() {
                    if let Some(draft) = detail.draft.as_mut() {
                        draft.perform(action);
                        detail.dirty = detail
                            .worktree_text
                            .as_deref()
                            .is_some_and(|worktree_text| draft.text() != worktree_text);
                    }
                }
                Task::none()
            }
            Message::ApplySelectedFile => {
                Task::none()
            }
            Message::DiscardSelectedFile => {
                Task::none()
            }
            Message::CloseWindow => Task::none(),
        }
    }

    pub fn view(&self) -> Element<'_, Message> {
        use iced::widget::{button, column, container, row, scrollable, text};
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
                let is_selected = self
                    .selected_detail
                    .as_ref()
                    .is_some_and(|detail| detail.path == file_path);
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
                .on_press(Message::SelectFile(FileSelection {
                    path: file_path,
                    status: file_change.status.clone(),
                    staged: file_change.staged,
                }))
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
                let is_selected = self
                    .selected_detail
                    .as_ref()
                    .is_some_and(|detail| detail.path == file_path);
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
                .on_press(Message::SelectFile(FileSelection {
                    path: file_path,
                    status: file_change.status.clone(),
                    staged: file_change.staged,
                }))
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

        let detail_pane: Element<'_, Message> = if let Some(detail) = &self.selected_detail {
            detail::view_selected_detail(detail)
        } else {
            graph::view_commit_graph(&self.commit_history)
        };

        row![
            container(scrollable(content))
                .width(Length::Fixed(300.0))
                .height(Length::Fill),
            detail_pane,
            detail_pane,
        ]
        .height(Length::Fill)
        .into()
    }
}

fn load_selected_file_detail(repo_path: &Path, selection: &FileSelection) -> SelectedFileDetail {
    let base_bytes = match get_base_file_content(repo_path, &selection.path) {
        Ok(bytes) => bytes,
        Err(error) => {
            return SelectedFileDetail::error(
                selection,
                format!("Failed to load base content: {error}"),
            );
        }
    };

    let worktree_bytes = match get_worktree_file_content(repo_path, &selection.path) {
        Ok(bytes) => Some(bytes),
        Err(error)
            if selection.status == git_data::FileStatus::Deleted
                && error.kind() == std::io::ErrorKind::NotFound =>
        {
            None
        }
        Err(error) => {
            return SelectedFileDetail::error(
                selection,
                format!("Failed to load working tree content: {error}"),
            );
        }
    };

    let diff = git_data::get_file_diff(repo_path, &selection.path).ok();
    let content_kind = if base_bytes
        .as_deref()
        .is_some_and(|bytes| classify_file_content(bytes) == FileContentKind::Binary)
        || worktree_bytes
            .as_deref()
            .is_some_and(|bytes| classify_file_content(bytes) == FileContentKind::Binary)
    {
        FileContentKind::Binary
    } else {
        FileContentKind::Text
    };

    if content_kind == FileContentKind::Binary {
        return SelectedFileDetail {
            path: selection.path.clone(),
            status: selection.status.clone(),
            staged: selection.staged,
            content_kind,
            base_text: None,
            worktree_text: None,
            draft: None,
            dirty: false,
            diff,
            detail_error: None,
        };
    }

    let base_text = base_bytes
        .map(|bytes| String::from_utf8(bytes).unwrap_or_default())
        .unwrap_or_default();
    let worktree_text = worktree_bytes
        .map(|bytes| String::from_utf8(bytes).unwrap_or_default())
        .unwrap_or_default();

    SelectedFileDetail {
        path: selection.path.clone(),
        status: selection.status.clone(),
        staged: selection.staged,
        content_kind,
        base_text: Some(base_text),
        worktree_text: Some(worktree_text.clone()),
        draft: Some(text_editor::Content::with_text(&worktree_text)),
        dirty: false,
        diff,
        detail_error: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use git2::{IndexAddOption, Repository, Signature};
    use iced::widget::text_editor;

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
    fn selecting_text_file_initializes_compare_editor_state() {
        with_temp_repo(|repo_path, repo| {
            commit_file(repo, repo_path, "README.md", "# test\n");
            std::fs::write(repo_path.join("README.md"), "# test\nnew line\n")
                .expect("update repo file");

            let (mut git_window, _) =
                GitWindow::new(Uuid::new_v4(), "repo".into(), repo_path.to_path_buf());

            let _ = git_window.update(Message::SelectFile(FileSelection {
                path: PathBuf::from("README.md"),
                status: git_data::FileStatus::Modified,
                staged: false,
            }));

            let detail = git_window
                .selected_detail
                .as_ref()
                .expect("selected file detail");

            assert_eq!(detail.path, PathBuf::from("README.md"));
            assert_eq!(detail.status, git_data::FileStatus::Modified);
            assert!(!detail.staged);
            assert_eq!(detail.content_kind, git_data::FileContentKind::Text);
            assert_eq!(detail.base_text.as_deref(), Some("# test\n"));
            assert_eq!(detail.worktree_text.as_deref(), Some("# test\nnew line\n"));
            assert_eq!(
                detail.draft.as_ref().map(text_editor::Content::text).as_deref(),
                Some("# test\nnew line\n")
            );
            assert!(!detail.dirty);
            assert!(detail.detail_error.is_none());
        });
    }

    #[test]
    fn selecting_binary_file_enters_non_editable_detail_state() {
        with_temp_repo(|repo_path, repo| {
            std::fs::write(repo_path.join("image.bin"), [0_u8, 159, 146, 150])
                .expect("write binary repo file");

            let mut index = repo.index().expect("open index");
            index
                .add_path(std::path::Path::new("image.bin"))
                .expect("stage binary file");
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

            std::fs::write(repo_path.join("image.bin"), [0_u8, 159, 146, 151])
                .expect("update binary repo file");

            let (mut git_window, _) =
                GitWindow::new(Uuid::new_v4(), "repo".into(), repo_path.to_path_buf());

            let _ = git_window.update(Message::SelectFile(FileSelection {
                path: PathBuf::from("image.bin"),
                status: git_data::FileStatus::Modified,
                staged: false,
            }));

            let detail = git_window
                .selected_detail
                .as_ref()
                .expect("selected binary detail");

            assert_eq!(detail.path, PathBuf::from("image.bin"));
            assert_eq!(detail.status, git_data::FileStatus::Modified);
            assert!(!detail.staged);
            assert_eq!(detail.content_kind, git_data::FileContentKind::Binary);
            assert!(detail.base_text.is_none());
            assert!(detail.worktree_text.is_none());
            assert!(detail.draft.is_none());
            assert!(!detail.dirty);
            assert!(detail.detail_error.is_none());
        });
    }

    #[test]
    fn git_window_detail_edit_marks_selected_text_file_dirty() {
        with_temp_repo(|repo_path, repo| {
            commit_file(repo, repo_path, "README.md", "# test\n");
            std::fs::write(repo_path.join("README.md"), "# test\nnew line\n")
                .expect("update repo file");

            let (mut git_window, _) =
                GitWindow::new(Uuid::new_v4(), "repo".into(), repo_path.to_path_buf());

            let _ = git_window.update(Message::SelectFile(FileSelection {
                path: PathBuf::from("README.md"),
                status: git_data::FileStatus::Modified,
                staged: false,
            }));
            let _ = git_window.update(Message::EditSelectedFile(text_editor::Action::Edit(
                text_editor::Edit::Insert('!'),
            )));

            let detail = git_window
                .selected_detail
                .as_ref()
                .expect("selected file detail");

            assert!(detail.dirty);
            assert_ne!(
                detail.draft.as_ref().map(text_editor::Content::text).as_deref(),
                detail.worktree_text.as_deref()
            );
        });
    }

    #[test]
    fn git_window_detail_selecting_deleted_file_uses_empty_worktree_state() {
        with_temp_repo(|repo_path, repo| {
            commit_file(repo, repo_path, "README.md", "# test\n");
            std::fs::remove_file(repo_path.join("README.md")).expect("delete repo file");

            let (mut git_window, _) =
                GitWindow::new(Uuid::new_v4(), "repo".into(), repo_path.to_path_buf());

            let _ = git_window.update(Message::SelectFile(FileSelection {
                path: PathBuf::from("README.md"),
                status: git_data::FileStatus::Deleted,
                staged: false,
            }));

            let detail = git_window
                .selected_detail
                .as_ref()
                .expect("selected deleted detail");

            assert_eq!(detail.path, PathBuf::from("README.md"));
            assert_eq!(detail.status, git_data::FileStatus::Deleted);
            assert_eq!(detail.content_kind, git_data::FileContentKind::Text);
            assert_eq!(detail.base_text.as_deref(), Some("# test\n"));
            assert_eq!(detail.worktree_text.as_deref(), Some(""));
            assert_eq!(
                detail.draft.as_ref().map(text_editor::Content::text).as_deref(),
                Some("")
            );
            assert!(detail.detail_error.is_none());
        });
    }
}
