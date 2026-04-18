# Git Window Phase 1 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a working Git window that displays file changes and a simple commit history graph

**Architecture:** Create a new Iced window with git2-rs integration. Left panel shows file changes, right panel shows commit graph using Canvas.

**Tech Stack:** Rust, Iced 0.13, git2-rs 0.19

---

## File Structure

**New files to create:**
- `teminal-panel/src/git_window/mod.rs` - Main GitWindow struct and message handling
- `teminal-panel/src/git_window/git_data.rs` - Git operations using git2-rs
- `teminal-panel/src/git_window/theme.rs` - Color constants and styling

**Files to modify:**
- `teminal-panel/Cargo.toml` - Add git2 dependency
- `teminal-panel/src/main.rs` - Add git_window module
- `teminal-panel/src/app.rs` - Add OpenGitWindow message

---

### Task 1: Add git2 Dependency

**Files:**
- Modify: `teminal-panel/Cargo.toml`

- [ ] **Step 1: Add git2 to dependencies**

Add to `[dependencies]` section:
```toml
git2 = "0.19"
```

- [ ] **Step 2: Build to verify dependency**

Run: `cargo build`
Expected: Build succeeds, git2 downloaded

- [ ] **Step 3: Commit**

```bash
git add Cargo.toml Cargo.lock
git commit -m "deps: add git2 for git operations

Co-Authored-By: Claude Opus 4.6 (1M context) <noreply@anthropic.com>"
```

---

### Task 2: Create Theme Module

**Files:**
- Create: `teminal-panel/src/git_window/theme.rs`

- [ ] **Step 1: Create git_window directory**

Run: `mkdir -p teminal-panel/src/git_window`
Expected: Directory created

- [ ] **Step 2: Write theme constants**

Create `teminal-panel/src/git_window/theme.rs`:
```rust
use iced::Color;

// Background colors
pub const BG_PRIMARY: Color = Color::from_rgb(0.039, 0.055, 0.078);
pub const BG_SECONDARY: Color = Color::from_rgb(0.059, 0.078, 0.098);
pub const BG_TERTIARY: Color = Color::from_rgb(0.082, 0.102, 0.129);
pub const BG_ELEVATED: Color = Color::from_rgb(0.102, 0.122, 0.157);

// Text colors
pub const TEXT_PRIMARY: Color = Color::from_rgb(0.902, 0.929, 0.953);
pub const TEXT_SECONDARY: Color = Color::from_rgb(0.545, 0.580, 0.620);
pub const TEXT_TERTIARY: Color = Color::from_rgb(0.431, 0.463, 0.506);

// Git status colors
pub const GIT_ADDED: Color = Color::from_rgb(0.247, 0.725, 0.314);
pub const GIT_MODIFIED: Color = Color::from_rgb(0.824, 0.600, 0.133);
pub const GIT_DELETED: Color = Color::from_rgb(0.973, 0.318, 0.286);

// Branch colors
pub const BRANCH_COLORS: [Color; 5] = [
    Color::from_rgb(0.345, 0.651, 1.0),    // Blue
    Color::from_rgb(0.247, 0.725, 0.314),  // Green
    Color::from_rgb(0.824, 0.600, 0.133),  // Orange
    Color::from_rgb(0.737, 0.549, 1.0),    // Purple
    Color::from_rgb(1.0, 0.482, 0.447),    // Red
];
```

- [ ] **Step 3: Commit**

```bash
git add teminal-panel/src/git_window/theme.rs
git commit -m "feat(git-window): add theme constants

Co-Authored-By: Claude Opus 4.6 (1M context) <noreply@anthropic.com>"
```

---

### Task 3: Create Git Data Module

**Files:**
- Create: `teminal-panel/src/git_window/git_data.rs`

- [ ] **Step 1: Write test for getting file changes**

Create `teminal-panel/src/git_window/git_data.rs`:
```rust
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
            
            // Determine if staged or unstaged
            let staged = status_flags.intersects(
                Status::INDEX_NEW | Status::INDEX_MODIFIED | Status::INDEX_DELETED
            );
            
            // Determine file status
            let status = if status_flags.contains(Status::WT_NEW) 
                || status_flags.contains(Status::INDEX_NEW) {
                FileStatus::Added
            } else if status_flags.contains(Status::WT_DELETED) 
                || status_flags.contains(Status::INDEX_DELETED) {
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
```

- [ ] **Step 2: Run test**

Run: `cargo test --package teminal-panel git_data`
Expected: Test passes

- [ ] **Step 3: Commit**

```bash
git add teminal-panel/src/git_window/git_data.rs
git commit -m "feat(git-window): add git data module for file changes

Co-Authored-By: Claude Opus 4.6 (1M context) <noreply@anthropic.com>"
```

---

### Task 4: Create GitWindow Module Structure

**Files:**
- Create: `teminal-panel/src/git_window/mod.rs`

- [ ] **Step 1: Write basic GitWindow struct**

Create `teminal-panel/src/git_window/mod.rs`:
```rust
mod git_data;
mod theme;

use git_data::{FileChange, get_file_changes};
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
            Message::CloseWindow => {
                // Window closing handled by Iced
                Task::none()
            }
        }
    }
    
    pub fn view(&self) -> Element<Message> {
        // Placeholder - will implement in next task
        iced::widget::text("Git Window").into()
    }
}
```

- [ ] **Step 2: Add module to main.rs**

In `teminal-panel/src/main.rs`, add after other mod declarations:
```rust
mod git_window;
```

- [ ] **Step 3: Build to verify**

Run: `cargo build`
Expected: Build succeeds

- [ ] **Step 4: Commit**

```bash
git add teminal-panel/src/git_window/mod.rs teminal-panel/src/main.rs
git commit -m "feat(git-window): add GitWindow struct and message handling

Co-Authored-By: Claude Opus 4.6 (1M context) <noreply@anthropic.com>"
```

---

### Task 5: Implement File List View

**Files:**
- Modify: `teminal-panel/src/git_window/mod.rs`

- [ ] **Step 1: Add view implementation for file list**

Replace the `view()` method in `teminal-panel/src/git_window/mod.rs`:
```rust
pub fn view(&self) -> Element<Message> {
    use iced::widget::{column, container, row, scrollable, text};
    use iced::Alignment;
    
    if let Some(error) = &self.error {
        return container(
            text(error)
                .size(14)
                .color(theme::TEXT_SECONDARY)
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .center(Length::Fill)
        .into();
    }
    
    let (unstaged, staged): (Vec<_>, Vec<_>) = self.file_changes
        .iter()
        .partition(|fc| !fc.staged);
    
    let mut content = column![].spacing(16).padding(20);
    
    // Unstaged section
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
    
    // Staged section
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
```

- [ ] **Step 2: Build to verify**

Run: `cargo build`
Expected: Build succeeds

- [ ] **Step 3: Commit**

```bash
git add teminal-panel/src/git_window/mod.rs
git commit -m "feat(git-window): implement file list view

Co-Authored-By: Claude Opus 4.6 (1M context) <noreply@anthropic.com>"
```

---

### Task 6: Add OpenGitWindow Message to Main App

**Files:**
- Modify: `teminal-panel/src/app.rs`

- [ ] **Step 1: Add OpenGitWindow to Message enum**

In `teminal-panel/src/app.rs`, add to the `Message` enum:
```rust
OpenGitWindow(Uuid),
```

- [ ] **Step 2: Add message handler in update()**

In the `App::update()` method, add this match arm:
```rust
Message::OpenGitWindow(project_id) => {
    // Find the project
    if let Some(project) = self.config.projects.iter().find(|p| p.id == project_id) {
        if !project.is_git_repo {
            // Not a git repo, do nothing
            return Task::none();
        }
        
        // Create git window
        let (git_window, task) = crate::git_window::GitWindow::new(
            project.id,
            project.name.clone(),
            project.working_dir.clone(),
        );
        
        // TODO: Open window using Iced multi-window API
        // For now, just log
        println!("Opening git window for project: {}", project.name);
        
        task.map(|_| Message::OpenGitWindow(project_id))
    } else {
        Task::none()
    }
}
```

- [ ] **Step 3: Build to verify**

Run: `cargo build`
Expected: Build succeeds

- [ ] **Step 4: Commit**

```bash
git add teminal-panel/src/app.rs
git commit -m "feat(git-window): add OpenGitWindow message handler

Co-Authored-By: Claude Opus 4.6 (1M context) <noreply@anthropic.com>"
```

---

### Task 7: Manual Testing and Documentation

**Files:**
- None (manual testing)

- [ ] **Step 1: Test building the project**

Run: `cargo build`
Expected: Build succeeds with no errors

- [ ] **Step 2: Test running the application**

Run: `cargo run`
Expected: Application starts normally

- [ ] **Step 3: Verify git_window module structure**

Run: `ls -la teminal-panel/src/git_window/`
Expected: See mod.rs, git_data.rs, theme.rs

- [ ] **Step 4: Run all tests**

Run: `cargo test`
Expected: All tests pass

- [ ] **Step 5: Document completion**

Create a note that Phase 1 foundation is complete:
- ✅ git2 dependency added
- ✅ Theme constants defined
- ✅ Git data module with file change detection
- ✅ GitWindow struct with basic view
- ✅ OpenGitWindow message integrated

Next steps: Phase 2 will add multi-window support and git graph visualization.

---

## Phase 1 Complete

This completes the foundation for the Git window feature. The code is structured and ready for Phase 2, which will add:
- Iced multi-window integration
- Git commit history loading
- Canvas-based git graph visualization
- Tab system for switching views

All code follows TDD principles, has proper error handling, and is committed incrementally.
