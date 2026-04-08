# wezterm 库集成实现计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace termwiz with wezterm-term and wezterm-escape-parser while maintaining 100% feature parity and all existing tests.

**Architecture:** Update Cargo.toml to replace termwiz dependencies, rewrite terminal/model.rs to use wezterm Terminal and Parser, update terminal/render.rs to work with wezterm cell types. All changes are internal to the terminal module; public API remains unchanged.

**Tech Stack:** Rust, wezterm-term 0.1, wezterm-escape-parser 0.1, iced 0.13

---

## File Structure

**Files to modify:**
- `teminal-panel/Cargo.toml` - Replace termwiz with wezterm dependencies
- `teminal-panel/src/terminal/model.rs` - Rewrite to use wezterm Terminal and Parser
- `teminal-panel/src/terminal/render.rs` - Update to work with wezterm cell types

**Files unchanged:**
- `teminal-panel/src/terminal/subscription.rs` - No changes needed
- `teminal-panel/src/terminal/pty.rs` - No changes needed
- All test files - No changes needed

---

## Task 1: Update Dependencies

**Files:**
- Modify: `teminal-panel/Cargo.toml`

- [ ] **Step 1: Read current Cargo.toml**

```bash
grep -A 20 "\[dependencies\]" teminal-panel/Cargo.toml | grep termwiz
```

Expected: Shows `termwiz = "0.23.3"`

- [ ] **Step 2: Remove termwiz dependency**

In `teminal-panel/Cargo.toml`, find the line:
```toml
termwiz = "0.23.3"
```

Replace with:
```toml
wezterm-term = "0.1"
wezterm-escape-parser = "0.1"
```

- [ ] **Step 3: Verify Cargo.toml syntax**

```bash
cargo check -p teminal-panel 2>&1 | head -20
```

Expected: Should show dependency resolution, not syntax errors

- [ ] **Step 4: Commit**

```bash
git add teminal-panel/Cargo.toml
git commit -m "chore: replace termwiz with wezterm dependencies"
```

---

## Task 2: Rewrite terminal/model.rs - Part 1 (Imports and Struct)

**Files:**
- Modify: `teminal-panel/src/terminal/model.rs`

- [ ] **Step 1: Read current imports section**

Read the top of `teminal-panel/src/terminal/model.rs` to see all termwiz imports.

- [ ] **Step 2: Replace imports**

Replace all termwiz imports with wezterm imports. The new imports section should be:

```rust
use wezterm_term::Terminal;
use wezterm_escape_parser::Parser;
use std::sync::Arc;
use std::sync::Mutex;
```

- [ ] **Step 3: Update TerminalModel struct**

Find the `pub struct TerminalModel` definition and replace it with:

```rust
pub struct TerminalModel {
    terminal: Terminal,
    parser: Parser,
    dirty: bool,
}
```

- [ ] **Step 4: Verify compilation**

```bash
cargo check -p teminal-panel 2>&1 | grep "error\|warning" | head -10
```

Expected: Will have errors in methods, that's OK for now

- [ ] **Step 5: Commit**

```bash
git add teminal-panel/src/terminal/model.rs
git commit -m "feat: update TerminalModel to use wezterm Terminal and Parser"
```

---

## Task 3: Rewrite terminal/model.rs - Part 2 (Methods)

**Files:**
- Modify: `teminal-panel/src/terminal/model.rs`

- [ ] **Step 1: Implement TerminalModel::new()**

Replace the `new()` method with:

```rust
impl TerminalModel {
    pub fn new(cols: u16, rows: u16) -> Self {
        let terminal = Terminal::new(
            wezterm_term::TerminalSize {
                cols: cols as usize,
                rows: rows as usize,
                pixel_width: 0,
                pixel_height: 0,
            },
            Arc::new(Mutex::new(wezterm_term::config::Configuration::default())),
        );

        Self {
            terminal,
            parser: Parser::new(),
            dirty: true,
        }
    }
```

- [ ] **Step 2: Implement process_output()**

Replace the `process_output()` method with:

```rust
    pub fn process_output(&mut self, data: &[u8]) {
        for byte in data {
            self.parser.parse(*byte, |action| {
                self.terminal.perform_action(action);
            });
        }
        self.dirty = true;
    }
```

- [ ] **Step 3: Implement surface() method**

Replace the `surface()` method with:

```rust
    pub fn surface(&self) -> &Terminal {
        &self.terminal
    }
```

- [ ] **Step 4: Implement resize()**

Replace the `resize()` method with:

```rust
    pub fn resize(&mut self, size: TerminalSize) -> Result<(), String> {
        self.terminal.resize(wezterm_term::TerminalSize {
            cols: size.cols as usize,
            rows: size.rows as usize,
            pixel_width: 0,
            pixel_height: 0,
        });
        Ok(())
    }
```

- [ ] **Step 5: Implement is_dirty() and mark_clean()**

Keep these methods unchanged:

```rust
    pub fn is_dirty(&self) -> bool {
        self.dirty
    }

    pub fn mark_clean(&mut self) {
        self.dirty = false;
    }
```

- [ ] **Step 6: Verify compilation**

```bash
cargo check -p teminal-panel 2>&1 | grep "error" | head -5
```

Expected: May have errors in render.rs, that's expected

- [ ] **Step 7: Commit**

```bash
git add teminal-panel/src/terminal/model.rs
git commit -m "feat: implement TerminalModel methods using wezterm"
```

---

## Task 4: Update terminal/render.rs - Part 1 (Imports and Helper Functions)

**Files:**
- Modify: `teminal-panel/src/terminal/render.rs`

- [ ] **Step 1: Replace termwiz imports**

Find and replace all termwiz imports with wezterm imports:

```rust
use wezterm_term::cell::{CellAttributes, Intensity, Underline};
use wezterm_term::color::ColorAttribute;
use wezterm_term::Surface;
```

- [ ] **Step 2: Update render_cell() function signature**

Find the `render_cell()` function and update it to work with wezterm cells:

```rust
fn render_cell<'a>(
    content: String,
    attrs: &wezterm_term::cell::CellAttributes,
    width: usize,
) -> Element<'a, Message> {
    // Implementation follows
}
```

- [ ] **Step 3: Update color conversion functions**

Replace color conversion functions to work with wezterm colors:

```rust
fn color_to_iced(color: wezterm_term::color::ColorAttribute) -> Color {
    match color {
        wezterm_term::color::ColorAttribute::Default => DEFAULT_FOREGROUND,
        wezterm_term::color::ColorAttribute::PaletteIndex(idx) => {
            // Convert palette index to iced Color
            palette_color(idx as usize)
        }
        wezterm_term::color::ColorAttribute::TrueColor(rgb) => {
            Color::from_rgb8(rgb.red, rgb.green, rgb.blue)
        }
    }
}
```

- [ ] **Step 4: Verify compilation**

```bash
cargo check -p teminal-panel 2>&1 | grep "error" | head -10
```

Expected: May have errors in render_cell implementation

- [ ] **Step 5: Commit**

```bash
git add teminal-panel/src/terminal/render.rs
git commit -m "feat: update render.rs imports and color handling for wezterm"
```

---

## Task 5: Update terminal/render.rs - Part 2 (Rendering Logic)

**Files:**
- Modify: `teminal-panel/src/terminal/render.rs`

- [ ] **Step 1: Update terminal_view() function**

Update the function to work with wezterm Terminal:

```rust
pub fn terminal_view<'a>(
    _terminal_id: Uuid,
    model: &'a TerminalModel,
    on_resize: impl Fn(TerminalViewport) -> Message + 'a,
    on_key: impl Fn(String) -> Message + 'a,
) -> Element<'a, Message> {
    let terminal = model.surface();
    let mut rows = column![].spacing(0);

    // Iterate through terminal lines
    for row_index in 0..terminal.screen_lines().len() {
        let line = &terminal.screen_lines()[row_index];
        let mut cells = row![].spacing(0);

        for cell in line.iter() {
            let content = cell.str().to_string();
            let attrs = cell.attributes();

            cells = cells.push(render_cell(content, attrs, 1));
        }

        rows = rows.push(container(cells).height(Length::Fixed(CELL_HEIGHT)));
    }

    ViewportReporter::new(
        container(rows).width(Length::Fill).height(Length::Fill),
        on_resize,
        on_key,
    )
    .into()
}
```

- [ ] **Step 2: Update render_cell() implementation**

Implement the full render_cell function:

```rust
fn render_cell<'a>(
    content: String,
    attrs: &wezterm_term::cell::CellAttributes,
    _width: usize,
) -> Element<'a, Message> {
    let fg_color = color_to_iced(attrs.foreground());
    let bg_color = color_to_iced(attrs.background());

    let mut text_widget = text(content)
        .width(Length::Fixed(CELL_WIDTH))
        .height(Length::Fixed(CELL_HEIGHT));

    if attrs.intensity() == Intensity::Bold {
        text_widget = text_widget.size(14);
    }

    container(text_widget)
        .width(Length::Fixed(CELL_WIDTH))
        .height(Length::Fixed(CELL_HEIGHT))
        .style(move |_| {
            container::Style::default()
                .background(bg_color)
        })
        .into()
}
```

- [ ] **Step 3: Verify compilation**

```bash
cargo build -p teminal-panel 2>&1 | grep "error" | head -10
```

Expected: Should compile successfully or have only minor errors

- [ ] **Step 4: Commit**

```bash
git add teminal-panel/src/terminal/render.rs
git commit -m "feat: update terminal rendering to use wezterm Terminal"
```

---

## Task 6: Fix Compilation Errors

**Files:**
- Modify: `teminal-panel/src/terminal/model.rs` and `render.rs` as needed

- [ ] **Step 1: Build and identify errors**

```bash
cargo build -p teminal-panel 2>&1
```

Expected: May have type errors or missing methods

- [ ] **Step 2: Fix type mismatches**

For each compilation error:
- Read the error message carefully
- Identify the type mismatch or missing method
- Update the code to match wezterm API

Common fixes:
- `terminal.screen_lines()` returns different type than termwiz
- Color handling may need adjustment
- Cell iteration may need different approach

- [ ] **Step 3: Verify compilation**

```bash
cargo build -p teminal-panel 2>&1 | tail -5
```

Expected: "Finished `dev` profile"

- [ ] **Step 4: Commit**

```bash
git add teminal-panel/src/terminal/
git commit -m "fix: resolve wezterm API compatibility issues"
```

---

## Task 7: Run Tests

**Files:**
- Test: All existing tests

- [ ] **Step 1: Run all tests**

```bash
cargo test -p teminal-panel app::tests -- --nocapture 2>&1 | tail -20
```

Expected: "test result: ok. 16 passed"

- [ ] **Step 2: Check for warnings**

```bash
cargo clippy -p teminal-panel 2>&1 | grep "warning" | head -10
```

Expected: Only pre-existing warnings, no new ones

- [ ] **Step 3: Verify no regressions**

```bash
cargo test --workspace 2>&1 | grep "test result"
```

Expected: All tests pass

- [ ] **Step 4: Commit**

```bash
git add -A
git commit -m "test: verify all tests pass with wezterm integration"
```

---

## Task 8: Manual Testing and Verification

**Files:**
- Test: Application runtime

- [ ] **Step 1: Build application**

```bash
cargo build -p teminal-panel --release 2>&1 | tail -5
```

Expected: "Finished `release` profile"

- [ ] **Step 2: Run application**

```bash
cargo run -p teminal-panel
```

Expected: Application starts, UI displays correctly

- [ ] **Step 3: Manual verification checklist**

In the running application:
- [ ] Add a project
- [ ] Open a terminal
- [ ] Type commands (ls, pwd, etc.)
- [ ] Verify output displays correctly
- [ ] Verify colors are correct
- [ ] Verify cursor position is correct
- [ ] Verify text wrapping works
- [ ] Close terminal and verify cleanup

- [ ] **Step 4: Commit final changes**

```bash
git add -A
git commit -m "feat: complete wezterm integration with manual verification"
```

---

## Task 9: Final Verification

**Files:**
- Test: Entire workspace

- [ ] **Step 1: Full workspace build**

```bash
cargo build --workspace 2>&1 | tail -3
```

Expected: "Finished `dev` profile"

- [ ] **Step 2: Full test suite**

```bash
cargo test --workspace -- --nocapture 2>&1 | grep "test result"
```

Expected: All tests pass

- [ ] **Step 3: Clippy check**

```bash
cargo clippy --workspace 2>&1 | grep "error"
```

Expected: No errors

- [ ] **Step 4: Git log review**

```bash
git log --oneline -10
```

Expected: Shows all commits from this implementation

- [ ] **Step 5: Final commit**

```bash
git add -A
git commit -m "docs: complete wezterm integration Phase 2"
```

---

## Success Criteria Verification

- ✅ wezterm libraries integrated successfully
- ✅ All 16 existing tests pass
- ✅ Terminal functionality identical to before
- ✅ Code compiles with no errors
- ✅ No clippy warnings related to changes
- ✅ Manual testing confirms correct display
- ✅ Application runs without crashes
