# Git Window Phase 2 Design

**Date:** 2026-04-18  
**Status:** Approved

## Goal

Implement true multi-window Git support so each Git-backed project can open its own independent operating system window. Each window should show a left-side file change list and a right-side Git history graph.

## Scope

- Open a real system window for a Git project from the terminal footer Git affordance.
- Allow one Git window per Git project at a time.
- If a Git window for the same project is already open, focus that window instead of opening a duplicate.
- Render a two-pane layout:
  - left: staged and unstaged file changes
  - right: simple commit history graph
- Clean up window state when a Git window closes.

## Non-Goals

- No in-app modal or floating panel fallback.
- No multiple Git windows for the same project.
- No full branch-lane graph layout solver yet.
- No diff tabs, staging actions, or commit actions in this phase.

## UX

### Open behavior

When the user clicks the Git affordance for a Git-backed project:

- If that project has no Git window open, open a new OS window.
- If that project already has a Git window open, focus the existing window.

### Window content

The Git window uses a fixed two-column layout:

- Left pane: file changes list
  - `UNSTAGED CHANGES`
  - `STAGED CHANGES`
- Right pane: commit history graph
  - a scrollable visual history view
  - commit node, connector, short hash, and subject line

### Close behavior

Closing a Git window removes all state and ID mappings associated with that project window. Reopening the same project after close creates a fresh window and reloads data.

## Architecture

### Main app ownership

The main `App` remains the owner of all window lifecycle state.

Add two mappings:

- `git_windows_by_project: HashMap<Uuid, GitWindowState>`
- `git_window_projects_by_id: HashMap<iced::window::Id, Uuid>`

`GitWindowState` stores:

- the `iced::window::Id`
- the `GitWindow` view/state object
- project metadata needed for title and refresh behavior

This keeps lifecycle rules explicit:

- project identity answers "is there already a Git window for this project?"
- window identity answers "which project window just emitted an event or closed?"

### Multi-window strategy

Use Iced window tasks and window events to manage true system windows.

Phase 2 needs:

- window open task for a new Git window
- window focus task for an existing Git window
- window close event handling to clean up mappings

The application must route view/title updates by `window::Id`, not assume a single main window.

### Git window data model

The existing `GitWindow` foundation from Phase 1 should be extended, not replaced.

Add commit graph data in `git_window/git_data.rs`:

- `CommitNode`
  - `oid`
  - `short_id`
  - `summary`
  - `author`
  - `timestamp`
  - `parent_count`

Phase 2 graph is intentionally simple:

- single history ordering from `HEAD`
- one visual lane
- parent connector lines only

This is enough to produce a usable history view without prematurely committing to a full branch graph layout engine.

## Rendering

### Left pane

Reuse the Phase 1 file list presentation, with only layout adjustments needed to fit the final split view.

### Right pane

Render commit history with Iced `canvas`.

Each row shows:

- a node circle
- a vertical connector
- short hash
- commit subject

Optional secondary text:

- author
- relative or formatted time

### Layout proportions

- Left pane: fixed width around 280-320 px
- Right pane: fill remaining width

This keeps file status scanning predictable while giving the graph room to breathe.

## Messages

Add Git-window-specific messages for:

- opening a project Git window
- focusing an existing Git window
- window-open completion with returned `window::Id`
- window-close cleanup
- selecting a file in the left pane if needed
- graph scroll if needed for canvas state

The main app message model should be explicit about whether a message targets:

- the main window
- a specific Git window

## Testing

Add focused tests for:

- opening a Git window only for Git-backed projects
- refusing duplicate windows for the same project
- tracking the returned `window::Id` against `project_id`
- cleaning up Git window state on close
- commit history loading for non-repo/error cases and simple repo cases where practical

Keep logic testable outside the Iced widget tree whenever possible:

- mapping helpers
- commit loading helpers
- window-state cleanup helpers

## Risks

- Iced multi-window state wiring is the main integration risk because the current app is still shaped like a single-window application.
- Graph rendering complexity can balloon if branch-lane layout is attempted too early.

Phase 2 avoids that by shipping a simple, reliable commit history graph first.

## Exit Criteria

Phase 2 is complete when:

- clicking Git opens a real OS window
- each Git project can own one independent Git window
- repeat clicks focus the existing project window
- left pane shows file changes
- right pane shows commit history graph
- closing the window fully removes tracked state
