# Git Compare Editor Design

**Date:** 2026-04-19  
**Status:** Proposed

## Goal

Upgrade the Git window file detail area from a patch preview into a VS Code-like compare editor for file-level changes.

The compare editor should let the user:

- click a changed file in the Git window
- compare the Git base version against the current working tree version
- edit the working tree version in place
- explicitly apply or discard those edits

## Scope

- Replace the current right-side patch/detail pane with a compare editor experience.
- Keep the existing left-side changed-file list.
- Support text files with:
  - left read-only base pane
  - right editable working tree pane
  - `Apply` and `Discard` actions
- Support binary files with:
  - metadata/info presentation
  - explicit "editing not supported" messaging
- Refresh file detail and diff state after apply/discard.

## Non-Goals

- No live write-through on every keystroke.
- No three-way merge editor.
- No staging, unstaging, or commit actions in this phase.
- No external file-watch reconciliation for edits performed outside the app.
- No complex VS Code-style intra-line decorations or full diff gutter algorithm in the first version.

## UX

### File selection

The left-side file list remains the entry point.

When the user clicks a file:

- if the file is text, the right side opens a compare editor
- if the file is binary, the right side shows a binary-file information view

The currently selected file should be visually highlighted in the file list.

### Text compare editor

The text compare editor uses two columns:

- Left: `Base`
  - read-only
  - content loaded from the Git base version for that file
- Right: `Working Tree`
  - editable
  - initialized from the current file content on disk

The detail header shows:

- file path
- file kind/status indicator
- dirty state when the right pane has unapplied edits

The footer or action row shows:

- `Apply`
- `Discard`

### Apply / Discard semantics

`Apply`

- writes the current editable content to the working tree file on disk
- reloads the working tree content and file diff state
- refreshes the file list and detail pane
- clears dirty state

`Discard`

- discards in-memory edits only
- restores the editable pane from the current working tree file on disk
- clears dirty state

This behavior is intentionally explicit. Editing alone must not mutate files until `Apply`.

### Binary files

Binary files should not attempt to open in the text compare editor.

Instead, the detail area should display:

- file path
- binary file status
- a short message that editing is not supported

`Apply` and `Discard` should be hidden or disabled in this state.

## Architecture

### Git window file detail state

The Git window should replace the current `selected_diff`-only detail model with explicit file-detail state.

Introduce a dedicated file detail structure storing:

- selected file path
- file kind: text or binary
- base content for text files
- working tree content loaded from disk
- editor draft state for the editable pane
- dirty state
- diff/summary text if still useful for refresh or fallbacks
- detail-level error state

This keeps compare-editor logic isolated from the rest of the Git window.

### Data loading helpers

Extend `git_window/git_data.rs` with helpers for:

- reading the base revision contents for a file
- reading the current working tree file contents
- detecting binary vs text content
- writing edited working tree content back to disk
- recomputing patch text/diff after apply

These helpers should be independently testable and avoid coupling UI code to `git2` details.

### Message flow

The Git window message model should add explicit compare-editor actions:

- select file
- edit working-tree draft
- apply draft
- discard draft

The update flow should be:

1. file selected
2. detail content loaded
3. draft edited in memory
4. apply or discard executed explicitly
5. state refreshed

## Rendering

### Layout

Keep the Git window as a two-pane top-level layout:

- Left pane: changed files list
- Right pane: file detail area

Inside the right pane for text files:

- header row
- two-column compare editor
- action row / status area

Suggested proportions:

- left file list: 280-320 px fixed width
- right detail area: fill remaining width
- inside right detail: near-even split between base and working tree panes

### Editor widgets

Use Iced editor primitives for the working-tree pane if they support the needed editing model cleanly.

The base pane should remain read-only and should not share mutable state with the working-tree draft.

If the available widget requires separate text state objects, keep them fully distinct.

## Error handling

- If base content fails to load, show an error in the detail area and keep the rest of the Git window usable.
- If working tree content fails to load, show an error and disable editing actions.
- If apply fails, keep the user draft intact and show the failure without losing text.
- If discard fails to reload the file, surface the error and keep the current draft until the user changes selection or retries.

## Testing

Add focused tests for:

- loading base and working tree text content for a selected file
- detecting binary files and entering non-editable detail mode
- selecting a file initializes compare-editor state
- editing marks the file detail dirty
- apply writes updated content to disk and refreshes state
- discard restores the editor from current disk content
- apply failures preserve in-memory draft content

Prefer testing helper/data logic directly where possible. Keep UI-specific tests minimal and focused on state transitions.

## Risks

- Iced text editing primitives may impose state-management constraints that affect how compare panes are structured.
- Large files may make the compare editor sluggish if the first version tries to render too much text at once.
- Binary detection needs to be conservative enough to avoid sending binary data into the text editor path.

## Exit Criteria

This phase is complete when:

- clicking a changed text file opens a two-pane compare editor
- the left pane is read-only base content
- the right pane is editable working tree content
- edits remain in memory until `Apply`
- `Apply` writes back to disk and refreshes Git-window detail state
- `Discard` resets the editable pane without writing
- binary files show a non-editable info state instead of a broken text view
