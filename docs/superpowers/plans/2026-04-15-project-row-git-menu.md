# Project Row Git Menu Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a three-dots button to each left-side project row that opens a per-project context menu, with a Git icon + `Git` label as the first item only for git repositories.

**Architecture:** Extend `App` state with a single open project-menu identifier and new messages to toggle or dismiss that menu. Update the project row renderer in `teminal-panel/src/view/project_panel.rs` to place a trailing kebab button beside the existing close button and render a `ContextMenu` overlay anchored to the clicked row. Non-git projects still show the three-dots button, but their menu omits the Git item.

**Tech Stack:** Rust, Iced 0.13 widgets/layout, `iced_fonts::bootstrap`, existing `teminal_ui::components::ContextMenu`, cargo test

---

## File Structure

- **Modify:** `teminal-panel/src/app.rs`
  - Add project-row menu state to `App`
  - Add new `Message` variants for toggling and dismissing a project menu
  - Update `App::new()` and `App::update()` so project menus open, close, and reset consistently alongside existing hover/selection/removal behavior
- **Modify:** `teminal-panel/src/view/project_panel.rs`
  - Add the trailing three-dots button to each project row
  - Render the per-project context menu next to the corresponding row
  - Keep the existing close button behavior unchanged
  - Only include the Git menu item when `project.is_git_repo` is true
- **Modify:** `teminal-panel/src/app_test.rs`
  - Add state-focused tests for toggling, switching, dismissing, and clearing project row menus
- **Do not modify:** `ui/src/components/context_menu.rs`
  - Reuse as-is unless implementation proves it cannot support the menu width or styling required here

### Task 1: Add project menu state and messages

**Files:**
- Modify: `teminal-panel/src/app.rs:18-80`
- Test: `teminal-panel/src/app_test.rs`

- [ ] **Step 1: Write the failing tests**

```rust
#[test]
fn toggle_project_menu_opens_and_closes_selected_project_menu() {
    let mut app = test_app();
    let project_id = Uuid::new_v4();

    let _ = app.update(Message::ToggleProjectMenu(project_id));
    assert_eq!(app.open_project_menu, Some(project_id));

    let _ = app.update(Message::ToggleProjectMenu(project_id));
    assert_eq!(app.open_project_menu, None);
}

#[test]
fn toggle_project_menu_switches_open_menu_between_projects() {
    let mut app = test_app();
    let first = Uuid::new_v4();
    let second = Uuid::new_v4();

    let _ = app.update(Message::ToggleProjectMenu(first));
    let _ = app.update(Message::ToggleProjectMenu(second));

    assert_eq!(app.open_project_menu, Some(second));
}

#[test]
fn hide_project_menu_clears_open_menu_state() {
    let mut app = test_app();
    let project_id = Uuid::new_v4();

    let _ = app.update(Message::ToggleProjectMenu(project_id));
    let _ = app.update(Message::HideProjectMenu);

    assert_eq!(app.open_project_menu, None);
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test toggle_project_menu_opens_and_closes_selected_project_menu toggle_project_menu_switches_open_menu_between_projects hide_project_menu_clears_open_menu_state`
Expected: FAIL with missing `ToggleProjectMenu`, `HideProjectMenu`, or `open_project_menu`

- [ ] **Step 3: Write the minimal implementation**

Add the new field to `App` and initialize it to `None`:

```rust
pub struct App {
    pub config: AppConfig,
    pub selected_project: Option<Uuid>,
    pub hovered_project: Option<Uuid>,
    pub open_project_menu: Option<Uuid>,
    pub expanded_projects: HashSet<Uuid>,
    // ...
}
```

```rust
Self {
    config: AppConfig::load(),
    selected_project: None,
    hovered_project: None,
    open_project_menu: None,
    expanded_projects: HashSet::new(),
    // ...
}
```

Add the message variants:

```rust
pub enum Message {
    SelectProject(Uuid),
    #[cfg(test)]
    AddProject { name: String, working_dir: String },
    RemoveProject(Uuid),
    HoverProject(Option<Uuid>),
    ToggleProjectMenu(Uuid),
    HideProjectMenu,
    ShowAddProjectForm,
    // ...
}
```

Handle them in `App::update()`:

```rust
Message::ToggleProjectMenu(id) => {
    self.settings_menu_open = false;
    self.open_project_menu = if self.open_project_menu == Some(id) {
        None
    } else {
        Some(id)
    };
}
Message::HideProjectMenu => {
    self.open_project_menu = None;
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test toggle_project_menu_opens_and_closes_selected_project_menu toggle_project_menu_switches_open_menu_between_projects hide_project_menu_clears_open_menu_state`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add teminal-panel/src/app.rs teminal-panel/src/app_test.rs
git commit -m "feat(ui): add project menu state"
```

### Task 2: Clear project menu state during conflicting project actions

**Files:**
- Modify: `teminal-panel/src/app.rs:144-260`
- Test: `teminal-panel/src/app_test.rs`

- [ ] **Step 1: Write the failing tests**

```rust
#[test]
fn removing_project_clears_open_project_menu_for_that_project() {
    with_temp_config_dir(|workspace_dir: &PathBuf| {
        let mut app = test_app();

        let _ = app.update(Message::AddProject {
            name: "Local project".into(),
            working_dir: workspace_dir.display().to_string(),
        });

        let project_id = app.config.projects[0].id;
        let _ = app.update(Message::ToggleProjectMenu(project_id));
        let _ = app.update(Message::RemoveProject(project_id));

        assert_eq!(app.open_project_menu, None);
    });
}

#[test]
fn opening_overlay_closes_open_project_menu() {
    let mut app = test_app();
    let project_id = Uuid::new_v4();

    let _ = app.update(Message::ToggleProjectMenu(project_id));
    let _ = app.update(Message::ShowAddProjectForm);

    assert_eq!(app.open_project_menu, None);
}

#[test]
fn opening_settings_menu_closes_open_project_menu() {
    let mut app = test_app();
    let project_id = Uuid::new_v4();

    let _ = app.update(Message::ToggleProjectMenu(project_id));
    let _ = app.update(Message::ToggleSettingsMenu);

    assert_eq!(app.open_project_menu, None);
    assert!(app.settings_menu_open);
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test removing_project_clears_open_project_menu_for_that_project opening_overlay_closes_open_project_menu opening_settings_menu_closes_open_project_menu`
Expected: FAIL because menu state is not cleared yet

- [ ] **Step 3: Write the minimal implementation**

Update existing branches in `App::update()` so unrelated overlays and removed projects clear the per-project menu:

```rust
Message::RemoveProject(id) => {
    self.config.projects.retain(|project| project.id != id);
    self.terminals.remove(&id);

    if self.selected_project == Some(id) {
        self.selected_project = None;
    }
    if self.hovered_project == Some(id) {
        self.hovered_project = None;
    }
    if self.open_project_menu == Some(id) {
        self.open_project_menu = None;
    }

    self.config.save();
}
```

```rust
Message::ShowAddProjectForm => {
    self.add_form = Default::default();
    self.settings_menu_open = false;
    self.open_project_menu = None;
    self.overlay = Some(OverlayState::AddProject);
}
```

```rust
Message::ToggleSettingsMenu => {
    self.open_project_menu = None;
    self.settings_menu_open = !self.settings_menu_open;
}
```

Also clear `open_project_menu` in these branches because they already shift focus away from the project row:

```rust
Message::ShowSshServices => { /* set self.open_project_menu = None; */ }
Message::HideOverlay => { /* set self.open_project_menu = None; */ }
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test removing_project_clears_open_project_menu_for_that_project opening_overlay_closes_open_project_menu opening_settings_menu_closes_open_project_menu`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add teminal-panel/src/app.rs teminal-panel/src/app_test.rs
git commit -m "fix(ui): reset project menu state"
```

### Task 3: Render the three-dots button in each project row

**Files:**
- Modify: `teminal-panel/src/view/project_panel.rs:9-90`
- Test: `teminal-panel/src/app_test.rs`

- [ ] **Step 1: Write the failing test**

Add a focused state test that proves the new button message path can coexist with hover-driven close visibility:

```rust
#[test]
fn hover_state_does_not_block_project_menu_toggle() {
    let mut app = test_app();
    let project_id = Uuid::new_v4();

    let _ = app.update(Message::HoverProject(Some(project_id)));
    let _ = app.update(Message::ToggleProjectMenu(project_id));

    assert_eq!(app.hovered_project, Some(project_id));
    assert_eq!(app.open_project_menu, Some(project_id));
}
```

- [ ] **Step 2: Run test to verify it fails if needed**

Run: `cargo test hover_state_does_not_block_project_menu_toggle`
Expected: FAIL only if Task 1 state wiring is incomplete; otherwise PASS and continue

- [ ] **Step 3: Write the minimal implementation**

In `teminal-panel/src/view/project_panel.rs`, add a menu trigger button beside the existing close button:

```rust
let menu_button = button(bootstrap::three_dots_vertical().size(10))
    .on_press(Message::ToggleProjectMenu(project.id))
    .padding([4, 4])
    .style(button::text);

let row_content = row![chevron_btn, project_button, menu_button, close_btn]
    .spacing(2)
    .align_y(iced::alignment::Vertical::Center);
```

Keep `close_btn` logic exactly as-is so the user retains the current remove-project affordance.

- [ ] **Step 4: Run tests to verify state still passes**

Run: `cargo test hover_state_does_not_block_project_menu_toggle removing_selected_project_clears_selection`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add teminal-panel/src/view/project_panel.rs teminal-panel/src/app_test.rs
git commit -m "feat(ui): add project row menu button"
```

### Task 4: Render the per-project context menu with a conditional Git item

**Files:**
- Modify: `teminal-panel/src/view/project_panel.rs:57-90`
- Test: `teminal-panel/src/app_test.rs`

- [ ] **Step 1: Write the failing tests**

Add state tests that lock down the visibility rule the UI will depend on:

```rust
#[test]
fn local_git_project_retains_git_repo_flag() {
    with_temp_config_dir(|workspace_dir: &PathBuf| {
        std::fs::create_dir_all(workspace_dir.join(".git")).expect("create git dir");

        let mut app = test_app();
        let _ = app.update(Message::AddProject {
            name: "repo".into(),
            working_dir: workspace_dir.display().to_string(),
        });

        assert!(app.config.projects[0].is_git_repo);
    });
}

#[test]
fn ssh_project_does_not_expose_git_repo_flag() {
    let service = sample_ssh_service();
    let mut app = test_app();
    app.config.ssh_services.push(service.clone());
    app.add_form.connection_kind = ProjectConnectionKind::Ssh;
    app.add_form.ssh_service_id = Some(service.id);
    app.add_form.name = "remote".into();
    app.add_form.selected_dir = Some(PathBuf::from("/srv/app"));

    let _ = app.update(Message::SubmitAddProjectForm);

    assert!(!app.config.projects[0].is_git_repo);
}
```

- [ ] **Step 2: Run tests to verify they fail if assumptions are wrong**

Run: `cargo test local_git_project_retains_git_repo_flag ssh_project_does_not_expose_git_repo_flag`
Expected: PASS in the current codebase; if so, keep the tests as regression coverage and continue

- [ ] **Step 3: Write the minimal implementation**

In `teminal-panel/src/view/project_panel.rs`, build menu content per project and only push the Git item when `project.is_git_repo` is true:

```rust
let project_menu: Option<Element<'_, Message>> = if self.open_project_menu == Some(project.id) {
    let mut items = column![].spacing(4);

    if project.is_git_repo {
        items = items.push(
            button(
                row![bootstrap::git().size(12), text("Git").size(12)]
                    .spacing(6)
                    .align_y(iced::alignment::Vertical::Center),
            )
            .width(Length::Fill)
            .style(button::text)
            .padding([6, 8]),
        );
    }

    Some(ContextMenu::new(items.into()).width(Length::Fixed(160.0)).into_element())
} else {
    None
};
```

Render that menu in a stacked row container so it appears alongside the clicked project row without affecting other rows:

```rust
let row_layer: Element<'_, Message> = if let Some(menu) = project_menu {
    iced::widget::stack![
        mouse_area(container(text(""))
            .width(Length::Fill)
            .height(Length::Fill))
            .on_press(Message::HideProjectMenu),
        container(
            column![container(menu).align_right(Length::Shrink), row_container]
                .spacing(0)
                .align_x(iced::alignment::Horizontal::Right)
        )
        .width(Length::Fill)
    ]
    .into()
} else {
    row_container.into()
};
```

Then keep hover tracking on the outer `mouse_area` around `row_layer`.

- [ ] **Step 4: Run tests to verify behavior still passes**

Run: `cargo test local_git_project_retains_git_repo_flag ssh_project_does_not_expose_git_repo_flag hover_state_does_not_block_project_menu_toggle`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add teminal-panel/src/view/project_panel.rs teminal-panel/src/app_test.rs
git commit -m "feat(ui): show project git menu item"
```

### Task 5: Verify the full feature end-to-end

**Files:**
- Modify: `teminal-panel/src/app.rs` (only if verification exposes a bug)
- Modify: `teminal-panel/src/view/project_panel.rs` (only if verification exposes a bug)
- Modify: `teminal-panel/src/app_test.rs` (only if verification exposes a bug)

- [ ] **Step 1: Run focused test coverage**

Run: `cargo test toggle_project_menu_opens_and_closes_selected_project_menu toggle_project_menu_switches_open_menu_between_projects hide_project_menu_clears_open_menu_state removing_project_clears_open_project_menu_for_that_project opening_overlay_closes_open_project_menu opening_settings_menu_closes_open_project_menu hover_state_does_not_block_project_menu_toggle local_git_project_retains_git_repo_flag ssh_project_does_not_expose_git_repo_flag`
Expected: PASS

- [ ] **Step 2: Run the full test suite**

Run: `cargo test`
Expected: PASS

- [ ] **Step 3: Run formatting**

Run: `cargo fmt --check`
Expected: PASS

- [ ] **Step 4: Manual UI verification**

Run: `cargo run`
Expected:
- Every project row shows a three-dots button at the right side
- Existing close button still behaves exactly as before
- Clicking a row’s three-dots button opens only that row’s menu
- Clicking the same button again closes that row’s menu
- Git repositories show `git icon + Git` as the first menu item
- Non-git projects open a menu without the Git item
- Opening the settings menu or add-project overlay closes any open project menu

- [ ] **Step 5: Commit**

```bash
git add teminal-panel/src/app.rs teminal-panel/src/view/project_panel.rs teminal-panel/src/app_test.rs
git commit -m "feat(ui): add project row git menu"
```

## Self-Review

- **Spec coverage:** Covered the approved scope: trailing three-dots button, per-project context menu, Git item only for git repositories, non-git rows still showing the button, existing close button preserved, left project list row only.
- **Placeholder scan:** No `TODO`/`TBD` placeholders remain; each code-changing step includes concrete code or commands.
- **Type consistency:** Plan consistently uses `open_project_menu: Option<Uuid>`, `Message::ToggleProjectMenu(Uuid)`, and `Message::HideProjectMenu` across state, tests, and view rendering.
