# Git Compare Editor Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the Git window patch preview with a VS Code-like compare editor that supports text-file editing with explicit `Apply` / `Discard`, while gracefully handling binary files as non-editable details.

**Architecture:** Keep the existing Git window shell and left-side file list, but refactor file detail into an explicit compare-editor state model. Move file-detail loading and persistence into `git_window/git_data.rs`, keep the Git window state transitions in `git_window/mod.rs`, and render text vs binary detail panes separately so the UI flow stays testable and focused.

**Tech Stack:** Rust, Iced 0.14, `iced::widget::text_editor`, git2-rs 0.19, standard filesystem I/O

---

## File Structure

**Create:**
- `teminal-panel/src/git_window/detail.rs` - Compare editor rendering helpers and file-detail view components for text vs binary states

**Modify:**
- `teminal-panel/src/git_window/git_data.rs` - Load base content, working tree content, binary/text classification, patch refresh, and working-tree writes
- `teminal-panel/src/git_window/mod.rs` - Replace `selected_diff` preview state with explicit compare-editor state and wire new messages
- `teminal-panel/src/app_test.rs` - Extend focused regression coverage only if app-level Git window routing needs adjustment after detail-state changes

---

### Task 1: Add File Detail Data Helpers

**Files:**
- Modify: `teminal-panel/src/git_window/git_data.rs`
- Test: `teminal-panel/src/git_window/git_data.rs`

- [ ] **Step 1: Write the failing tests**

Add helper tests covering:
- reading base revision text for a tracked file
- reading working tree text for the same file
- detecting binary content for a non-text file
- writing updated working tree text back to disk

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p teminal-panel 'git_file_detail_|git_binary_file_|git_write_worktree_' -- --nocapture`
Expected: FAIL because the new file-detail helpers do not exist yet

- [ ] **Step 3: Write minimal implementation**

In `teminal-panel/src/git_window/git_data.rs`:
- add a file-kind enum for text vs binary detail loading
- add `get_base_file_content(repo_path: &Path, file_path: &Path) -> Result<Option<Vec<u8>>, git2::Error>`
- add `get_worktree_file_content(repo_path: &Path, file_path: &Path) -> Result<Vec<u8>, std::io::Error>`
- add `classify_file_content(bytes: &[u8]) -> FileContentKind`
- add `write_worktree_file(repo_path: &Path, file_path: &Path, contents: &str) -> Result<(), std::io::Error>`
- keep the existing patch helper available for refresh after writes

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p teminal-panel 'git_file_detail_|git_binary_file_|git_write_worktree_' -- --nocapture`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add teminal-panel/src/git_window/git_data.rs
git commit -m "feat(git-window): add compare editor file data helpers"
```

---

### Task 2: Introduce Explicit Compare Editor State

**Files:**
- Modify: `teminal-panel/src/git_window/mod.rs`
- Test: `teminal-panel/src/git_window/mod.rs`

- [ ] **Step 1: Write the failing tests**

Add Git window state tests asserting:
- selecting a text file initializes compare-editor state with base and working tree content
- selecting a binary file enters a non-editable binary detail state
- editing the working tree pane marks the detail state dirty

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p teminal-panel 'git_window_detail_|selecting_text_file_|selecting_binary_file_' -- --nocapture`
Expected: FAIL because the Git window still stores only `selected_diff`

- [ ] **Step 3: Write minimal implementation**

In `teminal-panel/src/git_window/mod.rs`:
- replace `selected_diff` with a dedicated selected-file detail structure
- store selected path, file kind, base content, working tree content, editor draft, dirty state, and detail error
- add Git window messages for:
  - file selection
  - draft edit
  - apply
  - discard
- on file selection, load file detail through the new helpers

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p teminal-panel 'git_window_detail_|selecting_text_file_|selecting_binary_file_' -- --nocapture`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add teminal-panel/src/git_window/mod.rs
git commit -m "feat(git-window): track compare editor detail state"
```

---

### Task 3: Render Text and Binary Detail Panes

**Files:**
- Create: `teminal-panel/src/git_window/detail.rs`
- Modify: `teminal-panel/src/git_window/mod.rs`

- [ ] **Step 1: Write the failing build step**

Wire `mod detail;` into `git_window/mod.rs` and route the right-side pane through a detail view function before creating the new module.

Run: `cargo build`
Expected: FAIL because the detail module or detail view helpers are missing

- [ ] **Step 2: Write minimal implementation**

Create `teminal-panel/src/git_window/detail.rs` with:
- a text compare editor view:
  - header with file path and dirty status
  - read-only base pane
  - editable working tree pane
  - `Apply` / `Discard` action row
- a binary detail view:
  - file path
  - binary-file status
  - editing-not-supported message

Update `teminal-panel/src/git_window/mod.rs` to:
- make file rows clickable
- highlight the selected file
- route the right pane to text compare editor vs binary detail view

- [ ] **Step 3: Run build to verify it passes**

Run: `cargo build`
Expected: PASS

- [ ] **Step 4: Commit**

```bash
git add teminal-panel/src/git_window/detail.rs teminal-panel/src/git_window/mod.rs
git commit -m "feat(git-window): render compare editor detail pane"
```

---

### Task 4: Implement Apply and Discard Behavior

**Files:**
- Modify: `teminal-panel/src/git_window/mod.rs`
- Modify: `teminal-panel/src/git_window/git_data.rs`
- Test: `teminal-panel/src/git_window/mod.rs`

- [ ] **Step 1: Write the failing tests**

Add focused tests asserting:
- `Apply` writes the draft text to disk and refreshes working-tree detail state
- `Discard` restores the editable pane from the current on-disk file content
- apply failures preserve the in-memory draft and surface an error

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p teminal-panel 'apply_selected_file_|discard_selected_file_|apply_failure_' -- --nocapture`
Expected: FAIL because apply/discard behavior is incomplete

- [ ] **Step 3: Write minimal implementation**

In `teminal-panel/src/git_window/mod.rs`:
- implement `ApplySelectedFile` to:
  - write the draft via `write_worktree_file`
  - reload working tree text and patch state
  - refresh file list and dirty state
- implement `DiscardSelectedFile` to:
  - reload from current disk contents
  - rebuild the draft editor state
  - clear dirty state
- preserve the draft when an apply error occurs

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p teminal-panel 'apply_selected_file_|discard_selected_file_|apply_failure_' -- --nocapture`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add teminal-panel/src/git_window/mod.rs teminal-panel/src/git_window/git_data.rs
git commit -m "feat(git-window): support apply and discard in compare editor"
```

---

### Task 5: Refresh Git Window File State After Edits

**Files:**
- Modify: `teminal-panel/src/git_window/mod.rs`
- Test: `teminal-panel/src/git_window/mod.rs`

- [ ] **Step 1: Write the failing tests**

Add tests asserting:
- after `Apply`, the file list still reflects current git status
- when a file becomes clean after apply, the detail state refreshes without stale patch content
- selecting another file after apply loads the new compare state correctly

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p teminal-panel 'refresh_git_window_after_apply_|file_list_refresh_' -- --nocapture`
Expected: FAIL because the file list/detail refresh behavior is incomplete

- [ ] **Step 3: Write minimal implementation**

In `teminal-panel/src/git_window/mod.rs`:
- after apply, reload file changes through `get_file_changes`
- if the selected file no longer appears in the changed-file list, keep the detail pane stable enough to show the saved working tree content without stale patch state
- ensure subsequent file selection still uses the refreshed file list

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p teminal-panel 'refresh_git_window_after_apply_|file_list_refresh_' -- --nocapture`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add teminal-panel/src/git_window/mod.rs
git commit -m "fix(git-window): refresh file state after compare editor apply"
```

---

### Task 6: End-to-End Verification

**Files:**
- None

- [ ] **Step 1: Format**

Run: `cargo fmt`
Expected: PASS

- [ ] **Step 2: Run focused compare editor tests**

Run: `cargo test -p teminal-panel 'git_file_detail_|git_binary_file_|git_write_worktree_|git_window_detail_|selecting_text_file_|selecting_binary_file_|apply_selected_file_|discard_selected_file_|apply_failure_|refresh_git_window_after_apply_|file_list_refresh_' -- --nocapture`
Expected: PASS

- [ ] **Step 3: Run full non-doctest suite**

Run: `cargo test --lib --bins`
Expected: PASS

- [ ] **Step 4: Run the app manually**

Run: `cargo run -p teminal-panel`
Expected:
- main window starts
- clicking Git opens the Git window
- clicking a text file opens the compare editor
- editing stays in memory until `Apply`
- `Apply` updates the file and refreshes the Git window
- binary files show a non-editable detail pane

- [ ] **Step 5: Commit final polish if needed**

```bash
git add -A
git commit -m "chore(git-window): finalize compare editor support"
```
