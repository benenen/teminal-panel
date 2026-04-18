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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_file_changes_non_repo() {
        let result = get_file_changes(Path::new("/tmp"));
        assert!(result.is_err());
    }
}
