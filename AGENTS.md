# Repository Guidelines

## Project Structure & Module Organization

This workspace has two crates:

- `teminal-panel/`: the desktop application. Core state and message handling live in `src/app.rs`, config persistence in `src/config.rs`, project management in `src/project/`, and terminal integration in `src/terminal/`.
- `ui/`: reusable Iced wrappers and styling. Components live in `ui/src/components/`, containers in `ui/src/containers/`, and shared theme code in `ui/src/theme.rs`.

Tests are kept close to the code they cover. The app crate uses sibling test files like `teminal-panel/src/app_test.rs` and `teminal-panel/src/config_test.rs`. Design notes and plans live under `docs/superpowers/`.

## Build, Test, and Development Commands

- `cargo build`: build the full workspace.
- `cargo run -p teminal-panel`: launch the GUI app locally.
- `cargo test`: run all workspace tests.
- `cargo test -p teminal-panel`: run only application tests.
- `cargo fmt`: format Rust code.
- `cargo clippy --workspace --all-targets`: lint the workspace.
- `cargo check`: fast compile check without producing binaries.

## Coding Style & Naming Conventions

Follow standard Rust formatting with `cargo fmt`. Use 4-space indentation and keep files ASCII unless the file already requires Unicode. Prefer focused modules over large multipurpose files.

Naming follows Rust conventions:

- `snake_case` for functions, modules, and files
- `PascalCase` for structs and enums
- message variants in `app.rs` should stay explicit, e.g. `SelectTab`, `OpenTerminal`

Match existing Iced patterns: state in `App`, behavior in `update`, rendering in `view`.

## Testing Guidelines

Use Rust unit tests with `#[test]`. Add regression tests for bug fixes before changing behavior when practical. Name tests by behavior, e.g. `backend_events_do_not_override_user_selected_tab`.

Run `cargo test` before committing. For focused work, run a single test with `cargo test <test_name> -- --nocapture`.

## Commit & Pull Request Guidelines

Recent history favors short, imperative subjects, often with prefixes like `feat:`, `refactor(app):`, or plain fixes such as `Fix terminal tab selection behavior`. Keep commits scoped to one change.

Pull requests should include:

- a brief summary of behavior changes
- test evidence (`cargo test`, targeted commands, or screenshots for UI changes)
- linked issues or design docs when relevant

## Configuration & Architecture Notes

Project config is persisted by the app crate; behavior changes that touch projects or terminals should preserve save/load behavior. When editing terminal selection, focus, or panel behavior, add regression coverage in `teminal-panel/src/app_test.rs`.
