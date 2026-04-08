# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Build & Development Commands

```bash
# Build the project
cargo build

# Run the application
cargo run

# Run tests
cargo test

# Run a specific test
cargo test test_name

# Run tests with output
cargo test -- --nocapture

# Format code
cargo fmt

# Lint with clippy
cargo clippy

# Check without building
cargo check
```

## Architecture Overview

**teminal-panel** is a GUI application for managing multiple project terminals. It uses an Elm-like message-based architecture with Iced as the GUI framework.

### Core Architecture Pattern

The application follows a unidirectional data flow:
- **State**: Stored in `App` struct (src/app.rs)
- **Messages**: Enum of all possible state changes (Message enum in app.rs)
- **Update**: Pure function that processes messages and returns new state
- **View**: Pure function that renders UI from state
- **Subscriptions**: Async event streams (PTY output)

### Module Structure

- **ui/** - Reusable UI component library
  - **components/** - Button, TextInput wrappers
  - **containers/** - Modal, Container components
  - **theme.rs** - Color scheme and styling
- **teminal-panel/** - Main application
  - **app.rs** - Main application state and UI layout
  - **config.rs** - Configuration persistence (TOML format)
  - **project/** - Project struct definition
  - **terminal/** - Terminal emulation and PTY management

### Key Data Structures

- `App`: Main state container with config, selected project, terminals map, and PTY channel sender
- `Message`: Enum of all state changes (SelectProject, AddProject, PtyOutput, TerminalInput, etc.)
- `Project`: Represents a project with UUID, name, working directory, and connection type
- `TerminalState`: Active terminal session with PTY writer, model, and resize callback
- `TerminalModel`: Screen buffer with ANSI parsing

### Message Flow

1. User interaction → Message created
2. `App::update()` processes message, mutates state
3. `App::view()` renders UI from new state
4. PTY output arrives → `PtyOutput` message → update → view
5. Terminal resize → `TerminalViewportChanged` → PTY resize via callback

### Configuration

Projects are stored in TOML format. The config system supports backward compatibility: old configs used an "agents" field, new ones use "projects". On load, if "projects" is empty, it falls back to "agents".

## Testing

Tests are in `src/app.rs` and `src/config.rs`. Key patterns:

- Use `with_temp_config_dir()` to isolate config file tests
- Use `test_app()` to create a test App instance
- Use `insert_test_terminal()` to mock terminal state
- Tests verify message handling, config persistence, and terminal lifecycle

Run tests with: `cargo test`

## Dependencies

- **iced 0.13**: GUI framework with Elm-like architecture
- **portable-pty 0.8**: Cross-platform PTY spawning
- **tokio**: Async runtime (full features enabled)
- **termwiz 0.23.3**: Terminal utilities for ANSI parsing
- **serde/toml**: Config serialization
- **uuid**: Project IDs
- **rfd**: File dialogs (for folder picker feature)
- **dirs**: Platform-specific config directory paths

## Recent Work & Design

- Refactored to expose projects on config (commit 392704f)
- Design spec for project folder picker: `docs/superpowers/plans/2026-04-08-add-project-native-folder-picker.md`
- Launch configuration added for debugging

## Common Patterns

- **State mutations**: Always call `self.config.save()` after modifying projects
- **Terminal lifecycle**: Call `Self::shutdown_terminal()` when removing projects to clean up PTY
- **PTY communication**: Use `terminal.writer.write_all()` to send input; PTY output arrives via subscription
- **Viewport sizing**: Terminal grid size calculated from viewport dimensions using `CELL_WIDTH` and `CELL_HEIGHT` constants
