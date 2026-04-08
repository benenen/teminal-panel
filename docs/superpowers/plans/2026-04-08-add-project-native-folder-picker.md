# Add Project Native Folder Picker Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Rename the app's local "agent" concept to "project", keep manual project naming, and replace manual directory entry with a system-native folder picker.

**Architecture:** Keep the existing single-process `iced` app structure and current PTY startup logic, but migrate state/config naming from `Agent` to `Project`. Add a native folder selection task that updates the add-project form with a `PathBuf`, and keep config backward-compatible by reading legacy `agents` while writing only `projects`.

**Tech Stack:** Rust, iced 0.13, tokio 1, serde 1, toml 0.8, uuid 1, portable-pty 0.8, rfd 0.17

---

## File Structure

- Modify: `Cargo.toml`
  - Add the native file dialog dependency
- Create: `src/project/mod.rs`
  - Define `Project`, `ProjectStatus`, `Connection`, `SshAuth`, and `Project::new_local`
- Create: `src/project/panel.rs`
  - Define `AddProjectForm` with `name`, `selected_dir`, and `visible`
- Modify: `src/main.rs`
  - Swap `mod agent;` for `mod project;`
- Modify: `src/config.rs`
  - Rename `agents` to `projects` and add backward-compatible deserialization from `agents`
- Modify: `src/terminal/mod.rs`
  - Rename `agent_id` to `project_id`
- Modify: `src/app.rs`
  - Rename message/state fields from `Agent` to `Project`
  - Update panel/terminal copy to `Project`
  - Replace manual directory input with native folder selection flow
  - Update inline unit tests to cover renamed behavior and folder-selection form state
- Delete: `src/agent/mod.rs`
  - Replaced by `src/project/mod.rs`
- Delete: `src/agent/panel.rs`
  - Replaced by `src/project/panel.rs`

### Task 1: Add Backward-Compatible `Project` Config Model

**Files:**
- Modify: `Cargo.toml`
- Create: `src/project/mod.rs`
- Modify: `src/config.rs`
- Modify: `src/main.rs`
- Test: `src/config.rs`

- [ ] **Step 1: Add the native dialog dependency**

Update `Cargo.toml`:

```toml
[dependencies]
iced = { version = "0.13", features = ["tokio", "advanced", "canvas"] }
portable-pty = "0.8"
serde = { version = "1", features = ["derive"] }
toml = "0.8"
uuid = { version = "1", features = ["v4", "serde"] }
tokio = { version = "1", features = ["full"] }
dirs = "5"
once_cell = "1"
termwiz = "0.23.3"
rfd = "0.17.2"
```

- [ ] **Step 2: Write failing config compatibility tests**

Add tests to `src/config.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::AppConfig;

    #[test]
    fn load_supports_legacy_agents_field() {
        let config: AppConfig = toml::from_str(
            r#"
            [[agents]]
            id = "00000000-0000-0000-0000-000000000001"
            name = "Legacy Project"
            working_dir = "/tmp/project"
            is_git_repo = false

            [agents.connection]
            type = "local"
            "#,
        )
        .expect("deserialize config");

        assert_eq!(config.projects.len(), 1);
        assert_eq!(config.projects[0].name, "Legacy Project");
    }

    #[test]
    fn save_serializes_projects_field_only() {
        let config = AppConfig {
            projects: vec![crate::project::Project::new_local(
                "Demo".into(),
                std::path::PathBuf::from("/tmp/demo"),
            )],
        };

        let text = toml::to_string_pretty(&config).expect("serialize config");

        assert!(text.contains("[[projects]]"));
        assert!(!text.contains("[[agents]]"));
    }
}
```

- [ ] **Step 3: Run the config tests to verify they fail**

Run: `cargo test config::tests -- --nocapture`

Expected: FAIL because `AppConfig` still exposes `agents` and `crate::project` does not exist yet.

- [ ] **Step 4: Implement the `Project` model**

Create `src/project/mod.rs`:

```rust
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    pub id: Uuid,
    pub name: String,
    pub connection: Connection,
    pub working_dir: PathBuf,
    pub is_git_repo: bool,
    #[serde(skip)]
    pub status: ProjectStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum Connection {
    Local,
    Ssh {
        host: String,
        port: u16,
        user: String,
        auth: SshAuth,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum SshAuth {
    Password(String),
    Key { path: PathBuf, passphrase: Option<String> },
    Agent,
}

#[derive(Debug, Clone, Default)]
pub enum ProjectStatus {
    #[default]
    Disconnected,
    Connected,
    Connecting,
    Error(String),
}

impl Project {
    pub fn new_local(name: String, working_dir: PathBuf) -> Self {
        let is_git_repo = working_dir.join(".git").exists();
        Self {
            id: Uuid::new_v4(),
            name,
            connection: Connection::Local,
            working_dir,
            is_git_repo,
            status: ProjectStatus::Disconnected,
        }
    }
}

pub mod panel;
```

- [ ] **Step 5: Implement backward-compatible config deserialization**

Replace `src/config.rs` with:

```rust
use crate::project::Project;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AppConfig {
    #[serde(default)]
    pub projects: Vec<Project>,
}

#[derive(Debug, Deserialize, Default)]
struct AppConfigCompat {
    #[serde(default)]
    projects: Vec<Project>,
    #[serde(default)]
    agents: Vec<Project>,
}

impl AppConfig {
    pub fn config_path() -> PathBuf {
        dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("teminal-panel")
            .join("config.toml")
    }

    pub fn load() -> Self {
        let path = Self::config_path();
        if !path.exists() {
            return Self::default();
        }

        let content = std::fs::read_to_string(&path).unwrap_or_default();
        let compat: AppConfigCompat = toml::from_str(&content).unwrap_or_default();

        let projects = if compat.projects.is_empty() {
            compat.agents
        } else {
            compat.projects
        };

        Self { projects }
    }

    pub fn save(&self) {
        let path = Self::config_path();
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        if let Ok(content) = toml::to_string_pretty(self) {
            let _ = std::fs::write(&path, content);
        }
    }
}
```

Update `src/main.rs`:

```rust
mod app;
mod config;
mod project;
mod terminal;
```

- [ ] **Step 6: Run the config tests to verify they pass**

Run: `cargo test config::tests -- --nocapture`

Expected: PASS for both compatibility tests.

- [ ] **Step 7: Commit the config/model migration**

```bash
git add Cargo.toml src/config.rs src/main.rs src/project/mod.rs
git commit -m "refactor: migrate config model to projects"
```

### Task 2: Rename App and Terminal State from `Agent` to `Project`

**Files:**
- Create: `src/project/panel.rs`
- Modify: `src/terminal/mod.rs`
- Modify: `src/app.rs`
- Delete: `src/agent/mod.rs`
- Delete: `src/agent/panel.rs`
- Test: `src/app.rs`

- [ ] **Step 1: Write failing app tests for renamed project behavior**

Update or add tests in `src/app.rs`:

```rust
#[test]
fn show_and_hide_add_project_form_updates_visibility_and_resets_fields() {
    let mut app = test_app();

    let _ = app.update(Message::ShowAddProjectForm);
    assert!(app.add_form.visible);

    let _ = app.update(Message::FormNameChanged("Local project".into()));
    let _ = app.update(Message::ProjectFolderSelected(Some(std::path::PathBuf::from("/tmp/demo"))));

    assert_eq!(app.add_form.name, "Local project");
    assert_eq!(
        app.add_form.selected_dir,
        Some(std::path::PathBuf::from("/tmp/demo"))
    );

    let _ = app.update(Message::HideAddProjectForm);
    assert!(!app.add_form.visible);
    assert!(app.add_form.name.is_empty());
    assert_eq!(app.add_form.selected_dir, None);
}

#[test]
fn removing_selected_project_clears_selection() {
    with_temp_config_dir(|workspace_dir| {
        let mut app = test_app();

        let created = app.add_local_project("Local project".into(), workspace_dir.clone());
        assert!(created);

        let project_id = app.config.projects[0].id;
        let _ = app.update(Message::SelectProject(project_id));
        let _ = app.update(Message::RemoveProject(project_id));

        assert_eq!(app.selected_project, None);
        assert!(app.config.projects.is_empty());
    });
}
```

- [ ] **Step 2: Run the app tests to verify they fail**

Run: `cargo test app::tests::show_and_hide_add_project_form_updates_visibility_and_resets_fields app::tests::removing_selected_project_clears_selection -- --nocapture`

Expected: FAIL because the message names, state fields, and form shape still use `Agent`.

- [ ] **Step 3: Rename the form and terminal ownership types**

Create `src/project/panel.rs`:

```rust
#[derive(Debug, Clone, Default)]
pub struct AddProjectForm {
    pub name: String,
    pub selected_dir: Option<std::path::PathBuf>,
    pub visible: bool,
}
```

Update `src/terminal/mod.rs`:

```rust
pub struct TerminalState {
    pub id: Uuid,
    pub project_id: Uuid,
    pub model: model::TerminalModel,
    pub input_buf: String,
    pub writer: Box<dyn std::io::Write + Send>,
    pub lifecycle: Option<pty::PtyLifecycle>,
    pub last_size: Option<model::TerminalSize>,
    pub resize: Box<dyn Fn(model::TerminalSize) -> pty::PtyResult<()> + Send>,
}
```

- [ ] **Step 4: Rename app state, messages, and config usage**

Refactor `src/app.rs`:

```rust
use crate::project::{panel::AddProjectForm, Project};

pub struct App {
    pub config: AppConfig,
    pub selected_project: Option<Uuid>,
    pub add_form: AddProjectForm,
    pub terminals: HashMap<Uuid, TerminalState>,
    pub pty_tx: mpsc::Sender<(Uuid, Vec<u8>)>,
}

#[derive(Debug, Clone)]
pub enum Message {
    SelectProject(Uuid),
    AddProject { name: String, working_dir: PathBuf },
    RemoveProject(Uuid),
    ProjectStatusChanged(Uuid, String),
    ShowAddProjectForm,
    HideAddProjectForm,
    FormNameChanged(String),
    ChooseProjectFolder,
    ProjectFolderSelected(Option<PathBuf>),
    SubmitAddProjectForm,
    OpenTerminal(Uuid),
    PtyOutput(Uuid, Vec<u8>),
    TerminalViewportChanged(Uuid, TerminalViewport),
    TerminalInput(Uuid, String),
    InputChanged(Uuid, String),
}
```

Replace all `config.agents` references with `config.projects`, `selected_agent` with `selected_project`, and `agent_id` locals with `project_id`.

- [ ] **Step 5: Remove the old `agent` module**

Delete the old files after `src/app.rs` compiles against `crate::project`:

```bash
rm src/agent/mod.rs src/agent/panel.rs
```

- [ ] **Step 6: Run the renamed app tests to verify they pass**

Run: `cargo test app::tests::show_and_hide_add_project_form_updates_visibility_and_resets_fields app::tests::removing_selected_project_clears_selection -- --nocapture`

Expected: PASS for the renamed behavior tests.

- [ ] **Step 7: Commit the semantic rename**

```bash
git add src/app.rs src/project/panel.rs src/terminal/mod.rs src/project/mod.rs src/main.rs
git rm src/agent/mod.rs src/agent/panel.rs
git commit -m "refactor: rename agent UI state to project"
```

### Task 3: Replace Manual Directory Input with Native Folder Selection

**Files:**
- Modify: `src/app.rs`
- Test: `src/app.rs`

- [ ] **Step 1: Write failing tests for folder-selection submission rules**

Add tests in `src/app.rs`:

```rust
#[test]
fn submit_add_project_form_adds_project_and_resets_form() {
    with_temp_config_dir(|workspace_dir| {
        let mut app = test_app();

        let _ = app.update(Message::ShowAddProjectForm);
        let _ = app.update(Message::FormNameChanged("Local project".into()));
        let _ = app.update(Message::ProjectFolderSelected(Some(workspace_dir.clone())));
        let _ = app.update(Message::SubmitAddProjectForm);

        assert_eq!(app.config.projects.len(), 1);
        assert_eq!(app.config.projects[0].name, "Local project");
        assert_eq!(app.config.projects[0].working_dir, workspace_dir);
        assert!(!app.add_form.visible);
        assert_eq!(app.add_form.selected_dir, None);
    });
}

#[test]
fn submit_add_project_form_requires_selected_directory() {
    let mut app = test_app();

    let _ = app.update(Message::ShowAddProjectForm);
    let _ = app.update(Message::FormNameChanged("Local project".into()));
    let _ = app.update(Message::SubmitAddProjectForm);

    assert!(app.config.projects.is_empty());
    assert!(app.add_form.visible);
}

#[test]
fn project_folder_selected_none_preserves_existing_selection() {
    let mut app = test_app();

    let _ = app.update(Message::ShowAddProjectForm);
    let _ = app.update(Message::ProjectFolderSelected(Some(std::path::PathBuf::from("/tmp/demo"))));
    let _ = app.update(Message::ProjectFolderSelected(None));

    assert_eq!(
        app.add_form.selected_dir,
        Some(std::path::PathBuf::from("/tmp/demo"))
    );
}
```

- [ ] **Step 2: Run the new form tests to verify they fail**

Run: `cargo test app::tests::submit_add_project_form_adds_project_and_resets_form app::tests::submit_add_project_form_requires_selected_directory app::tests::project_folder_selected_none_preserves_existing_selection -- --nocapture`

Expected: FAIL because the add form still uses a string directory field and there is no folder picker flow.

- [ ] **Step 3: Implement `add_local_project` using `PathBuf`**

In `src/app.rs`, replace the creation helper with:

```rust
fn add_local_project(&mut self, name: String, working_dir: PathBuf) -> bool {
    let name = name.trim().to_string();

    if name.is_empty() || !working_dir.is_dir() {
        return false;
    }

    self.config.projects.push(Project::new_local(name, working_dir));
    self.config.save();
    true
}
```

- [ ] **Step 4: Implement the native folder picker task**

Add the message handlers in `src/app.rs`:

```rust
Message::ChooseProjectFolder => {
    return Task::perform(
        async {
            rfd::AsyncFileDialog::new()
                .pick_folder()
                .await
                .map(|handle| handle.path().to_path_buf())
        },
        Message::ProjectFolderSelected,
    );
}
Message::ProjectFolderSelected(selection) => {
    if let Some(path) = selection {
        self.add_form.selected_dir = Some(path);
    }
}
Message::SubmitAddProjectForm => {
    if let Some(path) = self.add_form.selected_dir.clone() {
        if self.add_local_project(self.add_form.name.clone(), path) {
            self.add_form = Default::default();
        }
    }
}
```

- [ ] **Step 5: Replace the form UI with folder-picker controls**

Update the add-project section in `src/app.rs`:

```rust
let selected_dir = self
    .add_form
    .selected_dir
    .as_ref()
    .map(|path| path.display().to_string())
    .unwrap_or_else(|| "No folder selected".into());

let add_section: Element<'_, Message> = if self.add_form.visible {
    column![
        text_input("Name", &self.add_form.name)
            .on_input(Message::FormNameChanged)
            .on_submit(Message::SubmitAddProjectForm),
        text(selected_dir).size(12),
        button(text("Choose Folder")).on_press(Message::ChooseProjectFolder),
        row![
            button(text("Add")).on_press(Message::SubmitAddProjectForm),
            button(text("Cancel")).on_press(Message::HideAddProjectForm),
        ]
        .spacing(6),
    ]
    .spacing(6)
    .into()
} else {
    button(text("+ Add Project"))
        .on_press(Message::ShowAddProjectForm)
        .into()
};
```

- [ ] **Step 6: Run the form tests to verify they pass**

Run: `cargo test app::tests::submit_add_project_form_adds_project_and_resets_form app::tests::submit_add_project_form_requires_selected_directory app::tests::project_folder_selected_none_preserves_existing_selection -- --nocapture`

Expected: PASS for the folder-selection and submission rules.

- [ ] **Step 7: Commit the folder picker implementation**

```bash
git add src/app.rs Cargo.toml
git commit -m "feat: add native folder picker for projects"
```

### Task 4: Finish Copy Migration and Run Full Verification

**Files:**
- Modify: `src/app.rs`
- Test: `src/app.rs`

- [ ] **Step 1: Write failing copy-focused tests**

Add or update `src/app.rs` tests to assert persistence and git repo detection still use `projects`:

```rust
#[test]
fn add_local_project_marks_git_repo_when_dot_git_exists() {
    with_temp_config_dir(|workspace_dir| {
        std::fs::create_dir_all(workspace_dir.join(".git")).expect("create git dir");

        let mut app = test_app();
        let created = app.add_local_project("Repo project".into(), workspace_dir.clone());

        assert!(created);
        assert!(app.config.projects[0].is_git_repo);
    });
}
```

Update existing terminal/selection tests to use `Project`-named messages and fields.

- [ ] **Step 2: Run the targeted verification and confirm failures if any old names remain**

Run: `cargo test app::tests -- --nocapture`

Expected: any failures should point to stale `Agent` names or config references.

- [ ] **Step 3: Complete the remaining copy migration**

In `src/app.rs`, update user-visible strings:

```rust
text("Projects").size(24)
text(format!("Project: {}", project.name)).size(24)
text("Project not found").size(24)
text("Select a project to open a terminal").size(24)
```

Keep `"Open Terminal"` unchanged.

- [ ] **Step 4: Run the full test suite**

Run: `cargo test`

Expected: PASS for the entire suite.

- [ ] **Step 5: Run compile verification**

Run: `cargo check`

Expected: PASS with no compile errors.

- [ ] **Step 6: Inspect the working tree**

Run: `git status --short`

Expected: only the intended project-rename and folder-picker changes are present.

- [ ] **Step 7: Commit the completed feature**

```bash
git add Cargo.toml src/app.rs src/config.rs src/main.rs src/project src/terminal/mod.rs
git commit -m "feat: rename agents to projects with folder picker"
```
