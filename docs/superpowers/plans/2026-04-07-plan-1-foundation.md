# Terminal Panel Plan 1: 基础框架 + Agent Panel + 本地 PTY 终端

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 搭建 iced 应用骨架，实现左侧 agent 列表面板和右侧基础本地 PTY 终端（文本输入/输出），配置文件持久化。

**Architecture:** iced 应用使用 `Application` trait，全局状态 `AppState` 通过 `Message` 枚举驱动更新。左侧 `AgentPanel` 组件负责列表展示和增删，右侧 `TerminalArea` 管理 PTY 进程和原始文本输出。PTY 数据通过 tokio channel 异步推送到 iced 消息循环。

**Tech Stack:** Rust, iced 0.13, portable-pty 0.8, serde + toml, uuid 1, tokio 1

---

### Task 1: 初始化 Cargo 项目

**Files:**
- Create: `Cargo.toml`
- Create: `src/main.rs`

- [ ] **Step 1: 初始化项目**

```bash
cargo init --name teminal-panel
```

- [ ] **Step 2: 写入 Cargo.toml 依赖**

```toml
[package]
name = "teminal-panel"
version = "0.1.0"
edition = "2021"

[dependencies]
iced = { version = "0.13", features = ["tokio"] }
portable-pty = "0.8"
serde = { version = "1", features = ["derive"] }
toml = "0.8"
uuid = { version = "1", features = ["v4", "serde"] }
tokio = { version = "1", features = ["full"] }
dirs = "5"
```

- [ ] **Step 3: 验证编译**

```bash
cargo check
```

Expected: 无错误

- [ ] **Step 4: Commit**

```bash
git add Cargo.toml src/main.rs
git commit -m "chore: init cargo project with dependencies"
```

---

### Task 2: 数据模型

**Files:**
- Create: `src/agent/mod.rs`
- Create: `src/config.rs`
- Modify: `src/main.rs`

- [ ] **Step 1: 创建 src/agent/mod.rs**

```rust
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Agent {
    pub id: Uuid,
    pub name: String,
    pub connection: Connection,
    pub working_dir: PathBuf,
    pub is_git_repo: bool,
    #[serde(skip)]
    pub status: AgentStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum Connection {
    Local,
    Ssh {
        host: String,
        port: u16,
        user: String,
        auth: SshAuth,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum SshAuth {
    Password(String),
    Key {
        path: PathBuf,
        passphrase: Option<String>,
    },
    Agent,
}

#[derive(Debug, Clone, Default)]
pub enum AgentStatus {
    #[default]
    Disconnected,
    Connected,
    Connecting,
    Error(String),
}

impl Agent {
    pub fn new_local(name: String, working_dir: PathBuf) -> Self {
        let is_git_repo = working_dir.join(".git").exists();
        Self {
            id: Uuid::new_v4(),
            name,
            connection: Connection::Local,
            working_dir,
            is_git_repo,
            status: AgentStatus::Disconnected,
        }
    }
}
```

- [ ] **Step 2: 创建 src/config.rs**

```rust
use crate::agent::Agent;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AppConfig {
    #[serde(default)]
    pub agents: Vec<Agent>,
}

impl AppConfig {
    pub fn config_path() -> PathBuf {
        dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("teminal-panel")
            .join("config.toml")
    }

    pub fn load() -> Self {
        let path = Self::config_path();
        if !path.exists() {
            return Self::default();
        }
        let content = std::fs::read_to_string(&path).unwrap_or_default();
        toml::from_str(&content).unwrap_or_default()
    }

    pub fn save(&self) {
        let path = Self::config_path();
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        if let Ok(content) = toml::to_string_pretty(self) {
            let _ = std::fs::write(&path, content);
        }
    }
}
```

- [ ] **Step 3: 在 src/main.rs 中声明模块**

```rust
mod agent;
mod config;

fn main() {
    println!("teminal-panel");
}
```

- [ ] **Step 4: 验证编译**

```bash
cargo check
```

Expected: 无错误

- [ ] **Step 5: Commit**

```bash
git add src/
git commit -m "feat: add Agent data model and config persistence"
```

---

### Task 3: iced 应用骨架

**Files:**
- Create: `src/app.rs`
- Modify: `src/main.rs`

- [ ] **Step 1: 创建 src/app.rs**

```rust
use crate::agent::{Agent, AgentStatus};
use crate::config::AppConfig;
use iced::{Element, Task, Theme};
use uuid::Uuid;

pub struct App {
    config: AppConfig,
    selected_agent: Option<Uuid>,
}

#[derive(Debug, Clone)]
pub enum Message {
    SelectAgent(Uuid),
    AddAgent { name: String, working_dir: String },
    RemoveAgent(Uuid),
    AgentStatusChanged(Uuid, String), // placeholder for future use
}

impl App {
    pub fn new() -> (Self, Task<Message>) {
        let config = AppConfig::load();
        (
            Self {
                config,
                selected_agent: None,
            },
            Task::none(),
        )
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::SelectAgent(id) => {
                self.selected_agent = Some(id);
            }
            Message::AddAgent { name, working_dir } => {
                let agent = Agent::new_local(name, working_dir.into());
                self.config.agents.push(agent);
                self.config.save();
            }
            Message::RemoveAgent(id) => {
                self.config.agents.retain(|a| a.id != id);
                if self.selected_agent == Some(id) {
                    self.selected_agent = None;
                }
                self.config.save();
            }
            Message::AgentStatusChanged(_, _) => {}
        }
        Task::none()
    }

    pub fn view(&self) -> Element<Message> {
        use iced::widget::{column, row, text};
        row![
            self.view_agent_panel(),
            self.view_terminal_area(),
        ]
        .into()
    }

    fn view_agent_panel(&self) -> Element<Message> {
        use iced::widget::{button, column, scrollable, text};
        let agents: Vec<Element<Message>> = self
            .config
            .agents
            .iter()
            .map(|agent| {
                let is_selected = self.selected_agent == Some(agent.id);
                let label = format!("{}\n{}", agent.name, agent.working_dir.display());
                button(text(label))
                    .on_press(Message::SelectAgent(agent.id))
                    .width(200)
                    .into()
            })
            .collect();

        column![
            text("Agents").size(16),
            scrollable(column(agents).spacing(4)),
        ]
        .spacing(8)
        .padding(8)
        .width(220)
        .into()
    }

    fn view_terminal_area(&self) -> Element<Message> {
        use iced::widget::{column, text};
        match self.selected_agent {
            Some(id) => {
                let agent = self.config.agents.iter().find(|a| a.id == id);
                match agent {
                    Some(a) => column![
                        text(format!("Terminal: {}", a.name)).size(14),
                        text("PTY not yet implemented").size(12),
                    ]
                    .padding(8)
                    .into(),
                    None => text("Agent not found").into(),
                }
            }
            None => text("Select an agent to open a terminal").into(),
        }
    }

    pub fn theme(&self) -> Theme {
        Theme::Dark
    }
}
```

- [ ] **Step 2: 更新 src/main.rs**

```rust
mod agent;
mod app;
mod config;

use app::{App, Message};

fn main() -> iced::Result {
    iced::application("teminal-panel", App::update, App::view)
        .theme(App::theme)
        .run_with(App::new)
}
```

- [ ] **Step 3: 验证编译并运行**

```bash
cargo run
```

Expected: 窗口打开，左侧显示 "Agents" 标题，右侧显示 "Select an agent to open a terminal"

- [ ] **Step 4: Commit**

```bash
git add src/app.rs src/main.rs
git commit -m "feat: add iced app skeleton with agent panel and empty terminal area"
```

---

### Task 4: Agent 增删 UI

**Files:**
- Create: `src/agent/panel.rs`
- Modify: `src/app.rs`
- Modify: `src/agent/mod.rs`

- [ ] **Step 1: 创建 src/agent/panel.rs — 添加 agent 的表单状态**

```rust
#[derive(Debug, Clone, Default)]
pub struct AddAgentForm {
    pub name: String,
    pub working_dir: String,
    pub visible: bool,
}
```

- [ ] **Step 2: 在 src/agent/mod.rs 末尾添加 pub mod panel**

```rust
pub mod panel;
```

- [ ] **Step 3: 在 src/app.rs 中集成表单状态**

在 `App` struct 中添加字段：

```rust
pub struct App {
    config: AppConfig,
    selected_agent: Option<Uuid>,
    add_form: crate::agent::panel::AddAgentForm,
}
```

在 `App::new()` 中初始化：

```rust
Self {
    config,
    selected_agent: None,
    add_form: Default::default(),
}
```

- [ ] **Step 4: 在 Message 枚举中添加表单消息**

```rust
pub enum Message {
    SelectAgent(Uuid),
    AddAgent { name: String, working_dir: String },
    RemoveAgent(Uuid),
    AgentStatusChanged(Uuid, String),
    ShowAddForm,
    HideAddForm,
    FormNameChanged(String),
    FormDirChanged(String),
    SubmitAddForm,
}
```

- [ ] **Step 5: 在 update() 中处理表单消息**

```rust
Message::ShowAddForm => {
    self.add_form.visible = true;
}
Message::HideAddForm => {
    self.add_form = Default::default();
}
Message::FormNameChanged(v) => {
    self.add_form.name = v;
}
Message::FormDirChanged(v) => {
    self.add_form.working_dir = v;
}
Message::SubmitAddForm => {
    if !self.add_form.name.is_empty() && !self.add_form.working_dir.is_empty() {
        let agent = Agent::new_local(
            self.add_form.name.clone(),
            self.add_form.working_dir.clone().into(),
        );
        self.config.agents.push(agent);
        self.config.save();
        self.add_form = Default::default();
    }
}
```

- [ ] **Step 6: 更新 view_agent_panel() 加入表单和删除按钮**

```rust
fn view_agent_panel(&self) -> Element<Message> {
    use iced::widget::{button, column, row, scrollable, text, text_input};

    let agents: Vec<Element<Message>> = self
        .config
        .agents
        .iter()
        .map(|agent| {
            let label = format!("{}\n{}", agent.name, agent.working_dir.display());
            let git_badge = if agent.is_git_repo {
                text("[git]").size(11)
            } else {
                text("").size(11)
            };
            row![
                button(text(&agent.name))
                    .on_press(Message::SelectAgent(agent.id))
                    .width(iced::Fill),
                button(text("x"))
                    .on_press(Message::RemoveAgent(agent.id)),
            ]
            .spacing(4)
            .into()
        })
        .collect();

    let add_section: Element<Message> = if self.add_form.visible {
        column![
            text_input("Name", &self.add_form.name)
                .on_input(Message::FormNameChanged),
            text_input("Directory", &self.add_form.working_dir)
                .on_input(Message::FormDirChanged),
            row![
                button(text("Add")).on_press(Message::SubmitAddForm),
                button(text("Cancel")).on_press(Message::HideAddForm),
            ]
            .spacing(4),
        ]
        .spacing(4)
        .into()
    } else {
        button(text("+ Add Agent"))
            .on_press(Message::ShowAddForm)
            .into()
    };

    column![
        text("Agents").size(16),
        scrollable(column(agents).spacing(4)),
        add_section,
    ]
    .spacing(8)
    .padding(8)
    .width(220)
    .into()
}
```

- [ ] **Step 7: 验证编译并运行**

```bash
cargo run
```

Expected: 左侧面板有 "+ Add Agent" 按钮，点击后显示表单，填写后可添加 agent，agent 旁有 "x" 删除按钮

- [ ] **Step 8: Commit**

```bash
git add src/
git commit -m "feat: add agent add/remove UI with form"
```

---

### Task 5: 本地 PTY 集成

**Files:**
- Create: `src/terminal/mod.rs`
- Create: `src/terminal/pty.rs`
- Modify: `src/app.rs`
- Modify: `src/main.rs`

- [ ] **Step 1: 创建 src/terminal/mod.rs**

```rust
pub mod pty;

use uuid::Uuid;

#[derive(Debug)]
pub struct TerminalState {
    pub id: Uuid,
    pub agent_id: Uuid,
    pub output: String,
    pub input_buf: String,
    pub writer: Box<dyn std::io::Write + Send>,
}
```

- [ ] **Step 2: 创建 src/terminal/pty.rs**

```rust
use portable_pty::{native_pty_system, CommandBuilder, PtySize};
use std::path::Path;
use tokio::sync::mpsc;

pub struct PtyHandle {
    pub writer: Box<dyn std::io::Write + Send>,
}

/// Spawn a PTY shell in the given working directory.
/// Returns (PtyHandle, receiver for output bytes).
pub fn spawn_shell(
    working_dir: &Path,
    tx: mpsc::UnboundedSender<(uuid::Uuid, Vec<u8>)>,
    terminal_id: uuid::Uuid,
) -> anyhow::Result<PtyHandle> {
    let pty_system = native_pty_system();
    let pair = pty_system.openpty(PtySize {
        rows: 24,
        cols: 80,
        pixel_width: 0,
        pixel_height: 0,
    })?;

    let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/sh".to_string());
    let mut cmd = CommandBuilder::new(&shell);
    cmd.cwd(working_dir);

    let _child = pair.slave.spawn_command(cmd)?;
    let writer = pair.master.take_writer()?;
    let mut reader = pair.master.try_clone_reader()?;

    // Spawn background thread to read PTY output
    std::thread::spawn(move || {
        let mut buf = [0u8; 4096];
        loop {
            match reader.read(&mut buf) {
                Ok(0) | Err(_) => break,
                Ok(n) => {
                    let _ = tx.send((terminal_id, buf[..n].to_vec()));
                }
            }
        }
    });

    Ok(PtyHandle { writer })
}
```

- [ ] **Step 3: 在 src/main.rs 中声明 terminal 模块**

```rust
mod agent;
mod app;
mod config;
mod terminal;
```

- [ ] **Step 4: 在 src/app.rs 中集成 PTY**

在文件顶部添加 import：

```rust
use crate::terminal::TerminalState;
use std::collections::HashMap;
use tokio::sync::mpsc;
```

在 `App` struct 中添加字段：

```rust
pub struct App {
    config: AppConfig,
    selected_agent: Option<Uuid>,
    add_form: crate::agent::panel::AddAgentForm,
    terminals: HashMap<Uuid, TerminalState>,
    pty_tx: mpsc::UnboundedSender<(Uuid, Vec<u8>)>,
    pty_rx: Option<mpsc::UnboundedReceiver<(Uuid, Vec<u8>)>>,
}
```

- [ ] **Step 5: 更新 App::new() 初始化 channel**

```rust
pub fn new() -> (Self, Task<Message>) {
    let config = AppConfig::load();
    let (pty_tx, pty_rx) = mpsc::unbounded_channel();
    (
        Self {
            config,
            selected_agent: None,
            add_form: Default::default(),
            terminals: HashMap::new(),
            pty_tx,
            pty_rx: Some(pty_rx),
        },
        Task::none(),
    )
}
```

- [ ] **Step 6: 在 Message 枚举中添加 PTY 消息**

```rust
pub enum Message {
    // ... existing variants ...
    OpenTerminal(Uuid),
    PtyOutput(Uuid, Vec<u8>),
    TerminalInput(Uuid, String),
    InputChanged(Uuid, String),
}
```

- [ ] **Step 7: 在 update() 中处理 PTY 消息**

```rust
Message::OpenTerminal(agent_id) => {
    if self.terminals.contains_key(&agent_id) {
        return Task::none();
    }
    let agent = self.config.agents.iter().find(|a| a.id == agent_id);
    if let Some(agent) = agent {
        let terminal_id = agent_id; // one terminal per agent for now
        match crate::terminal::pty::spawn_shell(
            &agent.working_dir,
            self.pty_tx.clone(),
            terminal_id,
        ) {
            Ok(handle) => {
                self.terminals.insert(agent_id, TerminalState {
                    id: terminal_id,
                    agent_id,
                    output: String::new(),
                    input_buf: String::new(),
                    writer: handle.writer,
                });
            }
            Err(e) => {
                eprintln!("Failed to spawn PTY: {e}");
            }
        }
    }
}
Message::PtyOutput(id, bytes) => {
    if let Some(term) = self.terminals.get_mut(&id) {
        term.output.push_str(&String::from_utf8_lossy(&bytes));
        // Keep last 10000 chars to avoid unbounded growth
        if term.output.len() > 10000 {
            let start = term.output.len() - 10000;
            term.output = term.output[start..].to_string();
        }
    }
}
Message::InputChanged(id, val) => {
    if let Some(term) = self.terminals.get_mut(&id) {
        term.input_buf = val;
    }
}
Message::TerminalInput(id, input) => {
    if let Some(term) = self.terminals.get_mut(&id) {
        let _ = term.writer.write_all(input.as_bytes());
        let _ = term.writer.write_all(b"\n");
        term.input_buf.clear();
    }
}
```

- [ ] **Step 8: 更新 view_terminal_area() 显示 PTY 输出**

```rust
fn view_terminal_area(&self) -> Element<Message> {
    use iced::widget::{button, column, scrollable, text, text_input};
    match self.selected_agent {
        Some(id) => {
            let agent = self.config.agents.iter().find(|a| a.id == id);
            match agent {
                Some(a) => {
                    if let Some(term) = self.terminals.get(&id) {
                        column![
                            text(format!("Terminal: {}", a.name)).size(14),
                            scrollable(
                                text(&term.output)
                                    .font(iced::Font::MONOSPACE)
                                    .size(13)
                            )
                            .height(iced::Fill),
                            text_input("$ ...", &term.input_buf)
                                .on_input(move |v| Message::InputChanged(id, v))
                                .on_submit(Message::TerminalInput(
                                    id,
                                    term.input_buf.clone(),
                                ))
                                .font(iced::Font::MONOSPACE),
                        ]
                        .padding(8)
                        .spacing(4)
                        .into()
                    } else {
                        column![
                            text(format!("Agent: {}", a.name)).size(14),
                            button(text("Open Terminal"))
                                .on_press(Message::OpenTerminal(id)),
                        ]
                        .padding(8)
                        .spacing(8)
                        .into()
                    }
                }
                None => text("Agent not found").into(),
            }
        }
        None => text("Select an agent to open a terminal").into(),
    }
}
```

- [ ] **Step 9: 订阅 PTY 输出（iced subscription）**

在 `src/app.rs` 中添加 subscription 方法：

```rust
pub fn subscription(&self) -> iced::Subscription<Message> {
    // Poll the pty_rx channel via a stream
    // We use iced::subscription::channel for async streams
    iced::subscription::channel(
        std::any::TypeId::of::<App>(),
        100,
        |mut output| async move {
            // This pattern requires storing rx somewhere accessible;
            // simplest approach: use a global once_cell
            loop {
                tokio::time::sleep(tokio::time::Duration::from_millis(16)).await;
            }
        },
    )
}
```

> **Note:** iced 0.13 的 subscription 需要通过 `iced::subscription::channel` 或 `Stream` 来桥接 tokio channel。实际实现中，将 `pty_rx` 包装成 `iced::subscription::channel` 的 stream，在每次有数据时发出 `Message::PtyOutput`。参考 iced 官方示例 `websocket.rs` 中的 channel subscription 模式。

- [ ] **Step 10: 在 main.rs 中注册 subscription**

```rust
fn main() -> iced::Result {
    iced::application("teminal-panel", App::update, App::view)
        .theme(App::theme)
        .subscription(App::subscription)
        .run_with(App::new)
}
```

- [ ] **Step 11: 验证编译**

```bash
cargo check
```

Expected: 无错误（subscription 实现可能需要根据实际 iced API 调整）

- [ ] **Step 12: Commit**

```bash
git add src/
git commit -m "feat: integrate portable-pty for local terminal with text output"
```

---

### Task 6: PTY subscription 完整实现

**Files:**
- Create: `src/terminal/subscription.rs`
- Modify: `src/terminal/mod.rs`
- Modify: `src/app.rs`

- [ ] **Step 1: 创建 src/terminal/subscription.rs**

```rust
use iced::futures::StreamExt;
use tokio::sync::mpsc;
use uuid::Uuid;

pub fn pty_subscription(
    mut rx: mpsc::UnboundedReceiver<(Uuid, Vec<u8>)>,
) -> impl iced::futures::Stream<Item = (Uuid, Vec<u8>)> {
    iced::futures::stream::unfold(rx, |mut rx| async move {
        rx.recv().await.map(|msg| (msg, rx))
    })
}
```

- [ ] **Step 2: 在 src/terminal/mod.rs 中添加 pub mod subscription**

```rust
pub mod pty;
pub mod subscription;
```

- [ ] **Step 3: 用 once_cell 存储 rx，在 subscription 中消费**

在 `Cargo.toml` 中添加：

```toml
once_cell = "1"
```

在 `src/app.rs` 中，将 `pty_rx` 移入 `once_cell::sync::OnceCell`：

```rust
use once_cell::sync::OnceCell;
static PTY_RX: OnceCell<tokio::sync::Mutex<mpsc::UnboundedReceiver<(Uuid, Vec<u8>)>>> =
    OnceCell::new();
```

在 `App::new()` 中初始化：

```rust
let (pty_tx, pty_rx) = mpsc::unbounded_channel();
PTY_RX.set(tokio::sync::Mutex::new(pty_rx)).ok();
```

- [ ] **Step 4: 实现完整 subscription**

```rust
pub fn subscription(&self) -> iced::Subscription<Message> {
    iced::subscription::channel(
        std::any::TypeId::of::<App>(),
        100,
        |mut sender| async move {
            loop {
                if let Some(rx_mutex) = PTY_RX.get() {
                    let mut rx = rx_mutex.lock().await;
                    if let Some((id, bytes)) = rx.recv().await {
                        let _ = sender.send(Message::PtyOutput(id, bytes)).await;
                    }
                } else {
                    tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
                }
            }
        },
    )
}
```

- [ ] **Step 5: 验证编译并运行**

```bash
cargo run
```

Expected:
1. 添加一个本地 agent（填写有效目录路径）
2. 点击 agent，点击 "Open Terminal"
3. 右侧显示 shell 输出（prompt）
4. 在输入框输入命令（如 `ls`），回车后显示输出

- [ ] **Step 6: Commit**

```bash
git add src/ Cargo.toml
git commit -m "feat: complete PTY subscription, local terminal now functional"
```

---

### Task 7: 配置持久化验证

**Files:**
- Modify: `src/config.rs`

- [ ] **Step 1: 验证配置文件读写**

运行应用，添加一个 agent，关闭应用，重新运行：

```bash
cargo run
```

Expected: 重启后 agent 列表仍然存在，配置文件位于 `~/.config/teminal-panel/config.toml`

- [ ] **Step 2: 检查配置文件内容**

```bash
cat ~/.config/teminal-panel/config.toml
```

Expected: 包含刚才添加的 agent 信息

- [ ] **Step 3: Commit**

```bash
git commit -m "chore: verify config persistence works correctly"
```

---

## 完成标准

Plan 1 完成后，应用应该能够：

1. 启动显示左侧 agent 列表和右侧空白区域
2. 通过表单添加本地 agent（填写名称和目录路径）
3. 删除 agent
4. 点击 agent 后点击 "Open Terminal" 打开本地 PTY shell
5. 在终端输入框输入命令并看到输出（原始文本，无 ANSI 渲染）
6. 重启后 agent 配置持久化

**下一步：** Plan 2 — termwiz 集成，实现完整 ANSI 终端渲染
