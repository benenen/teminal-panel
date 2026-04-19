use git2::{Repository, Status, StatusOptions};
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

#[cfg(test)]
mod tests {
    use super::*;
    use git2::{IndexAddOption, Signature};
    use uuid::Uuid;

    fn with_temp_repo<T>(f: impl FnOnce(&Path, &Repository) -> T) -> T {
        let temp_dir = std::env::temp_dir().join(format!("teminal-panel-git-data-{}", Uuid::new_v4()));
        std::fs::create_dir_all(&temp_dir).expect("create temp repo dir");

        let repo = Repository::init(&temp_dir).expect("init repo");
        let result = f(&temp_dir, &repo);

        let _ = std::fs::remove_dir_all(&temp_dir);
        result
    }

    fn commit_file(repo: &Repository, repo_path: &Path, file_name: &str, contents: &str, message: &str) {
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
            message,
            &tree,
            &[],
        )
        .expect("create commit");
    }

    #[test]
    fn test_get_file_changes_non_repo() {
        let result = get_file_changes(Path::new("/tmp"));
        assert!(result.is_err());
    }

    #[test]
    fn git_commit_history_non_repo_fails_cleanly() {
        let result = get_commit_history(Path::new("/tmp"), 10);
        assert!(result.is_err());
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
}
