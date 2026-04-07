# Terminal Panel 设计文档

**日期：** 2026-04-07
**项目：** teminal-panel
**目标：** 使用 iced 构建一个多窗口终端管理工具，支持本地和 SSH 远程连接，显示 AI agent 工作区和 git 图表

## 项目概述

这是一个个人开发工具，用于监控和管理多个 AI agent 的运行状态。主要功能包括：

- 左侧面板显示 AI agent 列表（每个 agent 独立运行在不同目录）
- 右侧显示多窗口终端（支持本地和 SSH 远程）
- 对于 git 项目，显示可交互的 git 图表
- 支持动态添加/删除 SSH 连接
- 混合布局：左侧固定 agent 列表，右侧支持分割和标签切换

## 整体架构

```
┌─────────────────────────────────────────────────────────┐
│  teminal-panel                                          │
├──────────────┬──────────────────────────────────────────┤
│  Agent Panel │  Terminal Area                           │
│  (左侧固定)   │  (右侧，支持分割/标签)                    │
│              │                                          │
│ ┌──────────┐ │  ┌─────────────┬─────────────┐          │
│ │ Agent 1  │ │  │  Terminal 1 │  Terminal 2 │          │
│ │ /path/.. │ │  │             │             │          │
│ │ [git]    │ │  │             │             │          │
│ ├──────────┤ │  └─────────────┴─────────────┘          │
│ │ Agent 2  │ │  ┌─────────────────────────────┐        │
│ │ ssh://.. │ │  │  Git Graph (当选中git项目时)  │        │
│ │ [git]    │ │  │                             │        │
│ ├──────────┤ │  └─────────────────────────────┘        │
│ │ + 添加   │ │                                          │
│ └──────────┘ │                                          │
└──────────────┴──────────────────────────────────────────┘
```

### 核心模块

- **app** — iced 应用入口，管理全局状态和消息循环
- **agent_panel** — 左侧 agent 列表组件，显示 agent 名称、路径、状态
- **terminal_area** — 右侧终端区域，管理分割/标签布局
- **terminal_widget** — 单个终端实例，基于 termwiz 渲染
- **git_graph** — git 图表组件，基于 git2 + iced canvas
- **ssh_manager** — SSH 连接管理，基于 russh

## 数据模型

### Agent 配置

```rust
struct Agent {
    id: Uuid,
    name: String,
    connection: Connection,
    working_dir: PathBuf,
    is_git_repo: bool,
    status: AgentStatus,
}

enum Connection {
    Local,
    Ssh {
        host: String,
        port: u16,
        user: String,
        auth: SshAuth,
    },
}

enum SshAuth {
    Password(String),
    Key { path: PathBuf, passphrase: Option<String> },
    Agent,
}

enum AgentStatus {
    Connected,
    Disconnected,
    Connecting,
    Error(String),
}
```

### 应用状态

```rust
struct AppState {
    agents: Vec<Agent>,
    selected_agent: Option<Uuid>,
    terminals: HashMap<Uuid, Vec<TerminalState>>,
    layout: LayoutMode,
    config: AppConfig,
}

enum LayoutMode {
    Split(SplitDirection, Vec<Pane>),
    Tabs(Vec<Tab>),
}

struct TerminalState {
    id: Uuid,
    agent_id: Uuid,
    pty: PtyHandle,
    buffer: TerminalBuffer,
    scroll_offset: usize,
}
```

### 配置文件

配置文件位于 `~/.config/teminal-panel/config.toml`：

```toml
[agents]
[[agents.list]]
name = "Project A"
connection = "local"
working_dir = "/path/to/project-a"

[[agents.list]]
name = "Remote Server"
connection = { type = "ssh", host = "example.com", user = "dev", auth = "agent" }
working_dir = "/home/dev/workspace"

[layout]
mode = "split"
direction = "vertical"
ratio = 0.7
```

## 终端集成（termwiz）

### 技术选型

- **wezterm-term** — termwiz 的终端模拟器核心，处理 VT 序列解析和终端状态
- **portable-pty** — 本地 PTY 进程管理
- **russh** — SSH 协议实现
- **iced canvas** — 将 termwiz 的 cell buffer 渲染成像素

### 渲染流程

```
PTY/SSH stream
     ↓
termwiz VT parser（解析 ANSI 转义序列）
     ↓
TerminalState cell buffer（字符 + 颜色 + 属性）
     ↓
iced Canvas（逐 cell 绘制文字和背景色）
     ↓
屏幕输出
```

### 输入处理

- iced 键盘事件 → termwiz 编码 → 写入 PTY/SSH stdin
- 鼠标事件（滚动、选择）通过 termwiz 编码转发

### 实现要点

1. 使用 `wezterm-term::Terminal` 维护终端状态
2. 从 PTY/SSH 读取数据，喂给 `Terminal::advance_bytes()`
3. 从 `Terminal::screen()` 获取 cell buffer
4. 在 iced canvas 中逐行逐列绘制字符和背景色
5. 处理光标位置、选择区域、滚动缓冲区

## Git Graph 可视化

### 数据结构

```rust
struct GitGraph {
    repo_path: PathBuf,
    commits: Vec<CommitNode>,
    branches: Vec<BranchInfo>,
    layout: GraphLayout,
}

struct CommitNode {
    oid: git2::Oid,
    message: String,
    author: String,
    timestamp: i64,
    parents: Vec<git2::Oid>,
    children: Vec<git2::Oid>,
    position: (usize, usize), // (column, row)
}

struct GraphLayout {
    columns: Vec<Column>,
    rows: Vec<Row>,
    edges: Vec<Edge>,
}
```

### 布局算法

1. 从 HEAD 开始 BFS 遍历 commit 历史
2. 为每个 commit 分配列（column）— 主线在最左，分支向右展开
3. 计算 commit 之间的连线路径（处理合并、分叉）
4. 使用 iced canvas 绘制：圆点表示 commit，线条表示父子关系

### 交互功能

- 点击 commit 显示详情（diff、文件列表）
- 滚动查看历史
- 右键菜单（checkout、cherry-pick 等）

### 性能优化

- 只加载可见区域的 commit（虚拟滚动）
- 缓存布局计算结果
- 异步加载 git 数据，避免阻塞 UI

## SSH 连接管理

### 连接流程

```
用户添加 SSH agent
     ↓
验证连接参数（host、port、user）
     ↓
建立 SSH 连接（russh）
     ↓
认证（password/key/agent）
     ↓
打开 shell channel
     ↓
创建 TerminalState，接入 termwiz 渲染管道
```

### 核心实现

```rust
struct SshConnection {
    session: russh::client::Handle<SshClient>,
    channel: russh::Channel<russh::client::Msg>,
}

impl SshConnection {
    async fn connect(config: &SshConfig) -> Result<Self>;
    async fn execute(&mut self, cmd: &str) -> Result<String>;
    async fn read(&mut self) -> Result<Vec<u8>>;
    async fn write(&mut self, data: &[u8]) -> Result<()>;
}
```

### 连接管理界面

- **添加连接** — 弹出对话框，输入 host/user/auth
- **编辑连接** — 修改已保存的配置
- **删除连接** — 从配置文件移除
- **测试连接** — 验证参数是否正确

### 错误处理

- 连接超时 → 显示错误状态，允许重试
- 认证失败 → 提示重新输入密码/检查密钥
- 连接断开 → 自动重连（可配置）或显示断开状态

### 安全考虑

- 密码加密存储（使用系统 keychain）
- SSH key 路径存储，不存储 key 内容
- 支持 SSH agent 转发

## 布局系统

### 布局模式

```rust
enum LayoutMode {
    Split {
        direction: SplitDirection,
        ratio: f32,
        left: Box<LayoutNode>,
        right: Box<LayoutNode>,
    },
    Tabs {
        active: usize,
        tabs: Vec<TabContent>,
    },
    Terminal(Uuid),
    GitGraph(Uuid),
}

enum SplitDirection {
    Horizontal,
    Vertical,
}
```

### 用户操作

- 拖拽分割线调整大小
- 右键终端 → 水平/垂直分割
- 拖拽标签重新排序
- 关闭终端/标签
- 全屏某个终端

### 默认布局

- 选中 agent 时，右侧显示该 agent 的终端
- 如果是 git 项目，下方自动分割显示 git graph
- 可以手动调整布局，保存到配置文件

## 项目结构

```
teminal-panel/
├── Cargo.toml
├── src/
│   ├── main.rs                 # 入口，iced app 初始化
│   ├── app.rs                  # AppState、Message、update/view 顶层逻辑
│   ├── config.rs               # 配置文件读写（toml）
│   │
│   ├── agent/
│   │   ├── mod.rs              # Agent 数据结构
│   │   ├── panel.rs            # 左侧 agent 列表 UI 组件
│   │   └── manager.rs          # agent 增删改查
│   │
│   ├── terminal/
│   │   ├── mod.rs              # TerminalState 数据结构
│   │   ├── widget.rs           # iced canvas 终端渲染
│   │   ├── pty.rs              # 本地 PTY 管理（portable-pty）
│   │   └── area.rs             # 右侧终端区域布局管理
│   │
│   ├── ssh/
│   │   ├── mod.rs              # SSH 连接数据结构
│   │   ├── connection.rs       # russh 连接管理
│   │   └── dialog.rs           # 添加/编辑 SSH 连接的 UI 对话框
│   │
│   ├── git/
│   │   ├── mod.rs              # GitGraph 数据结构
│   │   ├── graph.rs            # git2 数据读取 + 布局算法
│   │   └── widget.rs           # iced canvas git graph 渲染
│   │
│   └── layout/
│       ├── mod.rs              # LayoutMode 数据结构
│       └── manager.rs          # 布局操作（分割、标签、拖拽）
│
└── docs/
    └── superpowers/
        └── specs/
```

## 依赖项

```toml
[dependencies]
iced = { version = "0.13", features = ["canvas", "tokio"] }
wezterm-term = "0.1"
portable-pty = "0.8"
russh = "0.44"
git2 = "0.19"
serde = { version = "1", features = ["derive"] }
toml = "0.8"
uuid = { version = "1", features = ["v4"] }
tokio = { version = "1", features = ["full"] }
```

## 实现顺序

1. **基础框架** — iced 窗口 + agent panel + 本地 PTY 终端
2. **终端渲染** — termwiz 集成，实现完整终端渲染
3. **SSH 支持** — SSH 连接管理和远程终端
4. **Git 可视化** — git graph 可视化
5. **布局系统** — 分割/标签混合布局

## 技术风险和缓解

### 风险 1：termwiz 集成复杂度

**风险：** termwiz 和 iced canvas 的集成可能比预期复杂，特别是字体渲染、颜色映射、光标处理等细节。

**缓解：**
- 先实现最小可用版本（单色、固定字体）
- 参考 wezterm 源码中的渲染实现
- 逐步添加颜色、字体、光标等特性

### 风险 2：SSH 连接稳定性

**风险：** SSH 连接可能因网络问题断开，需要处理重连、超时等边界情况。

**缓解：**
- 实现连接状态机，清晰处理各种状态转换
- 添加心跳检测和自动重连机制
- 提供手动重连按钮

### 风险 3：Git Graph 性能

**风险：** 大型仓库的 git 历史可能有数万个 commit，全量加载和渲染会导致性能问题。

**缓解：**
- 实现虚拟滚动，只加载可见区域
- 使用增量加载，按需获取更多历史
- 缓存布局计算结果

## 成功标准

1. 能够添加本地和 SSH agent，显示在左侧列表
2. 每个 agent 可以打开多个终端窗口
3. 终端支持完整的 ANSI 转义序列和颜色
4. Git 项目能够显示可交互的 commit graph
5. 支持分割和标签混合布局
6. 配置持久化到文件，重启后恢复状态
