# UI Crate Extraction Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Extract UI components into a standalone reusable library crate while maintaining 100% feature parity with current teminal-panel.

**Architecture:** Create a Rust workspace with two crates: `ui` (reusable components) and `teminal-panel` (application). Move existing code to `teminal-panel/` subdirectory, create `ui/` crate with Button, TextInput, Modal, Container, and Theme components. All components are thin wrappers around iced primitives.

**Tech Stack:** Rust, iced 0.13, workspace with path dependencies

---

## File Structure

**Files to create:**
- `ui/Cargo.toml` - UI crate manifest
- `ui/src/lib.rs` - Public API exports
- `ui/src/components/mod.rs` - Components module
- `ui/src/components/button.rs` - Button wrapper
- `ui/src/components/text_input.rs` - TextInput wrapper
- `ui/src/containers/mod.rs` - Containers module
- `ui/src/containers/container.rs` - Container wrapper
- `ui/src/containers/modal.rs` - Modal component
- `ui/src/theme.rs` - Theme and colors
- `ui/README.md` - UI crate documentation
- `teminal-panel/Cargo.toml` - Updated app manifest
- `teminal-panel/src/main.rs` - Moved from `src/main.rs`
- `teminal-panel/src/app.rs` - Updated to use ui crate
- `teminal-panel/src/config.rs` - Moved from `src/config.rs`
- `teminal-panel/src/project/` - Moved from `src/project/`
- `teminal-panel/src/terminal/` - Moved from `src/terminal/`
- `teminal-panel/README.md` - App documentation

**Files to modify:**
- `Cargo.toml` - Convert to workspace root
- `CLAUDE.md` - Update with new structure

**Files to delete:**
- `src/` directory (moved to `teminal-panel/src/`)

---

## Task 1: Create Workspace Structure

**Files:**
- Modify: `Cargo.toml`
- Create: `ui/Cargo.toml`
- Create: `teminal-panel/Cargo.toml`

- [ ] **Step 1: Backup current Cargo.toml**

```bash
cp Cargo.toml Cargo.toml.backup
```

- [ ] **Step 2: Create root workspace Cargo.toml**

Replace the entire content of `Cargo.toml` with:

```toml
[workspace]
members = ["ui", "teminal-panel"]
resolver = "2"

[workspace.package]
version = "0.1.0"
edition = "2021"
```

- [ ] **Step 3: Create ui/Cargo.toml**

Create file `ui/Cargo.toml`:

```toml
[package]
name = "teminal-ui"
version.workspace = true
edition.workspace = true

[dependencies]
iced = { version = "0.13", features = ["tokio", "advanced", "canvas"] }
```

- [ ] **Step 4: Create teminal-panel/Cargo.toml**

Create file `teminal-panel/Cargo.toml`:

```toml
[package]
name = "teminal-panel"
version.workspace = true
edition.workspace = true

[dependencies]
teminal-ui = { path = "../ui" }
iced = { version = "0.13", features = ["tokio", "advanced", "canvas"] }
portable-pty = "0.8"
serde = { version = "1", features = ["derive"] }
toml = "0.8"
uuid = { version = "1", features = ["v4", "serde"] }
tokio = { version = "1", features = ["full"] }
dirs = "5"
once_cell = "1"
termwiz = "0.23.3"
rfd = "0.17.2"
```

- [ ] **Step 5: Create ui/ and teminal-panel/ directories**

```bash
mkdir -p ui/src/{components,containers}
mkdir -p teminal-panel/src/{project,terminal}
```

- [ ] **Step 6: Verify workspace structure**

```bash
cargo build --workspace
```

Expected: Should fail because source files don't exist yet, but workspace structure is valid.

- [ ] **Step 7: Commit workspace setup**

```bash
git add Cargo.toml ui/Cargo.toml teminal-panel/Cargo.toml
git commit -m "chore: create workspace structure with ui and teminal-panel crates"
```

---

## Task 2: Create UI Crate - Theme Module

**Files:**
- Create: `ui/src/theme.rs`
- Create: `ui/src/lib.rs`

- [ ] **Step 1: Create theme.rs**

Create file `ui/src/theme.rs`:

```rust
use iced::Color;

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

    pub fn light() -> Self {
        Self {
            primary_color: Color::from_rgb8(200, 200, 200),
            background_color: Color::from_rgb8(240, 240, 240),
            text_color: Color::from_rgb8(30, 30, 30),
            border_color: Color::from_rgb8(150, 150, 150),
            modal_background: Color::from_rgb8(220, 220, 220),
        }
    }
}
```

- [ ] **Step 2: Create lib.rs**

Create file `ui/src/lib.rs`:

```rust
pub mod components;
pub mod containers;
pub mod theme;

pub use theme::Theme;
```

- [ ] **Step 3: Create components/mod.rs**

Create file `ui/src/components/mod.rs`:

```rust
pub mod button;
pub mod text_input;

pub use button::Button;
pub use text_input::TextInput;
```

- [ ] **Step 4: Create containers/mod.rs**

Create file `ui/src/containers/mod.rs`:

```rust
pub mod container;
pub mod modal;

pub use container::Container;
pub use modal::Modal;
```

- [ ] **Step 5: Verify compilation**

```bash
cargo build -p teminal-ui
```

Expected: Should compile successfully (modules are empty but valid).

- [ ] **Step 6: Commit theme and module structure**

```bash
git add ui/src/
git commit -m "feat: create ui crate with theme module and module structure"
```

---

## Task 3: Create UI Crate - Button Component

**Files:**
- Create: `ui/src/components/button.rs`

- [ ] **Step 1: Create button.rs**

Create file `ui/src/components/button.rs`:

```rust
use iced::widget;
use iced::{Element, Length};

pub struct Button<'a, Message> {
    label: String,
    on_press: Option<Message>,
    width: Length,
}

impl<'a, Message: 'a + Clone> Button<'a, Message> {
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            on_press: None,
            width: Length::Shrink,
        }
    }

    pub fn on_press(mut self, message: Message) -> Self {
        self.on_press = Some(message);
        self
    }

    pub fn width(mut self, width: Length) -> Self {
        self.width = width;
        self
    }

    pub fn into_element(self) -> Element<'a, Message> {
        let mut btn = widget::button(widget::text(self.label)).width(self.width);

        if let Some(message) = self.on_press {
            btn = btn.on_press(message);
        }

        btn.into()
    }
}
```

- [ ] **Step 2: Verify compilation**

```bash
cargo build -p teminal-ui
```

Expected: Should compile successfully.

- [ ] **Step 3: Commit button component**

```bash
git add ui/src/components/button.rs
git commit -m "feat: add Button component to ui crate"
```

---

## Task 4: Create UI Crate - TextInput Component

**Files:**
- Create: `ui/src/components/text_input.rs`

- [ ] **Step 1: Create text_input.rs**

Create file `ui/src/components/text_input.rs`:

```rust
use iced::widget;
use iced::{Element, Font};

pub struct TextInput<'a, Message> {
    placeholder: String,
    value: String,
    on_input: Option<Box<dyn Fn(String) -> Message + 'a>>,
    on_submit: Option<Message>,
}

impl<'a, Message: 'a + Clone> TextInput<'a, Message> {
    pub fn new(placeholder: impl Into<String>, value: &str) -> Self {
        Self {
            placeholder: placeholder.into(),
            value: value.to_string(),
            on_input: None,
            on_submit: None,
        }
    }

    pub fn on_input<F>(mut self, f: F) -> Self
    where
        F: Fn(String) -> Message + 'a,
    {
        self.on_input = Some(Box::new(f));
        self
    }

    pub fn on_submit(mut self, message: Message) -> Self {
        self.on_submit = Some(message);
        self
    }

    pub fn into_element(self) -> Element<'a, Message> {
        let mut input = widget::text_input(&self.placeholder, &self.value)
            .font(Font::MONOSPACE);

        if let Some(on_input) = self.on_input {
            input = input.on_input(on_input);
        }

        if let Some(on_submit) = self.on_submit {
            input = input.on_submit(on_submit);
        }

        input.into()
    }
}
```

- [ ] **Step 2: Verify compilation**

```bash
cargo build -p teminal-ui
```

Expected: Should compile successfully.

- [ ] **Step 3: Commit text_input component**

```bash
git add ui/src/components/text_input.rs
git commit -m "feat: add TextInput component to ui crate"
```

---

## Task 5: Create UI Crate - Container Component

**Files:**
- Create: `ui/src/containers/container.rs`

- [ ] **Step 1: Create container.rs**

Create file `ui/src/containers/container.rs`:

```rust
use iced::widget;
use iced::{Element, Length};

pub struct Container<'a, Message> {
    content: Element<'a, Message>,
    width: Length,
    height: Length,
}

impl<'a, Message: 'a> Container<'a, Message> {
    pub fn new(content: Element<'a, Message>) -> Self {
        Self {
            content,
            width: Length::Shrink,
            height: Length::Shrink,
        }
    }

    pub fn width(mut self, width: Length) -> Self {
        self.width = width;
        self
    }

    pub fn height(mut self, height: Length) -> Self {
        self.height = height;
        self
    }

    pub fn into_element(self) -> Element<'a, Message> {
        widget::container(self.content)
            .width(self.width)
            .height(self.height)
            .into()
    }
}
```

- [ ] **Step 2: Verify compilation**

```bash
cargo build -p teminal-ui
```

Expected: Should compile successfully.

- [ ] **Step 3: Commit container component**

```bash
git add ui/src/containers/container.rs
git commit -m "feat: add Container component to ui crate"
```

---

## Task 6: Create UI Crate - Modal Component

**Files:**
- Create: `ui/src/containers/modal.rs`

- [ ] **Step 1: Create modal.rs**

Create file `ui/src/containers/modal.rs`:

```rust
use iced::widget::{column, container, row, text};
use iced::{Color, Element, Length};

pub struct Modal<'a, Message> {
    content: Element<'a, Message>,
    title: Option<String>,
    width: Length,
}

impl<'a, Message: 'a> Modal<'a, Message> {
    pub fn new(content: Element<'a, Message>) -> Self {
        Self {
            content,
            title: None,
            width: Length::Fixed(400.0),
        }
    }

    pub fn with_title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    pub fn width(mut self, width: Length) -> Self {
        self.width = width;
        self
    }

    pub fn into_element(self) -> Element<'a, Message> {
        let mut modal_content = column![];

        if let Some(title) = self.title {
            modal_content = modal_content.push(text(title).size(20));
        }

        modal_content = modal_content.push(self.content);

        let modal = container(modal_content)
            .width(self.width)
            .style(|_| {
                container::Style::default()
                    .background(Color::from_rgb8(45, 45, 45))
                    .border(iced::Border {
                        color: Color::from_rgb8(100, 100, 100),
                        width: 1.0,
                        radius: 8.0.into(),
                    })
            })
            .padding(20);

        container(modal)
            .width(Length::Fill)
            .height(Length::Fill)
            .center_x(Length::Fill)
            .center_y(Length::Fill)
            .into()
    }
}
```

- [ ] **Step 2: Verify compilation**

```bash
cargo build -p teminal-ui
```

Expected: Should compile successfully.

- [ ] **Step 3: Commit modal component**

```bash
git add ui/src/containers/modal.rs
git commit -m "feat: add Modal component to ui crate"
```

---

## Task 7: Move Source Code to teminal-panel/

**Files:**
- Move: `src/*` → `teminal-panel/src/`

- [ ] **Step 1: Move source files**

```bash
mv src/* teminal-panel/src/
rmdir src
```

- [ ] **Step 2: Verify directory structure**

```bash
ls -la teminal-panel/src/
```

Expected: Should show main.rs, app.rs, config.rs, project/, terminal/ directories.

- [ ] **Step 3: Verify workspace builds**

```bash
cargo build --workspace
```

Expected: Should fail with import errors (we'll fix those next).

- [ ] **Step 4: Commit file move**

```bash
git add teminal-panel/src/
git rm -r src/
git commit -m "chore: move source code to teminal-panel crate"
```

---

## Task 8: Update teminal-panel to Use UI Crate

**Files:**
- Modify: `teminal-panel/src/app.rs`

- [ ] **Step 1: Add ui crate import to app.rs**

At the top of `teminal-panel/src/app.rs`, add:

```rust
use teminal_ui::components::{Button, TextInput};
use teminal_ui::containers::{Container, Modal};
use teminal_ui::Theme;
```

- [ ] **Step 2: Replace button creation in view_project_panel**

Find this line in `view_project_panel()`:

```rust
button(text("+ Add Project"))
    .width(Length::Fill)
    .on_press(Message::ShowAddProjectForm),
```

Replace with:

```rust
Button::new("+ Add Project")
    .width(Length::Fill)
    .on_press(Message::ShowAddProjectForm)
    .into_element(),
```

- [ ] **Step 3: Replace text_input in view_terminal_area**

Find this code in `view_terminal_area()`:

```rust
text_input("Project Name", &self.add_form.name)
    .on_input(Message::FormNameChanged)
    .on_submit(Message::SubmitAddProjectForm),
```

Replace with:

```rust
TextInput::new("Project Name", &self.add_form.name)
    .on_input(Message::FormNameChanged)
    .on_submit(Message::SubmitAddProjectForm)
    .into_element(),
```

- [ ] **Step 4: Replace modal creation**

Find this code in `view()`:

```rust
let modal = container(
    column![
        text("Add Project").size(20),
        text_input("Project Name", &self.add_form.name)
            .on_input(Message::FormNameChanged)
            .on_submit(Message::SubmitAddProjectForm),
        row![
            text(selected_dir).size(12),
            button(text("Choose Folder"))
                .on_press(Message::ChooseProjectFolder),
        ]
        .spacing(8)
        .align_y(iced::alignment::Vertical::Center),
        row![
            button(text("Add"))
                .width(Length::Fill)
                .on_press(Message::SubmitAddProjectForm),
            button(text("Cancel"))
                .width(Length::Fill)
                .on_press(Message::HideAddProjectForm),
        ]
        .spacing(8),
    ]
    .spacing(16)
    .padding(20)
)
.width(Length::Fixed(400.0))
.style(|_| {
    container::Style::default()
        .background(iced::Color::from_rgb8(45, 45, 45))
        .border(iced::Border {
            color: iced::Color::from_rgb8(100, 100, 100),
            width: 1.0,
            radius: 8.0.into(),
        })
});
```

Replace with:

```rust
let form_content = column![
    TextInput::new("Project Name", &self.add_form.name)
        .on_input(Message::FormNameChanged)
        .on_submit(Message::SubmitAddProjectForm)
        .into_element(),
    row![
        text(selected_dir).size(12),
        Button::new("Choose Folder")
            .on_press(Message::ChooseProjectFolder)
            .into_element(),
    ]
    .spacing(8)
    .align_y(iced::alignment::Vertical::Center),
    row![
        Button::new("Add")
            .width(Length::Fill)
            .on_press(Message::SubmitAddProjectForm)
            .into_element(),
        Button::new("Cancel")
            .width(Length::Fill)
            .on_press(Message::HideAddProjectForm)
            .into_element(),
    ]
    .spacing(8),
]
.spacing(16);

let modal = Modal::new(form_content)
    .with_title("Add Project")
    .into_element();
```

- [ ] **Step 5: Verify compilation**

```bash
cargo build -p teminal-panel
```

Expected: Should compile successfully.

- [ ] **Step 6: Run tests**

```bash
cargo test -p teminal-panel app::tests -- --nocapture
```

Expected: All 16 tests should pass.

- [ ] **Step 7: Commit ui crate integration**

```bash
git add teminal-panel/src/app.rs
git commit -m "feat: integrate ui crate components into teminal-panel"
```

---

## Task 9: Create Documentation

**Files:**
- Create: `ui/README.md`
- Create: `teminal-panel/README.md`
- Modify: `CLAUDE.md`

- [ ] **Step 1: Create ui/README.md**

Create file `ui/README.md`:

```markdown
# teminal-ui

A reusable UI component library built with Iced.

## Components

### Button

```rust
use teminal_ui::components::Button;

Button::new("Click me")
    .on_press(Message::ButtonPressed)
    .into_element()
```

### TextInput

```rust
use teminal_ui::components::TextInput;

TextInput::new("Placeholder", &value)
    .on_input(Message::InputChanged)
    .on_submit(Message::Submit)
    .into_element()
```

### Modal

```rust
use teminal_ui::containers::Modal;

Modal::new(content)
    .with_title("Dialog Title")
    .into_element()
```

### Container

```rust
use teminal_ui::containers::Container;

Container::new(content)
    .width(Length::Fill)
    .height(Length::Fill)
    .into_element()
```

### Theme

```rust
use teminal_ui::Theme;

let theme = Theme::dark();
```

## Features

- Thin wrappers around Iced primitives
- Consistent styling and theming
- Reusable across projects
- Type-safe message handling
```

- [ ] **Step 2: Create teminal-panel/README.md**

Create file `teminal-panel/README.md`:

```markdown
# teminal-panel

A GUI application for managing multiple project terminals.

## Building

```bash
cargo build -p teminal-panel
```

## Running

```bash
cargo run -p teminal-panel
```

## Testing

```bash
cargo test -p teminal-panel
```

## Architecture

- **ui crate** - Reusable UI components
- **teminal-panel crate** - Application using ui components
- **terminal module** - Terminal emulation and PTY management
- **config module** - Configuration persistence
- **project module** - Project management

See CLAUDE.md for detailed architecture documentation.
```

- [ ] **Step 3: Update CLAUDE.md**

Open `CLAUDE.md` and update the "Module Structure" section to:

```markdown
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
```

- [ ] **Step 4: Verify all files exist**

```bash
ls -la ui/README.md teminal-panel/README.md
```

Expected: Both files should exist.

- [ ] **Step 5: Commit documentation**

```bash
git add ui/README.md teminal-panel/README.md CLAUDE.md
git commit -m "docs: add README files and update CLAUDE.md for workspace structure"
```

---

## Task 10: Final Verification and Testing

**Files:**
- Test: All crates

- [ ] **Step 1: Build entire workspace**

```bash
cargo build --workspace
```

Expected: Should build successfully with no errors.

- [ ] **Step 2: Run all tests**

```bash
cargo test --workspace -- --nocapture
```

Expected: All 16 app tests should pass, no test failures.

- [ ] **Step 3: Check for warnings**

```bash
cargo clippy --workspace
```

Expected: No clippy warnings related to our changes.

- [ ] **Step 4: Verify UI functionality**

Run the application:

```bash
cargo run -p teminal-panel
```

Expected: Application should start, UI should look identical to before, all features should work (add project, open terminal, type commands, etc.).

- [ ] **Step 5: Verify workspace structure**

```bash
tree -L 2 -I target
```

Expected: Should show:
```
.
├── Cargo.toml
├── ui/
│   ├── Cargo.toml
│   └── src/
├── teminal-panel/
│   ├── Cargo.toml
│   └── src/
└── docs/
```

- [ ] **Step 6: Final commit**

```bash
git log --oneline -10
```

Expected: Should show all commits from this plan.

- [ ] **Step 7: Create summary**

```bash
git log --oneline --since="2 hours ago"
```

Expected: Should show all commits from this implementation.

---

## Success Criteria Verification

- ✅ Workspace structure created and builds successfully
- ✅ UI crate compiles with no warnings
- ✅ All 16 existing tests pass
- ✅ teminal-panel functionality identical to before
- ✅ Components are reusable (can be imported by other projects)
- ✅ Code is well-documented with examples

---

## Next Steps (Phase 2)

After this plan is complete:
1. Create separate implementation plan for wezterm integration
2. Replace termwiz with wezterm-escape-parser + wezterm-term
3. Update terminal module to use wezterm libraries
4. Run full test suite to verify compatibility
