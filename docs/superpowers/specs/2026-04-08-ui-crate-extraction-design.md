# UI Crate Extraction Design

**Date:** 2026-04-08
**Status:** Design Phase
**Scope:** Phase 1 - UI Component Library Extraction (wezterm migration in Phase 2)

## Overview

Refactor teminal-panel into a Rust workspace with two crates:
1. **ui** - Reusable UI component library (button, text_input, modal, container, theme)
2. **teminal-panel** - Application that uses the ui crate

This enables component reuse across projects and prepares the codebase for Phase 2 (wezterm integration).

## Goals

- Extract UI components into a standalone, reusable library
- Maintain 100% feature parity with current teminal-panel
- All existing tests pass without modification
- Prepare architecture for Phase 2 (wezterm-escape-parser + wezterm-term integration)

## Architecture

### Project Structure

```
teminal-panel/
├── Cargo.toml                    # Workspace root
├── ui/                           # New UI crate
│   ├── Cargo.toml
│   ├── src/
│   │   ├── lib.rs               # Public API exports
│   │   ├── components/
│   │   │   ├── mod.rs
│   │   │   ├── button.rs        # Wrapper around iced::button
│   │   │   └── text_input.rs    # Wrapper around iced::text_input
│   │   ├── containers/
│   │   │   ├── mod.rs
│   │   │   ├── modal.rs         # Modal dialog component
│   │   │   └── container.rs     # Wrapper around iced::container
│   │   └── theme.rs             # Color scheme, styling constants
│   └── README.md
├── teminal-panel/               # Renamed from src/ to teminal-panel/
│   ├── Cargo.toml
│   ├── src/
│   │   ├── main.rs
│   │   ├── app.rs               # Updated to use ui crate
│   │   ├── config.rs
│   │   ├── project/
│   │   └── terminal/
│   └── README.md
├── docs/
├── CLAUDE.md
└── .gitignore
```

### Workspace Configuration

**Root Cargo.toml:**
```toml
[workspace]
members = ["ui", "teminal-panel"]
resolver = "2"

[workspace.package]
version = "0.1.0"
edition = "2021"
```

### UI Crate Dependencies

**ui/Cargo.toml:**
```toml
[package]
name = "teminal-ui"
version.workspace = true
edition.workspace = true

[dependencies]
iced = { version = "0.13", features = ["tokio", "advanced", "canvas"] }
```

**teminal-panel/Cargo.toml:**
```toml
[package]
name = "teminal-panel"
version.workspace = true
edition.workspace = true

[dependencies]
teminal-ui = { path = "../ui" }
# ... other dependencies
```

## Component Design

### 1. Button Component

**File:** `ui/src/components/button.rs`

```rust
pub struct Button<'a, Message> {
    label: String,
    on_press: Option<Message>,
}

impl<'a, Message: 'a + Clone> Button<'a, Message> {
    pub fn new(label: impl Into<String>) -> Self { /* ... */ }
    pub fn on_press(mut self, message: Message) -> Self { /* ... */ }
    pub fn width(self, length: Length) -> Self { /* ... */ }
    pub fn into_element(self) -> Element<'a, Message> { /* ... */ }
}
```

**Usage in teminal-panel:**
```rust
use teminal_ui::components::Button;

Button::new("Add Project")
    .on_press(Message::ShowAddProjectForm)
    .into_element()
```

### 2. TextInput Component

**File:** `ui/src/components/text_input.rs`

```rust
pub struct TextInput<'a, Message> {
    placeholder: String,
    value: String,
    on_input: Option<Box<dyn Fn(String) -> Message + 'a>>,
}

impl<'a, Message: 'a> TextInput<'a, Message> {
    pub fn new(placeholder: impl Into<String>, value: &str) -> Self { /* ... */ }
    pub fn on_input<F>(mut self, f: F) -> Self
    where
        F: Fn(String) -> Message + 'a,
    { /* ... */ }
    pub fn into_element(self) -> Element<'a, Message> { /* ... */ }
}
```

### 3. Modal Container

**File:** `ui/src/containers/modal.rs`

```rust
pub struct Modal<'a, Message> {
    content: Element<'a, Message>,
    title: Option<String>,
    width: Length,
}

impl<'a, Message: 'a> Modal<'a, Message> {
    pub fn new(content: Element<'a, Message>) -> Self { /* ... */ }
    pub fn with_title(mut self, title: impl Into<String>) -> Self { /* ... */ }
    pub fn width(mut self, width: Length) -> Self { /* ... */ }
    pub fn into_element(self) -> Element<'a, Message> { /* ... */ }
}
```

**Usage in teminal-panel:**
```rust
use teminal_ui::containers::Modal;

Modal::new(form_content)
    .with_title("Add Project")
    .into_element()
```

### 4. Container Component

**File:** `ui/src/containers/container.rs`

Wrapper around iced::container with consistent styling.

```rust
pub struct Container<'a, Message> {
    content: Element<'a, Message>,
    width: Length,
    height: Length,
}

impl<'a, Message: 'a> Container<'a, Message> {
    pub fn new(content: Element<'a, Message>) -> Self { /* ... */ }
    pub fn width(mut self, width: Length) -> Self { /* ... */ }
    pub fn height(mut self, height: Length) -> Self { /* ... */ }
    pub fn into_element(self) -> Element<'a, Message> { /* ... */ }
}
```

### 5. Theme System

**File:** `ui/src/theme.rs`

```rust
pub struct Theme {
    pub primary_color: Color,
    pub background_color: Color,
    pub text_color: Color,
    pub border_color: Color,
    pub modal_background: Color,
}

impl Theme {
    pub fn dark() -> Self {
        Self {
            primary_color: Color::from_rgb8(100, 100, 100),
            background_color: Color::from_rgb8(30, 30, 30),
            text_color: Color::from_rgb8(229, 229, 229),
            border_color: Color::from_rgb8(100, 100, 100),
            modal_background: Color::from_rgb8(45, 45, 45),
        }
    }

    pub fn light() -> Self { /* ... */ }
}
```

## Phase 1 Implementation Steps

### Step 1: Create Workspace Structure
- Create `ui/` directory with Cargo.toml
- Update root Cargo.toml to define workspace
- Move teminal-panel code to `teminal-panel/` subdirectory

### Step 2: Implement UI Components
- Implement `components/button.rs` (wrapper around iced::button)
- Implement `components/text_input.rs` (wrapper around iced::text_input)
- Implement `containers/container.rs` (wrapper around iced::container)
- Implement `containers/modal.rs` (custom modal dialog)
- Implement `theme.rs` (color constants and theme)

### Step 3: Update teminal-panel
- Update imports to use `teminal_ui::*`
- Replace inline modal code with `Modal` component
- Replace button/text_input code with ui crate components
- Update Cargo.toml to depend on ui crate

### Step 4: Testing & Verification
- Run all existing tests (should pass without modification)
- Verify UI looks identical to before
- Verify all functionality works

### Step 5: Documentation
- Write README.md for ui crate
- Update CLAUDE.md with new structure
- Commit all changes

## Phase 2: wezterm Integration (Future)

After Phase 1 is complete and verified:
- Replace termwiz with wezterm-escape-parser + wezterm-term
- Update terminal module to use wezterm libraries
- May require adjustments to terminal component API
- Separate implementation plan for Phase 2

## Testing Strategy

**Phase 1 Testing:**
- All existing 16 app tests must pass without modification
- No new tests required (components are thin wrappers)
- Manual verification: UI looks and behaves identically

**Phase 2 Testing (future):**
- Terminal rendering tests with wezterm
- ANSI escape sequence parsing tests
- Integration tests with new terminal module

## Success Criteria

✅ Workspace structure created and builds successfully
✅ UI crate compiles with no warnings
✅ All 16 existing tests pass
✅ teminal-panel functionality identical to before
✅ Components are reusable (can be imported by other projects)
✅ Code is well-documented with examples

## Risks & Mitigations

| Risk | Mitigation |
|------|-----------|
| Breaking existing functionality | Keep all tests passing, verify UI visually |
| Circular dependencies | ui crate has no dependency on teminal-panel |
| Component API too restrictive | Design with extensibility in mind, use generics |
| Workspace build issues | Test workspace setup early, verify all members build |

## Out of Scope (Phase 1)

- wezterm library integration (Phase 2)
- Terminal component extraction (Terminal is app-specific)
- Advanced styling system (use iced's built-in theming)
- Component documentation site (README.md is sufficient)

## Out of Scope (Future)

- Publishing ui crate to crates.io (can be done later if needed)
- Cross-platform testing (assume iced handles this)
- Accessibility features (can be added incrementally)
