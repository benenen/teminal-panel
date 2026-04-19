mod detail;
mod git_data;
mod graph;
mod theme;

use self::git_data::{
    classify_file_content, get_base_file_content, get_commit_history, get_file_changes,
    get_index_file_content, get_worktree_file_content, write_worktree_file, CommitNode, FileChange,
    FileContentKind,
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

    fn selection(&self) -> FileSelection {
        FileSelection {
            path: self.path.clone(),
            status: self.status.clone(),
            staged: self.staged,
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
                self.selected_detail = Some(load_selected_file_detail(&self.repo_path, &selection));
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
                let Some(selection) = self
                    .selected_detail
                    .as_ref()
                    .map(SelectedFileDetail::selection)
                else {
                    return Task::none();
                };

                if selection.staged {
                    return Task::none();
                }

                let draft_text = self
                    .selected_detail
                    .as_ref()
                    .and_then(|detail| detail.draft.as_ref())
                    .map(text_editor::Content::text);

                let Some(draft_text) = draft_text else {
                    return Task::none();
                };

                match write_worktree_file(&self.repo_path, &selection.path, &draft_text) {
                    Ok(()) => {
                        if let Err(error) = self.refresh_selection_after_file_change(&selection) {
                            if let Some(detail) = self.selected_detail.as_mut() {
                                detail.detail_error =
                                    Some(format!("Failed to reload applied changes: {error}"));
                            }
                        }
                    }
                    Err(error) => {
                        if let Some(detail) = self.selected_detail.as_mut() {
                            detail.detail_error = Some(format!("Failed to apply changes: {error}"));
                        }
                    }
                }
                Task::none()
            }
            Message::DiscardSelectedFile => {
                let Some(selection) = self
                    .selected_detail
                    .as_ref()
                    .map(SelectedFileDetail::selection)
                else {
                    return Task::none();
                };

                if selection.staged {
                    return Task::none();
                }

                if let Err(error) = self.refresh_selection_after_file_change(&selection) {
                    if let Some(detail) = self.selected_detail.as_mut() {
                        detail.detail_error = Some(format!("Failed to discard changes: {error}"));
                    }
                }
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
                let is_selected =
                    is_selected_file_change(self.selected_detail.as_ref(), file_change);
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
                let is_selected =
                    is_selected_file_change(self.selected_detail.as_ref(), file_change);
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

    fn refresh_selection_after_file_change(
        &mut self,
        previous_selection: &FileSelection,
    ) -> Result<(), String> {
        let changes = get_file_changes(&self.repo_path)
            .map_err(|error| format!("Failed to load git data: {error}"))?;
        let next_selection = find_refreshed_selection(&changes, previous_selection);

        self.file_changes = changes;
        self.selected_detail = match next_selection {
            Some(selection) => Some(try_load_selected_file_detail(&self.repo_path, &selection)?),
            None => None,
        };

        Ok(())
    }
}

fn load_selected_file_detail(repo_path: &Path, selection: &FileSelection) -> SelectedFileDetail {
    try_load_selected_file_detail(repo_path, selection)
        .unwrap_or_else(|error| SelectedFileDetail::error(selection, error))
}

fn try_load_selected_file_detail(
    repo_path: &Path,
    selection: &FileSelection,
) -> Result<SelectedFileDetail, String> {
    let base_bytes = match if selection.staged {
        get_base_file_content(repo_path, &selection.path)
    } else {
        get_index_file_content(repo_path, &selection.path)
    } {
        Ok(bytes) => bytes,
        Err(error) => {
            return Err(format!("Failed to load base content: {error}"));
        }
    };

    let selected_bytes = match if selection.staged {
        get_index_file_content(repo_path, &selection.path)
            .map_err(|error| std::io::Error::other(error.message().to_string()))
    } else {
        get_worktree_file_content(repo_path, &selection.path).map(Some)
    } {
        Ok(bytes) => bytes,
        Err(error)
            if selection.status == git_data::FileStatus::Deleted
                && error.kind() == std::io::ErrorKind::NotFound =>
        {
            None
        }
        Err(error) => {
            return Err(format!("Failed to load working tree content: {error}"));
        }
    };

    let diff =
        git_data::get_file_diff_for_selection(repo_path, &selection.path, selection.staged).ok();
    let content_kind = if base_bytes
        .as_deref()
        .is_some_and(|bytes| classify_file_content(bytes) == FileContentKind::Binary)
        || selected_bytes
            .as_deref()
            .is_some_and(|bytes| classify_file_content(bytes) == FileContentKind::Binary)
    {
        FileContentKind::Binary
    } else {
        FileContentKind::Text
    };

    if content_kind == FileContentKind::Binary {
        return Ok(SelectedFileDetail {
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
        });
    }

    let base_text = base_bytes
        .map(|bytes| String::from_utf8(bytes).unwrap_or_default())
        .unwrap_or_default();
    let worktree_text = selected_bytes
        .map(|bytes| String::from_utf8(bytes).unwrap_or_default())
        .unwrap_or_default();

    Ok(SelectedFileDetail {
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
    })
}

fn is_selected_file_change(
    selected_detail: Option<&SelectedFileDetail>,
    file_change: &FileChange,
) -> bool {
    selected_detail.is_some_and(|detail| {
        detail.path == file_change.path && detail.staged == file_change.staged
    })
}

fn find_refreshed_selection(
    file_changes: &[FileChange],
    previous_selection: &FileSelection,
) -> Option<FileSelection> {
    file_changes
        .iter()
        .find(|change| {
            change.path == previous_selection.path && change.staged == previous_selection.staged
        })
        .map(|change| FileSelection {
            path: change.path.clone(),
            status: change.status.clone(),
            staged: change.staged,
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use git2::{IndexAddOption, Repository, Signature};
    use iced::widget::text_editor;
    use std::sync::Arc;

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
        let parent_commit = repo
            .head()
            .ok()
            .and_then(|head| head.target())
            .and_then(|oid| repo.find_commit(oid).ok());
        let parents = parent_commit.iter().collect::<Vec<_>>();

        repo.commit(
            Some("HEAD"),
            &signature,
            &signature,
            "Initial commit",
            &tree,
            &parents,
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
                detail
                    .draft
                    .as_ref()
                    .map(text_editor::Content::text)
                    .as_deref(),
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
                detail
                    .draft
                    .as_ref()
                    .map(text_editor::Content::text)
                    .as_deref(),
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
                detail
                    .draft
                    .as_ref()
                    .map(text_editor::Content::text)
                    .as_deref(),
                Some("")
            );
            assert!(detail.detail_error.is_none());
        });
    }

    #[test]
    fn git_window_detail_selecting_staged_file_uses_index_snapshot() {
        with_temp_repo(|repo_path, repo| {
            commit_file(repo, repo_path, "README.md", "base line\n");
            std::fs::write(repo_path.join("README.md"), "staged line\n")
                .expect("write staged content");

            let mut index = repo.index().expect("open index");
            index
                .add_path(std::path::Path::new("README.md"))
                .expect("stage modified file");
            index.write().expect("write index");

            std::fs::write(repo_path.join("README.md"), "staged line\nunstaged line\n")
                .expect("write unstaged follow-up");

            let (mut git_window, _) =
                GitWindow::new(Uuid::new_v4(), "repo".into(), repo_path.to_path_buf());

            let _ = git_window.update(Message::SelectFile(FileSelection {
                path: PathBuf::from("README.md"),
                status: git_data::FileStatus::Modified,
                staged: true,
            }));

            let detail = git_window
                .selected_detail
                .as_ref()
                .expect("selected staged detail");

            assert!(detail.staged);
            assert_eq!(detail.base_text.as_deref(), Some("base line\n"));
            assert_eq!(detail.worktree_text.as_deref(), Some("staged line\n"));
            assert_eq!(
                detail
                    .draft
                    .as_ref()
                    .map(text_editor::Content::text)
                    .as_deref(),
                Some("staged line\n")
            );
        });
    }

    #[test]
    fn git_window_detail_selecting_unstaged_file_uses_index_as_base_snapshot() {
        with_temp_repo(|repo_path, repo| {
            commit_file(repo, repo_path, "README.md", "base line\n");
            std::fs::write(repo_path.join("README.md"), "staged line\n")
                .expect("write staged content");

            let mut index = repo.index().expect("open index");
            index
                .add_path(std::path::Path::new("README.md"))
                .expect("stage modified file");
            index.write().expect("write index");

            std::fs::write(repo_path.join("README.md"), "staged line\nunstaged line\n")
                .expect("write unstaged follow-up");

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
                .expect("selected unstaged detail");

            assert!(!detail.staged);
            assert_eq!(detail.base_text.as_deref(), Some("staged line\n"));
            assert_eq!(
                detail.worktree_text.as_deref(),
                Some("staged line\nunstaged line\n")
            );
        });
    }

    #[test]
    fn git_window_selected_file_row_distinguishes_staged_entries() {
        let selected = SelectedFileDetail {
            path: PathBuf::from("README.md"),
            status: git_data::FileStatus::Modified,
            staged: true,
            content_kind: git_data::FileContentKind::Text,
            base_text: Some("base line\n".into()),
            worktree_text: Some("staged line\n".into()),
            draft: Some(text_editor::Content::with_text("staged line\n")),
            dirty: false,
            diff: None,
            detail_error: None,
        };

        let staged_change = FileChange {
            path: PathBuf::from("README.md"),
            status: git_data::FileStatus::Modified,
            staged: true,
        };
        let unstaged_change = FileChange {
            path: PathBuf::from("README.md"),
            status: git_data::FileStatus::Modified,
            staged: false,
        };

        assert!(is_selected_file_change(Some(&selected), &staged_change));
        assert!(!is_selected_file_change(Some(&selected), &unstaged_change));
    }

    #[test]
    fn apply_selected_file_writes_draft_to_disk_and_refreshes_state() {
        with_temp_repo(|repo_path, repo| {
            commit_file(repo, repo_path, "README.md", "base line\n");
            std::fs::write(repo_path.join("README.md"), "working line\n")
                .expect("write working tree content");

            let (mut git_window, _) =
                GitWindow::new(Uuid::new_v4(), "repo".into(), repo_path.to_path_buf());

            let _ = git_window.update(Message::SelectFile(FileSelection {
                path: PathBuf::from("README.md"),
                status: git_data::FileStatus::Modified,
                staged: false,
            }));
            let _ = git_window.update(Message::EditSelectedFile(text_editor::Action::SelectAll));
            let _ = git_window.update(Message::EditSelectedFile(text_editor::Action::Edit(
                text_editor::Edit::Paste(Arc::new("applied line\n".into())),
            )));
            let _ = git_window.update(Message::ApplySelectedFile);

            let detail = git_window
                .selected_detail
                .as_ref()
                .expect("selected file detail");

            assert_eq!(
                std::fs::read_to_string(repo_path.join("README.md")).expect("read applied file"),
                "applied line\n"
            );
            assert_eq!(detail.worktree_text.as_deref(), Some("applied line\n"));
            assert_eq!(
                detail
                    .draft
                    .as_ref()
                    .map(text_editor::Content::text)
                    .as_deref(),
                Some("applied line\n")
            );
            assert!(!detail.dirty);
            assert!(detail.detail_error.is_none());
        });
    }

    #[test]
    fn discard_selected_file_reloads_current_disk_contents() {
        with_temp_repo(|repo_path, repo| {
            commit_file(repo, repo_path, "README.md", "base line\n");
            std::fs::write(repo_path.join("README.md"), "working line\n")
                .expect("write working tree content");

            let (mut git_window, _) =
                GitWindow::new(Uuid::new_v4(), "repo".into(), repo_path.to_path_buf());

            let _ = git_window.update(Message::SelectFile(FileSelection {
                path: PathBuf::from("README.md"),
                status: git_data::FileStatus::Modified,
                staged: false,
            }));
            let _ = git_window.update(Message::EditSelectedFile(text_editor::Action::SelectAll));
            let _ = git_window.update(Message::EditSelectedFile(text_editor::Action::Edit(
                text_editor::Edit::Paste(Arc::new("draft line\n".into())),
            )));

            std::fs::write(repo_path.join("README.md"), "reloaded line\n")
                .expect("write reloaded content");

            let _ = git_window.update(Message::DiscardSelectedFile);

            let detail = git_window
                .selected_detail
                .as_ref()
                .expect("selected file detail");

            assert_eq!(detail.worktree_text.as_deref(), Some("reloaded line\n"));
            assert_eq!(
                detail
                    .draft
                    .as_ref()
                    .map(text_editor::Content::text)
                    .as_deref(),
                Some("reloaded line\n")
            );
            assert!(!detail.dirty);
            assert!(detail.detail_error.is_none());
        });
    }

    #[cfg(unix)]
    #[test]
    fn apply_failure_preserves_in_memory_draft() {
        use std::os::unix::fs::symlink;

        with_temp_repo(|repo_path, repo| {
            commit_file(repo, repo_path, "README.md", "base line\n");
            std::fs::write(repo_path.join("README.md"), "working line\n")
                .expect("write working tree content");

            let (mut git_window, _) =
                GitWindow::new(Uuid::new_v4(), "repo".into(), repo_path.to_path_buf());

            let _ = git_window.update(Message::SelectFile(FileSelection {
                path: PathBuf::from("README.md"),
                status: git_data::FileStatus::Modified,
                staged: false,
            }));
            let _ = git_window.update(Message::EditSelectedFile(text_editor::Action::SelectAll));
            let _ = git_window.update(Message::EditSelectedFile(text_editor::Action::Edit(
                text_editor::Edit::Paste(Arc::new("draft line\n".into())),
            )));

            let outside_dir = std::env::temp_dir().join(format!(
                "teminal-panel-git-window-outside-{}",
                Uuid::new_v4()
            ));
            std::fs::create_dir_all(&outside_dir).expect("create outside dir");
            std::fs::write(outside_dir.join("outside.txt"), "outside\n").expect("write outside");

            std::fs::remove_file(repo_path.join("README.md")).expect("remove repo file");
            symlink(outside_dir.join("outside.txt"), repo_path.join("README.md"))
                .expect("create escape symlink");

            let _ = git_window.update(Message::ApplySelectedFile);

            let detail = git_window
                .selected_detail
                .as_ref()
                .expect("selected file detail");

            assert_eq!(
                detail
                    .draft
                    .as_ref()
                    .map(text_editor::Content::text)
                    .as_deref(),
                Some("draft line\n")
            );
            assert!(detail.dirty);
            assert!(detail
                .detail_error
                .as_deref()
                .is_some_and(|error| error.contains("Failed to apply changes")));

            let _ = std::fs::remove_file(repo_path.join("README.md"));
            let _ = std::fs::remove_dir_all(&outside_dir);
        });
    }

    #[test]
    fn refresh_git_window_after_apply_clears_detail_for_clean_file() {
        with_temp_repo(|repo_path, repo| {
            commit_file(repo, repo_path, "README.md", "base line\n");
            std::fs::write(repo_path.join("README.md"), "working line\n")
                .expect("write working tree content");

            let (mut git_window, _) =
                GitWindow::new(Uuid::new_v4(), "repo".into(), repo_path.to_path_buf());

            let _ = git_window.update(Message::SelectFile(FileSelection {
                path: PathBuf::from("README.md"),
                status: git_data::FileStatus::Modified,
                staged: false,
            }));
            let _ = git_window.update(Message::EditSelectedFile(text_editor::Action::SelectAll));
            let _ = git_window.update(Message::EditSelectedFile(text_editor::Action::Edit(
                text_editor::Edit::Paste(Arc::new("base line\n".into())),
            )));
            let _ = git_window.update(Message::ApplySelectedFile);

            assert!(git_window
                .file_changes
                .iter()
                .all(|change| change.path != PathBuf::from("README.md")));
            assert!(git_window.selected_detail.is_none());
        });
    }

    #[test]
    fn refresh_git_window_after_apply_updates_recreated_file_status() {
        with_temp_repo(|repo_path, repo| {
            commit_file(repo, repo_path, "README.md", "base line\n");
            std::fs::remove_file(repo_path.join("README.md")).expect("delete repo file");

            let (mut git_window, _) =
                GitWindow::new(Uuid::new_v4(), "repo".into(), repo_path.to_path_buf());

            let _ = git_window.update(Message::SelectFile(FileSelection {
                path: PathBuf::from("README.md"),
                status: git_data::FileStatus::Deleted,
                staged: false,
            }));
            let _ = git_window.update(Message::EditSelectedFile(text_editor::Action::SelectAll));
            let _ = git_window.update(Message::EditSelectedFile(text_editor::Action::Edit(
                text_editor::Edit::Paste(Arc::new("restored line\n".into())),
            )));
            let _ = git_window.update(Message::ApplySelectedFile);

            let detail = git_window
                .selected_detail
                .as_ref()
                .expect("selected file detail");

            assert_eq!(detail.status, git_data::FileStatus::Modified);
            assert!(git_window.file_changes.iter().any(|change| {
                change.path == PathBuf::from("README.md")
                    && change.status == git_data::FileStatus::Modified
                    && !change.staged
            }));
        });
    }

    #[test]
    fn file_list_refresh_after_apply_allows_selecting_another_file() {
        with_temp_repo(|repo_path, repo| {
            commit_file(repo, repo_path, "README.md", "base line\n");
            commit_file(repo, repo_path, "NOTES.md", "notes base\n");
            std::fs::write(repo_path.join("README.md"), "working line\n")
                .expect("write working tree content");
            std::fs::write(repo_path.join("NOTES.md"), "notes changed\n")
                .expect("write notes content");

            let (mut git_window, _) =
                GitWindow::new(Uuid::new_v4(), "repo".into(), repo_path.to_path_buf());

            let _ = git_window.update(Message::SelectFile(FileSelection {
                path: PathBuf::from("README.md"),
                status: git_data::FileStatus::Modified,
                staged: false,
            }));
            let _ = git_window.update(Message::EditSelectedFile(text_editor::Action::SelectAll));
            let _ = git_window.update(Message::EditSelectedFile(text_editor::Action::Edit(
                text_editor::Edit::Paste(Arc::new("base line\n".into())),
            )));
            let _ = git_window.update(Message::ApplySelectedFile);

            let _ = git_window.update(Message::SelectFile(FileSelection {
                path: PathBuf::from("NOTES.md"),
                status: git_data::FileStatus::Modified,
                staged: false,
            }));

            let detail = git_window
                .selected_detail
                .as_ref()
                .expect("selected notes detail");

            assert_eq!(detail.path, PathBuf::from("NOTES.md"));
            assert_eq!(detail.base_text.as_deref(), Some("notes base\n"));
            assert_eq!(detail.worktree_text.as_deref(), Some("notes changed\n"));
        });
    }
}
