# Terminal Panel Plan 2: termwiz ANSI Rendering Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the raw PTY text viewer with a `termwiz`-backed ANSI terminal model and renderer, and synchronize PTY rows/cols with terminal viewport size changes.

**Architecture:** Keep `portable-pty` for process transport, but route PTY bytes through a new `termwiz` parser + `Surface` model. Render the resulting screen grid with a dedicated terminal view in `iced`, and deduplicate viewport-driven resize updates before calling `MasterPty::resize`.

**Tech Stack:** Rust, iced 0.13, portable-pty 0.8, termwiz 0.23.3, tokio 1, uuid 1

---

## File Structure

- Modify: `Cargo.toml`
  - Add `termwiz = "0.23.3"`
- Modify: `src/terminal/mod.rs`
  - Replace raw text-oriented `TerminalState` fields with model-oriented state and resize metadata
- Create: `src/terminal/model.rs`
  - Own `termwiz::escape::parser::Parser`, `termwiz::surface::Surface`, and ANSI action application helpers
- Create: `src/terminal/render.rs`
  - Expose an `iced` terminal view that paints visible cells, colors, and cursor
- Modify: `src/terminal/pty.rs`
  - Keep the PTY master handle and expose a `resize` method
- Modify: `src/app.rs`
  - Route `PtyOutput` into `TerminalModel`
  - Track viewport size changes and issue PTY resize requests
  - Replace `scrollable(text(...))` with the terminal renderer

### Task 1: Add `termwiz` and Refactor Terminal State Shell

**Files:**
- Modify: `Cargo.toml`
- Modify: `src/terminal/mod.rs`
- Modify: `src/app.rs`

- [ ] **Step 1: Add the dependency**

Update `Cargo.toml`:

```toml
[dependencies]
iced = { version = "0.13", features = ["tokio", "advanced"] }
portable-pty = "0.8"
serde = { version = "1", features = ["derive"] }
toml = "0.8"
uuid = { version = "1", features = ["v4", "serde"] }
tokio = { version = "1", features = ["full"] }
dirs = "5"
once_cell = "1"
termwiz = "0.23.3"
```

- [ ] **Step 2: Replace the terminal state shape in `src/terminal/mod.rs`**

Change the module to expose the new submodules and state:

```rust
pub mod model;
pub mod pty;
pub mod render;
pub mod subscription;

use uuid::Uuid;

pub struct TerminalState {
    pub id: Uuid,
    pub agent_id: Uuid,
    pub model: model::TerminalModel,
    pub input_buf: String,
    pub writer: Box<dyn std::io::Write + Send>,
    pub lifecycle: Option<pty::PtyLifecycle>,
    pub last_size: Option<model::TerminalSize>,
}
```

- [ ] **Step 3: Update compile errors in `src/app.rs` without adding behavior yet**

Temporarily adjust terminal creation to instantiate the model:

```rust
TerminalState {
    id: agent_id,
    agent_id,
    model: crate::terminal::model::TerminalModel::new(80, 24),
    input_buf: String::new(),
    writer: handle.writer,
    lifecycle: Some(handle.lifecycle),
    last_size: None,
}
```

Remove the raw-text helper methods that will no longer be used:

- `trim_output_to_last_chars`
- `append_output_bytes`

- [ ] **Step 4: Run compile-focused verification**

Run: `cargo check`

Expected: build fails only because `src/terminal/model.rs` and `src/terminal/render.rs` do not exist yet, or because the new fields are not fully wired.

- [ ] **Step 5: Commit the state shell**

```bash
git add Cargo.toml src/terminal/mod.rs src/app.rs
git commit -m "refactor: prepare terminal state for termwiz model"
```

### Task 2: Implement the `termwiz` Terminal Model

**Files:**
- Create: `src/terminal/model.rs`
- Modify: `src/terminal/mod.rs`
- Test: `src/terminal/model.rs`

- [ ] **Step 1: Write failing unit tests for the model**

Create tests in `src/terminal/model.rs` for the core parser/surface behavior:

```rust
#[test]
fn advance_plain_text_updates_surface() {
    let mut model = TerminalModel::new(5, 3);
    model.advance_bytes(b"abc");
    assert!(model.screen_text().starts_with("abc"));
}

#[test]
fn advance_sgr_color_sets_cell_attributes() {
    let mut model = TerminalModel::new(5, 3);
    model.advance_bytes(b"\x1b[31mR");
    let cell = model.visible_cells()[0][0].clone();
    assert_eq!(cell.str(), "R");
    assert_ne!(cell.attrs().foreground(), termwiz::color::ColorAttribute::Default);
}

#[test]
fn clear_display_erases_prior_text() {
    let mut model = TerminalModel::new(5, 3);
    model.advance_bytes(b"abc\x1b[2J");
    assert!(!model.screen_text().contains("abc"));
}

#[test]
fn resize_updates_surface_dimensions() {
    let mut model = TerminalModel::new(80, 24);
    model.resize(TerminalSize { cols: 100, rows: 40 });
    assert_eq!(model.size(), TerminalSize { cols: 100, rows: 40 });
}
```

- [ ] **Step 2: Run the new tests and confirm they fail**

Run: `cargo test terminal::model::tests -- --nocapture`

Expected: FAIL because `TerminalModel` does not exist yet.

- [ ] **Step 3: Implement the model skeleton**

Create the model and size types:

```rust
use termwiz::escape::parser::Parser;
use termwiz::surface::Surface;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TerminalSize {
    pub cols: u16,
    pub rows: u16,
}

pub struct TerminalModel {
    parser: Parser,
    surface: Surface,
    size: TerminalSize,
    dirty: bool,
}
```

Implement core methods:

```rust
impl TerminalModel {
    pub fn new(cols: u16, rows: u16) -> Self { /* create Parser and Surface */ }

    pub fn advance_bytes(&mut self, bytes: &[u8]) { /* parser.parse(...); apply actions */ }

    pub fn resize(&mut self, size: TerminalSize) { /* resize Surface when changed */ }

    pub fn size(&self) -> TerminalSize { self.size }

    pub fn surface(&self) -> &Surface { &self.surface }

    pub fn is_dirty(&self) -> bool { self.dirty }

    pub fn mark_clean(&mut self) { self.dirty = false; }
}
```

- [ ] **Step 4: Implement ANSI action application**

Inside `src/terminal/model.rs`, add a narrow adapter:

```rust
fn apply_action(surface: &mut Surface, action: termwiz::escape::Action) {
    use termwiz::escape::{Action, ControlCode};

    match action {
        Action::Print(c) => surface.add_change(termwiz::surface::Change::Text(c.to_string())),
        Action::PrintString(text) => surface.add_change(termwiz::surface::Change::Text(text)),
        Action::Control(ControlCode::CarriageReturn) => {
            let (_, row) = surface.cursor_position();
            surface.add_change(termwiz::surface::Change::CursorPosition {
                x: termwiz::surface::Position::Absolute(0),
                y: termwiz::surface::Position::Absolute(row),
            });
        }
        Action::Control(ControlCode::LineFeed) => {
            surface.add_change(termwiz::surface::Change::Text("\n".into()));
        }
        Action::CSI(csi) => apply_csi(surface, csi),
        _ => {}
    }
}
```

Implement `apply_csi` in the same file for the subset required in this phase:

- `CSI::Sgr(...)` -> foreground/background/intensity/underline/reverse changes
- `CSI::Cursor(...)` -> cursor movement and absolute position
- `CSI::Edit(...)` -> insert/delete/erase operations where directly representable
- `CSI::Mode(...)` only when it affects visible state and has a clear `Surface` mapping
- clear-screen and clear-line variants through `Change::ClearScreen`, `Change::ClearToEndOfLine`, `Change::ClearToEndOfScreen`

Keep unsupported cases as no-ops with a comment, not panics.

- [ ] **Step 5: Add test-only helpers for assertions**

Implement helpers only for tests:

```rust
#[cfg(test)]
impl TerminalModel {
    fn screen_text(&self) -> String {
        self.surface.screen_chars_to_string()
    }

    fn visible_cells(&mut self) -> Vec<&mut [termwiz::cell::Cell]> {
        self.surface.screen_cells()
    }
}
```

- [ ] **Step 6: Run model verification**

Run: `cargo test terminal::model::tests -- --nocapture`

Expected: PASS for the new model tests.

- [ ] **Step 7: Commit the model**

```bash
git add src/terminal/mod.rs src/terminal/model.rs
git commit -m "feat: add termwiz terminal model"
```

### Task 3: Add PTY Resize Support

**Files:**
- Modify: `src/terminal/pty.rs`
- Modify: `src/terminal/mod.rs`
- Modify: `src/app.rs`
- Test: `src/app.rs`

- [ ] **Step 1: Write a failing resize coordination test**

Add a test in `src/app.rs` that records resize requests:

```rust
#[test]
fn terminal_resize_requests_pty_resize_once_per_new_grid_size() {
    let mut app = test_app();
    let agent_id = Uuid::new_v4();
    let tracker = insert_test_terminal_with_resize(&mut app, agent_id);

    let _ = app.update(Message::TerminalViewportChanged(
        agent_id,
        TerminalViewport { width: 800.0, height: 384.0 },
    ));
    let _ = app.update(Message::TerminalViewportChanged(
        agent_id,
        TerminalViewport { width: 800.0, height: 384.0 },
    ));

    assert_eq!(tracker.lock().unwrap().as_slice(), &[TerminalSize { cols: 100, rows: 24 }]);
}
```

- [ ] **Step 2: Run the test and confirm it fails**

Run: `cargo test terminal_resize_requests_pty_resize_once_per_new_grid_size -- --nocapture`

Expected: FAIL because there is no resize path yet.

- [ ] **Step 3: Keep the PTY master handle and add `resize`**

Update `src/terminal/pty.rs`:

```rust
use portable_pty::{native_pty_system, Child, CommandBuilder, MasterPty, PtySize};
use std::sync::Arc;

pub struct PtyHandle {
    pub writer: Box<dyn std::io::Write + Send>,
    pub lifecycle: PtyLifecycle,
    pub controller: Arc<dyn MasterPty + Send>,
}

impl PtyHandle {
    pub fn resize(&self, size: crate::terminal::model::TerminalSize) -> PtyResult<()> {
        self.controller.resize(PtySize {
            rows: size.rows,
            cols: size.cols,
            pixel_width: 0,
            pixel_height: 0,
        })?;
        Ok(())
    }
}
```

When spawning:

```rust
let mut pair = pty_system.openpty(PtySize { rows: 24, cols: 80, pixel_width: 0, pixel_height: 0 })?;
let controller = pair.master;
let writer = controller.take_writer()?;
let mut reader = controller.try_clone_reader()?;
```

Store `controller` inside `Arc`.

- [ ] **Step 4: Thread resize capability through terminal state**

Add a resize callback or controller handle to `TerminalState`:

```rust
pub resize: Box<dyn Fn(TerminalSize) -> crate::terminal::pty::PtyResult<()> + Send + Sync>,
```

or, if cleaner:

```rust
pub controller: std::sync::Arc<dyn portable_pty::MasterPty + Send>,
```

Prefer whichever keeps tests easiest to fake without over-coupling.

- [ ] **Step 5: Add the viewport message and dedup logic in `src/app.rs`**

Add:

```rust
TerminalViewportChanged(Uuid, TerminalViewport),
```

and supporting types:

```rust
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TerminalViewport {
    pub width: f32,
    pub height: f32,
}
```

When handling the message:

1. Compute `TerminalSize` from fixed metrics
2. Ignore zero or unchanged sizes
3. Call `terminal.model.resize(size)`
4. Call the PTY resize entry point
5. Store `terminal.last_size = Some(size)`

- [ ] **Step 6: Run resize tests**

Run: `cargo test terminal_resize_requests_pty_resize_once_per_new_grid_size -- --nocapture`

Expected: PASS

- [ ] **Step 7: Commit the resize support**

```bash
git add src/terminal/pty.rs src/terminal/mod.rs src/app.rs
git commit -m "feat: add PTY resize support for terminal viewport"
```

### Task 4: Build the `iced` Terminal Renderer

**Files:**
- Create: `src/terminal/render.rs`
- Modify: `src/terminal/mod.rs`
- Modify: `src/app.rs`

- [ ] **Step 1: Write a focused renderer unit test**

If the chosen renderer exposes a pure helper, add a test such as:

```rust
#[test]
fn color_attribute_mapping_preserves_basic_ansi_colors() {
    assert_eq!(map_color(ColorAttribute::PaletteIndex(1)), iced::Color::from_rgb8(205, 49, 49));
}
```

If no pure helper is practical, skip this unit test and rely on app tests plus manual verification.

- [ ] **Step 2: Create a render module with fixed metrics**

Add constants:

```rust
pub const CELL_WIDTH: f32 = 8.0;
pub const CELL_HEIGHT: f32 = 16.0;
pub const FONT_SIZE: f32 = 14.0;
```

Expose:

```rust
pub fn terminal_view<'a>(
    terminal_id: uuid::Uuid,
    model: &'a crate::terminal::model::TerminalModel,
    on_resize: impl Fn(crate::app::TerminalViewport) -> crate::app::Message + 'a,
) -> iced::Element<'a, crate::app::Message>
```

- [ ] **Step 3: Implement the renderer with a fixed-grid custom widget tree**

Prefer a fixed-grid composition of rows, containers, and text widgets. Avoid `canvas` if it causes stale glyphs or awkward resize reporting.

The draw logic should:

```rust
for (row_index, line) in model.surface().screen_lines().iter().enumerate() {
    for (col_index, cell) in line.visible_cells().enumerate() {
        // fill background rect
        // draw glyph text
    }
}

// draw cursor after cells
```

Map supported attributes:

- foreground color
- background color
- bold
- underline
- reverse video

Do not attempt selection, IME, or width-perfect grapheme measurement in this phase.

- [ ] **Step 4: Emit viewport size changes from the view**

Ensure the view reports its laid-out pixel bounds back to the app:

```rust
on_resize(TerminalViewport {
    width: bounds.width,
    height: bounds.height,
})
```

Wrap the renderer in a tiny custom widget that can publish `TerminalViewportChanged`.

- [ ] **Step 5: Replace the raw output widget in `src/app.rs`**

Replace:

```rust
scrollable(text(&terminal.output).font(iced::Font::MONOSPACE))
```

with the renderer:

```rust
crate::terminal::render::terminal_view(selected_id, &terminal.model, move |viewport| {
    Message::TerminalViewportChanged(selected_id, viewport)
})
```

- [ ] **Step 6: Run compile verification**

Run: `cargo check`

Expected: PASS

- [ ] **Step 7: Commit the renderer**

```bash
git add src/terminal/render.rs src/terminal/mod.rs src/app.rs
git commit -m "feat: render termwiz surface in iced terminal view"
```

### Task 5: Wire PTY Output Into the Model and Preserve Existing Input Flow

**Files:**
- Modify: `src/app.rs`
- Modify: `src/terminal/model.rs`
- Test: `src/app.rs`

- [ ] **Step 1: Write failing app-level tests for PTY output**

Add tests replacing the raw-text assumptions:

```rust
#[test]
fn pty_output_advances_terminal_model_screen() {
    let mut app = test_app();
    let agent_id = Uuid::new_v4();
    let _ = insert_test_terminal(&mut app, agent_id, None);

    let _ = app.update(Message::PtyOutput(agent_id, b"hi".to_vec()));

    let terminal = app.terminals.get_mut(&agent_id).unwrap();
    assert!(terminal.model.screen_text().starts_with("hi"));
}

#[test]
fn ansi_output_updates_surface_instead_of_literal_escape_text() {
    let mut app = test_app();
    let agent_id = Uuid::new_v4();
    let _ = insert_test_terminal(&mut app, agent_id, None);

    let _ = app.update(Message::PtyOutput(agent_id, b\"\\x1b[31mR\".to_vec()));

    let terminal = app.terminals.get_mut(&agent_id).unwrap();
    assert!(!terminal.model.screen_text().contains(\"[31m\"));
}
```

- [ ] **Step 2: Run the app tests and confirm they fail**

Run: `cargo test pty_output_advances_terminal_model_screen ansi_output_updates_surface_instead_of_literal_escape_text -- --nocapture`

Expected: FAIL until `Message::PtyOutput` uses the model.

- [ ] **Step 3: Change `Message::PtyOutput` handling**

Update `src/app.rs`:

```rust
Message::PtyOutput(id, bytes) => {
    if let Some(terminal) = self.terminals.get_mut(&id) {
        terminal.model.advance_bytes(&bytes);
    }
}
```

- [ ] **Step 4: Preserve submitted text input**

Keep the existing input semantics for this phase:

```rust
let _ = terminal.writer.write_all(input.as_bytes());
let _ = terminal.writer.write_all(b"\n");
terminal.input_buf.clear();
```

Do not expand this into full key protocol handling in Plan 2.

- [ ] **Step 5: Update or replace obsolete tests**

Remove Plan 1 tests that assert on:

- `terminal.output`
- UTF-8 pending-bytes buffer behavior
- output trimming

Replace them with model/surface assertions.

- [ ] **Step 6: Run the full automated suite**

Run: `cargo test`

Expected: PASS

- [ ] **Step 7: Commit the integration**

```bash
git add src/app.rs src/terminal/model.rs
git commit -m "feat: route PTY output through termwiz terminal model"
```

### Task 6: Manual Verification and Cleanup

**Files:**
- Modify: `docs/superpowers/specs/2026-04-08-plan-2-termwiz-rendering-design.md`
- Modify: `docs/superpowers/plans/2026-04-08-plan-2-termwiz-rendering.md`

- [ ] **Step 1: Run formatting and checks**

Run:

```bash
cargo fmt
cargo check
cargo test
```

Expected: all commands succeed.

- [ ] **Step 2: Run Xvfb validation**

Run the app under Xvfb using the same environment pattern from Plan 1:

```bash
DISPLAY=:99 \
XDG_RUNTIME_DIR=/tmp/teminal-panel-plan2/runtime \
XDG_CONFIG_HOME=/tmp/teminal-panel-plan2/config \
WINIT_UNIX_BACKEND=x11 \
LIBGL_ALWAYS_SOFTWARE=1 \
WGPU_BACKEND=gl \
cargo run
```

Validate manually:

- `printf '\033[31mred\033[0m\n'` shows red text
- `printf 'a\rX\n'` leaves `X` at the start of the line
- `clear` clears prior visible content
- resizing the terminal area causes `stty size` inside the shell to change

- [ ] **Step 3: Update plan/spec notes if implementation diverged**

If the final implementation uses `termwiz::Surface` plus a local adapter instead of any previously named crate or API, make sure both docs reflect that final shape before closing the branch.

- [ ] **Step 4: Commit final verification/doc alignment**

```bash
git add docs/superpowers/specs/2026-04-08-plan-2-termwiz-rendering-design.md docs/superpowers/plans/2026-04-08-plan-2-termwiz-rendering.md
git commit -m "docs: finalize plan 2 terminal rendering notes"
```
