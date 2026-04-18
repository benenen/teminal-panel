# Git Window Phase 2 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add real multi-window Git support so each Git-backed project can open one independent OS window with a left-side changes list and a right-side commit graph.

**Architecture:** Keep `App` as the source of truth for window lifecycle and route Git windows by both `project_id` and `window::Id`. Extend the existing `git_window` module instead of replacing it: add commit-history loading, a canvas-based graph view, and window cleanup logic tied to Iced window events.

**Tech Stack:** Rust, Iced 0.14, git2-rs 0.19, Iced canvas

---

## File Structure

**Create:**
- `teminal-panel/src/git_window/graph.rs` - Canvas program and graph rendering helpers for simple commit history

**Modify:**
- `teminal-panel/src/app.rs` - Track Git windows, open/focus windows, and clean up closed windows
- `teminal-panel/src/app_test.rs` - Add focused tests for Git window state management helpers
- `teminal-panel/src/git_window/mod.rs` - Add split layout, graph pane integration, and per-window state
- `teminal-panel/src/git_window/git_data.rs` - Add commit-history loading for the graph
- `teminal-panel/src/main.rs` - Adjust application wiring only if needed for multi-window routing

---

### Task 1: Add App-Level Git Window State

**Files:**
- Modify: `teminal-panel/src/app.rs`
- Test: `teminal-panel/src/app_test.rs`

- [ ] **Step 1: Write the failing tests**

Add tests for pure state helpers that assert:
- opening a Git window for a non-Git project is rejected
- a project with an existing Git window does not allocate a duplicate
- window close cleanup removes both project and window ID mappings

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p teminal-panel git_window_state_ -- --nocapture`
Expected: FAIL because the helper/state does not exist yet

- [ ] **Step 3: Write minimal implementation**

In `teminal-panel/src/app.rs`:
- add `GitWindowState`
- add `git_windows_by_project: HashMap<Uuid, GitWindowState>`
- add `git_window_projects_by_id: HashMap<iced::window::Id, Uuid>`
- add small helper methods that keep these mappings consistent

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p teminal-panel git_window_state_ -- --nocapture`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add teminal-panel/src/app.rs teminal-panel/src/app_test.rs
git commit -m "feat(git-window): track git window state in app"
```

---

### Task 2: Load Commit History Data

**Files:**
- Modify: `teminal-panel/src/git_window/git_data.rs`
- Test: `teminal-panel/src/app_test.rs`

- [ ] **Step 1: Write the failing tests**

Add tests for commit data helpers that assert:
- non-repo paths fail cleanly
- a repo with at least one commit returns at least one `CommitNode`
- the returned node includes short ID and summary text

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p teminal-panel git_commit_history_ -- --nocapture`
Expected: FAIL because commit-history loading is not implemented yet

- [ ] **Step 3: Write minimal implementation**

In `teminal-panel/src/git_window/git_data.rs`:
- add `CommitNode`
- add `get_commit_history(repo_path: &Path, limit: usize) -> Result<Vec<CommitNode>, git2::Error>`
- use `Repository::revwalk()` from `HEAD`
- capture `oid`, `short_id`, `summary`, `author`, `timestamp`, and `parent_count`

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p teminal-panel git_commit_history_ -- --nocapture`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add teminal-panel/src/git_window/git_data.rs teminal-panel/src/app_test.rs
git commit -m "feat(git-window): load commit history for graph"
```

---

### Task 3: Render a Simple Commit Graph

**Files:**
- Create: `teminal-panel/src/git_window/graph.rs`
- Modify: `teminal-panel/src/git_window/mod.rs`

- [ ] **Step 1: Write the failing build step**

Wire `mod graph;` and call a graph view function from `GitWindow::view()` before implementing the new module fully.

Run: `cargo build`
Expected: FAIL because the graph module or graph view function is missing

- [ ] **Step 2: Write minimal implementation**

Create `teminal-panel/src/git_window/graph.rs` with:
- a small canvas program that receives `&[CommitNode]`
- one-lane drawing only
- node circles, vertical connectors, short hash, and summary text

Update `teminal-panel/src/git_window/mod.rs` to:
- load commit history in `GitWindow::new`
- keep file list on the left
- render the graph pane on the right
- use a fixed-width left pane and fill-width right pane

- [ ] **Step 3: Run build to verify it passes**

Run: `cargo build`
Expected: PASS

- [ ] **Step 4: Commit**

```bash
git add teminal-panel/src/git_window/graph.rs teminal-panel/src/git_window/mod.rs teminal-panel/src/git_window/git_data.rs
git commit -m "feat(git-window): render simple commit graph"
```

---

### Task 4: Open and Focus Real Git Windows

**Files:**
- Modify: `teminal-panel/src/app.rs`
- Modify: `teminal-panel/src/main.rs` (only if application wiring requires explicit multi-window routing)
- Test: `teminal-panel/src/app_test.rs`

- [ ] **Step 1: Write the failing tests**

Add tests for message handling helpers that assert:
- `OpenGitWindow(project_id)` creates a new tracked Git window for a Git repo
- opening the same project twice reuses existing tracked state instead of duplicating it

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p teminal-panel open_git_window_ -- --nocapture`
Expected: FAIL because window open/focus logic is incomplete

- [ ] **Step 3: Write minimal implementation**

In `teminal-panel/src/app.rs`:
- extend `Message` with explicit Git-window lifecycle messages as needed:
  - open request
  - window opened with returned `window::Id`
  - window close cleanup
- on `OpenGitWindow(project_id)`:
  - reject non-Git projects
  - focus existing Git window if present
  - otherwise create a new Iced window with Git content
- update titles/view routing by `window::Id` as needed for the additional window

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p teminal-panel open_git_window_ -- --nocapture`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add teminal-panel/src/app.rs teminal-panel/src/app_test.rs teminal-panel/src/main.rs
git commit -m "feat(git-window): open and focus git windows"
```

---

### Task 5: Clean Up Closed Git Windows

**Files:**
- Modify: `teminal-panel/src/app.rs`
- Test: `teminal-panel/src/app_test.rs`

- [ ] **Step 1: Write the failing test**

Add a test that simulates a Git window close event and asserts:
- the `window::Id -> project_id` mapping is removed
- the `project_id -> GitWindowState` mapping is removed

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p teminal-panel close_git_window_ -- --nocapture`
Expected: FAIL because cleanup is incomplete

- [ ] **Step 3: Write minimal implementation**

Handle the relevant Iced window-close event in `teminal-panel/src/app.rs` and remove both mappings consistently.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p teminal-panel close_git_window_ -- --nocapture`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add teminal-panel/src/app.rs teminal-panel/src/app_test.rs
git commit -m "fix(git-window): clean up state on window close"
```

---

### Task 6: End-to-End Verification

**Files:**
- None

- [ ] **Step 1: Format**

Run: `cargo fmt`
Expected: PASS

- [ ] **Step 2: Run focused tests**

Run: `cargo test -p teminal-panel 'git_window_state_|git_commit_history_|open_git_window_|close_git_window_' -- --nocapture`
Expected: PASS

- [ ] **Step 3: Run full non-doctest suite**

Run: `cargo test --lib --bins`
Expected: PASS

- [ ] **Step 4: Run the app manually**

Run: `cargo run -p teminal-panel`
Expected: Main window starts and opening a Git project launches a real Git window

- [ ] **Step 5: Commit final polish if needed**

```bash
git add -A
git commit -m "chore(git-window): finalize phase 2 multi-window support"
```
