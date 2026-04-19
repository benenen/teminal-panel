use git2::{DiffFormat, DiffOptions, Repository, Status, StatusOptions};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq)]
pub enum FileStatus {
    Added,
    Modified,
    Deleted,
}

#[derive(Debug, Clone)]
pub struct FileChange {
    pub path: PathBuf,
    pub status: FileStatus,
    pub staged: bool,
}

#[derive(Debug, Clone)]
pub struct CommitNode {
    pub oid: git2::Oid,
    pub short_id: String,
    pub summary: String,
    pub author: String,
    pub timestamp: i64,
    pub parent_count: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileContentKind {
    Text,
    Binary,
}

pub fn get_file_changes(repo_path: &Path) -> Result<Vec<FileChange>, git2::Error> {
    let repo = Repository::open(repo_path)?;
    let mut opts = StatusOptions::new();
    opts.include_untracked(true);

    let statuses = repo.statuses(Some(&mut opts))?;
    let mut changes = Vec::new();

    for entry in statuses.iter() {
        if let Some(path) = entry.path() {
            let path_buf = PathBuf::from(path);
            let status_flags = entry.status();

            let staged = status_flags
                .intersects(Status::INDEX_NEW | Status::INDEX_MODIFIED | Status::INDEX_DELETED);

            let status = if status_flags.contains(Status::WT_NEW)
                || status_flags.contains(Status::INDEX_NEW)
            {
                FileStatus::Added
            } else if status_flags.contains(Status::WT_DELETED)
                || status_flags.contains(Status::INDEX_DELETED)
            {
                FileStatus::Deleted
            } else {
                FileStatus::Modified
            };

            changes.push(FileChange {
                path: path_buf,
                status,
                staged,
            });
        }
    }

    Ok(changes)
}

pub fn get_commit_history(repo_path: &Path, limit: usize) -> Result<Vec<CommitNode>, git2::Error> {
    let repo = Repository::open(repo_path)?;
    let mut revwalk = repo.revwalk()?;
    revwalk.push_head()?;

    let mut commits = Vec::new();
    for oid in revwalk.take(limit) {
        let oid = oid?;
        let commit = repo.find_commit(oid)?;
        let oid_text = oid.to_string();

        commits.push(CommitNode {
            oid,
            short_id: oid_text.chars().take(7).collect(),
            summary: commit.summary().unwrap_or_default().to_string(),
            author: commit.author().name().unwrap_or("Unknown").to_string(),
            timestamp: commit.time().seconds(),
            parent_count: commit.parent_count(),
        });
    }

    Ok(commits)
}

pub fn get_file_diff(repo_path: &Path, file_path: &Path) -> Result<String, git2::Error> {
    let repo = Repository::open(repo_path)?;
    let head_tree = repo.head().ok().and_then(|head| head.peel_to_tree().ok());

    let mut opts = DiffOptions::new();
    opts.pathspec(file_path);

    let diff = repo.diff_tree_to_workdir_with_index(head_tree.as_ref(), Some(&mut opts))?;
    let mut patch = String::new();

    diff.print(DiffFormat::Patch, |_delta, _hunk, line| {
        match line.origin() {
            '+' | '-' | ' ' | '\\' => patch.push(line.origin()),
            _ => {}
        }
        patch.push_str(String::from_utf8_lossy(line.content()).as_ref());
        true
    })?;

    Ok(patch)
}

pub fn get_base_file_content(
    repo_path: &Path,
    file_path: &Path,
) -> Result<Option<Vec<u8>>, git2::Error> {
    let repo = Repository::open(repo_path)?;
    let head = match repo.head() {
        Ok(head) => head,
        Err(err) => {
            if err.code() == git2::ErrorCode::NotFound
                || err.code() == git2::ErrorCode::UnbornBranch
            {
                return Ok(None);
            }
            return Err(err);
        }
    };
    let tree = head.peel_to_tree()?;

    let entry = match tree.get_path(file_path) {
        Ok(entry) => entry,
        Err(err) => {
            if err.code() == git2::ErrorCode::NotFound {
                return Ok(None);
            }
            return Err(err);
        }
    };

    if entry.kind() != Some(git2::ObjectType::Blob) {
        return Ok(None);
    }

    let blob = repo.find_blob(entry.id())?;
    Ok(Some(blob.content().to_vec()))
}

fn resolve_repo_relative_path(
    repo_path: &Path,
    file_path: &Path,
) -> Result<PathBuf, std::io::Error> {
    if file_path.is_absolute() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "file path must be repo-relative",
        ));
    }

    let mut normalized = PathBuf::new();
    for component in file_path.components() {
        match component {
            std::path::Component::Normal(part) => normalized.push(part),
            std::path::Component::CurDir => {}
            std::path::Component::ParentDir
            | std::path::Component::RootDir
            | std::path::Component::Prefix(_) => {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    "file path must stay within repo root",
                ));
            }
        }
    }

    if normalized.as_os_str().is_empty() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "file path must not be empty",
        ));
    }

    Ok(repo_path.join(normalized))
}

pub fn get_worktree_file_content(
    repo_path: &Path,
    file_path: &Path,
) -> Result<Vec<u8>, std::io::Error> {
    let full_path = resolve_repo_relative_path(repo_path, file_path)?;
    std::fs::read(full_path)
}

pub fn classify_file_content(bytes: &[u8]) -> FileContentKind {
    if bytes.contains(&0) {
        return FileContentKind::Binary;
    }

    if std::str::from_utf8(bytes).is_ok() {
        FileContentKind::Text
    } else {
        FileContentKind::Binary
    }
}

pub fn write_worktree_file(
    repo_path: &Path,
    file_path: &Path,
    contents: &str,
) -> Result<(), std::io::Error> {
    let full_path = resolve_repo_relative_path(repo_path, file_path)?;
    if let Some(parent) = full_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(full_path, contents)
}

#[cfg(test)]
mod tests {
    use super::*;
    use git2::{IndexAddOption, Signature};
    use uuid::Uuid;

    fn with_temp_repo<T>(f: impl FnOnce(&Path, &Repository) -> T) -> T {
        let temp_dir =
            std::env::temp_dir().join(format!("teminal-panel-git-data-{}", Uuid::new_v4()));
        std::fs::create_dir_all(&temp_dir).expect("create temp repo dir");

        let repo = Repository::init(&temp_dir).expect("init repo");
        let result = f(&temp_dir, &repo);

        let _ = std::fs::remove_dir_all(&temp_dir);
        result
    }

    fn with_temp_non_repo_dir<T>(f: impl FnOnce(&Path) -> T) -> T {
        let temp_dir = std::env::temp_dir().join(format!(
            "teminal-panel-git-data-non-repo-{}",
            Uuid::new_v4()
        ));
        std::fs::create_dir_all(&temp_dir).expect("create temp non-repo dir");

        let result = f(&temp_dir);

        let _ = std::fs::remove_dir_all(&temp_dir);
        result
    }

    fn commit_file(
        repo: &Repository,
        repo_path: &Path,
        file_name: &str,
        contents: &str,
        message: &str,
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

        repo.commit(Some("HEAD"), &signature, &signature, message, &tree, &[])
            .expect("create commit");
    }

    #[test]
    fn test_get_file_changes_non_repo() {
        with_temp_non_repo_dir(|temp_dir| {
            let result = get_file_changes(temp_dir);
            assert!(result.is_err());
        });
    }

    #[test]
    fn git_commit_history_non_repo_fails_cleanly() {
        with_temp_non_repo_dir(|temp_dir| {
            let result = get_commit_history(temp_dir, 10);
            assert!(result.is_err());
        });
    }

    #[test]
    fn git_commit_history_returns_commit_nodes_for_repo() {
        with_temp_repo(|repo_path, repo| {
            commit_file(repo, repo_path, "README.md", "# test\n", "Initial commit");

            let history = get_commit_history(repo_path, 10).expect("load commit history");

            assert!(!history.is_empty());
        });
    }

    #[test]
    fn git_commit_history_includes_short_id_and_summary_text() {
        with_temp_repo(|repo_path, repo| {
            commit_file(repo, repo_path, "README.md", "# test\n", "Initial commit");

            let history = get_commit_history(repo_path, 10).expect("load commit history");
            let commit = history.first().expect("at least one commit");

            assert_eq!(commit.summary, "Initial commit");
            assert_eq!(commit.short_id.len(), 7);
            assert!(!commit.short_id.is_empty());
        });
    }

    #[test]
    fn git_file_diff_non_repo_fails_cleanly() {
        with_temp_non_repo_dir(|temp_dir| {
            let result = get_file_diff(temp_dir, Path::new("README.md"));
            assert!(result.is_err());
        });
    }

    #[test]
    fn git_file_diff_returns_patch_text_for_modified_file() {
        with_temp_repo(|repo_path, repo| {
            commit_file(repo, repo_path, "README.md", "# test\n", "Initial commit");
            std::fs::write(repo_path.join("README.md"), "# test\nnew line\n")
                .expect("update repo file");

            let diff = get_file_diff(repo_path, Path::new("README.md")).expect("load file diff");

            assert!(diff.contains("diff --git"));
            assert!(diff.contains("README.md"));
            assert!(diff.contains("+new line"));
        });
    }

    #[test]
    fn git_file_diff_preserves_non_utf8_bytes_with_lossy_fallback() {
        with_temp_repo(|repo_path, repo| {
            commit_file(repo, repo_path, "README.md", "hello\n", "Initial commit");
            std::fs::write(repo_path.join("README.md"), [b'f', b'o', 0x80, b'o', b'\n'])
                .expect("write non-utf8 content");

            let diff = get_file_diff(repo_path, Path::new("README.md")).expect("load file diff");

            assert!(diff.contains("+fo\u{FFFD}o"));
        });
    }

    #[test]
    fn git_file_detail_reads_base_revision_text_for_tracked_file() {
        with_temp_repo(|repo_path, repo| {
            commit_file(
                repo,
                repo_path,
                "README.md",
                "base line\n",
                "Initial commit",
            );
            std::fs::write(repo_path.join("README.md"), "working tree line\n")
                .expect("update repo file");

            let base_content =
                get_base_file_content(repo_path, Path::new("README.md")).expect("load base text");

            assert_eq!(
                base_content.expect("tracked file should exist in base"),
                b"base line\n".to_vec()
            );
        });
    }

    #[test]
    fn git_file_detail_reads_worktree_text_for_tracked_file() {
        with_temp_repo(|repo_path, repo| {
            commit_file(
                repo,
                repo_path,
                "README.md",
                "base line\n",
                "Initial commit",
            );
            std::fs::write(repo_path.join("README.md"), "working tree line\n")
                .expect("update repo file");

            let worktree_content = get_worktree_file_content(repo_path, Path::new("README.md"))
                .expect("load worktree text");

            assert_eq!(worktree_content, b"working tree line\n".to_vec());
        });
    }

    #[test]
    fn git_binary_file_classification_detects_non_text_content() {
        let bytes = [0x42, 0x49, 0x4e, 0x00, 0xff];

        let kind = classify_file_content(&bytes);

        assert_eq!(kind, FileContentKind::Binary);
    }

    #[test]
    fn git_write_worktree_file_updates_contents_on_disk() {
        with_temp_repo(|repo_path, repo| {
            commit_file(
                repo,
                repo_path,
                "README.md",
                "base line\n",
                "Initial commit",
            );

            write_worktree_file(repo_path, Path::new("README.md"), "updated line\n")
                .expect("write working tree file");

            let updated = std::fs::read(repo_path.join("README.md")).expect("read updated file");
            assert_eq!(updated, b"updated line\n".to_vec());
        });
    }

    #[test]
    fn git_file_detail_rejects_absolute_worktree_path() {
        with_temp_repo(|repo_path, repo| {
            commit_file(
                repo,
                repo_path,
                "README.md",
                "base line\n",
                "Initial commit",
            );
            let abs_path = repo_path.join("README.md");

            let read_result = get_worktree_file_content(repo_path, abs_path.as_path());
            let write_result = write_worktree_file(repo_path, abs_path.as_path(), "updated line\n");

            assert_eq!(
                read_result.expect_err("absolute path should fail").kind(),
                std::io::ErrorKind::InvalidInput
            );
            assert_eq!(
                write_result.expect_err("absolute path should fail").kind(),
                std::io::ErrorKind::InvalidInput
            );
        });
    }

    #[test]
    fn git_file_detail_rejects_parent_traversal_worktree_path() {
        with_temp_repo(|repo_path, repo| {
            commit_file(
                repo,
                repo_path,
                "README.md",
                "base line\n",
                "Initial commit",
            );

            let outside_path = repo_path
                .parent()
                .expect("temp repo has parent")
                .join("outside.txt");
            std::fs::write(&outside_path, "outside content\n").expect("write outside file");

            let read_result = get_worktree_file_content(repo_path, Path::new("../outside.txt"));
            let write_result =
                write_worktree_file(repo_path, Path::new("../outside.txt"), "updated line\n");

            assert_eq!(
                read_result.expect_err("traversal should fail").kind(),
                std::io::ErrorKind::InvalidInput
            );
            assert_eq!(
                write_result.expect_err("traversal should fail").kind(),
                std::io::ErrorKind::InvalidInput
            );
        });
    }
}
