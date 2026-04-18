# Git Window 设计文档

**日期:** 2026-04-18  
**状态:** 已批准

## 概述

为 teminal-panel 应用添加一个独立的 Git 窗口功能。当用户点击终端 footer 中的 git 图标时，打开一个新的操作系统窗口，显示 git 仓库的状态和历史。

## 目标

- 提供可视化的 git 仓库浏览体验
- 显示文件变更列表（工作区 + 暂存区）
- 显示 git 提交历史图（git graph）
- 支持查看文件 diff
- 类似 VSCode 的 tab 布局

## 用户体验

### 触发方式

用户点击终端下方 footer 中的 git 图标，打开独立的 Git 窗口。

### 窗口布局

```
┌─────────────────────────────────────────────────────────────┐
│  Git Window - Project Name                                   │
├───────────────┬─────────────────────────────────────────────┤
│               │ [Git Graph] [file.rs] [main.rs]             │
│  Changes      ├─────────────────────────────────────────────┤
│               │                                              │
│  Unstaged:    │                                              │
│  ● file.rs    │         Git Graph Canvas                     │
│  ● main.rs    │         或                                    │
│               │         File Diff View                       │
│  Staged:      │                                              │
│  ● test.rs    │                                              │
│               │                                              │
│               │                                              │
└───────────────┴─────────────────────────────────────────────┘
```

- **左侧面板**（300px 固定宽度）：文件变更列表
  - 工作区更改（Unstaged）
  - 暂存区更改（Staged）
- **右侧面板**（自适应宽度）：Tab 视图
  - 默认 Tab: Git Graph
  - 点击文件后：新增 Diff Tab

### 交互流程

1. 用户点击 git 图标 → 打开 Git 窗口
2. 窗口显示文件变更列表和 Git Graph
3. 用户点击左侧文件 → 右侧新增 Diff Tab 并切换到该 Tab
4. 用户可以在 Tab 之间切换查看不同内容

## 技术架构

### 技术栈

- **GUI 框架**: Iced 0.13（多窗口支持）
- **Git 库**: git2-rs 0.19（libgit2 的 Rust 绑定）
- **绘图**: Iced Canvas（用于绘制 git graph）
- **语言**: Rust

### 核心组件

#### 1. GitWindow

主窗口结构，管理整个 Git 窗口的状态。

```rust
pub struct GitWindow {
    project_id: Uuid,
    project_name: String,
    repo_path: PathBuf,
    repository: git2::Repository,
    
    // 左侧面板
    unstaged_files: Vec<FileChange>,
    staged_files: Vec<FileChange>,
    selected_file: Option<PathBuf>,
    
    // 右侧面板
    tabs: Vec<Tab>,
    active_tab: usize,
    
    // Git Graph 数据
    commits: Vec<CommitNode>,
    graph_scroll: f32,
}

pub struct FileChange {
    path: PathBuf,
    status: FileStatus,  // Added, Modified, Deleted
}

pub enum Tab {
    GitGraph,
    FileDiff { path: PathBuf, diff: String },
}

pub struct CommitNode {
    oid: git2::Oid,
    message: String,
    author: String,
    timestamp: i64,
    parents: Vec<git2::Oid>,
    column: usize,  // 用于布局
    row: usize,     // 用于布局
}
```

#### 2. FileChangesList

左侧文件变更列表组件。

功能：
- 显示工作区和暂存区的文件
- 显示文件状态图标（新增/修改/删除）
- 支持点击选择文件
- 支持文件暂存/取消暂存操作（Phase 3）

#### 3. GitGraphCanvas

使用 Iced Canvas 绘制的 Git Graph 组件。

功能：
- 绘制提交节点（彩色圆圈）
- 绘制分支连接线
- 显示提交信息（hash、message、author）
- 显示分支标签
- 支持滚动查看历史

布局算法：
1. 从 HEAD 开始遍历提交历史
2. 为每个分支分配一个列（column）
3. 计算提交节点坐标：
   - x = column * 30px
   - y = row * 40px
4. 绘制父子提交之间的连接线
5. 使用不同颜色区分不同分支

#### 4. DiffView

文件 diff 显示组件。

功能：
- 显示文件的 diff 内容
- 语法高亮（新增行绿色，删除行红色）
- 显示行号
- 支持滚动

#### 5. TabBar

Tab 栏组件。

功能：
- 显示所有打开的 Tab
- 支持切换 Tab
- 支持关闭 Tab（除了 Git Graph Tab）

### 消息定义

```rust
// Git 窗口消息
#[derive(Debug, Clone)]
pub enum GitWindowMessage {
    // 文件列表相关
    SelectFile(PathBuf),
    RefreshFileList,
    StageFile(PathBuf),      // Phase 3
    UnstageFile(PathBuf),    // Phase 3
    
    // Tab 相关
    SelectTab(usize),
    CloseTab(usize),
    
    // Git Graph 相关
    GraphScroll(f32),
    SelectCommit(git2::Oid), // Phase 3
    
    // 窗口控制
    CloseWindow,
}

// 主应用新增消息
pub enum Message {
    // ... 现有消息
    OpenGitWindow(Uuid),  // 打开 git 窗口
}
```

### 文件结构

```
teminal-panel/src/
├── git_window/
│   ├── mod.rs           # GitWindow 主结构和消息处理
│   ├── file_list.rs     # 文件变更列表组件
│   ├── git_graph.rs     # Git Graph Canvas 组件
│   ├── diff_view.rs     # Diff 显示组件
│   ├── tab_bar.rs       # Tab 栏组件
│   └── git_data.rs      # Git 数据获取（git2 封装）
```

### 数据流

#### 打开 Git 窗口流程

1. 用户点击终端 footer 的 git 图标
2. 主应用发送 `Message::OpenGitWindow(project_id)`
3. 主应用的 `update()` 方法处理消息
4. 使用 Iced 的多窗口 API 创建新窗口
5. 新窗口初始化 `GitWindow` 状态：
   - 使用 `git2::Repository::open()` 打开仓库
   - 加载文件变更列表
   - 加载提交历史
6. 渲染窗口界面

#### 文件选择流程

1. 用户点击左侧文件列表中的文件
2. 发送 `GitWindowMessage::SelectFile(path)`
3. `GitWindow::update()` 处理消息：
   - 使用 git2 获取文件的 diff
   - 创建新的 `Tab::FileDiff`
   - 添加到 tabs 列表
   - 切换到新 Tab
4. 重新渲染界面

#### Git Graph 交互

1. 使用 `git2::Repository::revwalk()` 遍历提交历史
2. 为每个提交创建 `CommitNode`
3. 计算布局（column 和 row）
4. 在 Canvas 中绘制：
   - 绘制连接线（使用 `Path`）
   - 绘制提交节点（使用 `Circle`）
   - 绘制文本标签（使用 `Text`）
5. 处理滚动事件更新 `graph_scroll`

## UI 设计

### 颜色方案

**背景色：**
- 窗口背景：`rgb(0.1, 0.1, 0.1)`
- 左侧面板背景：`rgb(0.12, 0.12, 0.12)`
- Tab 栏背景：`rgb(0.15, 0.15, 0.15)`

**文件状态颜色：**
- 新增文件：绿色 `rgb(0.3, 0.8, 0.3)`
- 修改文件：黄色 `rgb(0.9, 0.7, 0.2)`
- 删除文件：红色 `rgb(0.9, 0.3, 0.3)`

**Git Graph 颜色：**
- 分支 1：蓝色 `rgb(0.3, 0.5, 0.9)`
- 分支 2：绿色 `rgb(0.3, 0.8, 0.5)`
- 分支 3：橙色 `rgb(0.9, 0.6, 0.2)`
- 分支 4：紫色 `rgb(0.7, 0.3, 0.8)`

**Diff 视图颜色：**
- 新增行背景：`rgba(0.2, 0.6, 0.2, 0.2)`
- 删除行背景：`rgba(0.8, 0.2, 0.2, 0.2)`
- 行号：`rgb(0.5, 0.5, 0.5)`

### 布局尺寸

- 窗口默认大小：1200x800
- 最小窗口大小：800x600
- 左侧面板宽度：300px（固定）
- Tab 栏高度：40px
- 文件列表项高度：28px
- Git Graph 行高：40px
- Git Graph 列宽：30px
- 提交节点半径：6px

### 字体和图标

- 字体：系统默认等宽字体
- 文件状态图标：使用 Bootstrap Icons
  - 新增：`bootstrap::file_plus()`
  - 修改：`bootstrap::file_diff()`
  - 删除：`bootstrap::file_minus()`
- Git 图标：`bootstrap::git()`

## 错误处理

### 错误场景

1. **项目不是 git 仓库**
   - 检测：`git2::Repository::open()` 返回错误
   - 处理：显示提示信息 "This project is not a git repository"
   - UI：在窗口中央显示错误消息

2. **Git 操作失败**
   - 场景：获取文件变更、提交历史、diff 失败
   - 处理：捕获 git2 错误，显示错误消息
   - UI：在相应区域显示错误提示

3. **文件 diff 获取失败**
   - 场景：文件已删除、权限问题等
   - 处理：显示错误消息而不是 diff 内容
   - UI：在 Diff Tab 中显示错误信息

### 错误消息格式

```rust
pub enum GitError {
    NotARepository,
    OperationFailed(String),
    FileNotFound(PathBuf),
}

impl GitError {
    pub fn display_message(&self) -> String {
        match self {
            GitError::NotARepository => 
                "This project is not a git repository".to_string(),
            GitError::OperationFailed(msg) => 
                format!("Git operation failed: {}", msg),
            GitError::FileNotFound(path) => 
                format!("File not found: {}", path.display()),
        }
    }
}
```

## 测试策略

### 单元测试

测试 `git_data.rs` 中的 git2 封装函数：
- 测试获取文件变更列表
- 测试获取提交历史
- 测试获取文件 diff
- 测试错误处理

### 集成测试

- 测试 GitWindow 初始化
- 测试文件选择和 Tab 创建
- 测试 Git Graph 数据加载

### 手动测试

- 测试窗口打开和关闭
- 测试 UI 交互（点击文件、切换 Tab）
- 测试 Canvas 绘制效果
- 测试不同 git 仓库状态（有/无更改、不同分支数量）
- 测试大仓库性能

## 实现计划

### Phase 1: MVP（最小可行产品）

**目标：** 基本功能可用

1. 创建独立窗口
   - 实现 `GitWindow` 结构
   - 集成 Iced 多窗口 API
   - 添加 `OpenGitWindow` 消息处理

2. 显示文件变更列表
   - 实现 `git_data.rs` 中的文件变更获取
   - 实现 `FileChangesList` 组件
   - 显示工作区和暂存区文件

3. 简单的 Git Graph 显示
   - 实现线性提交历史加载
   - 实现基本的 Canvas 绘制
   - 显示提交节点和连接线（单分支）

**预计时间：** 3-5 天

### Phase 2: 完整功能

**目标：** 完整的 Git 可视化体验

4. 完整的 Git Graph
   - 实现多分支布局算法
   - 支持分支合并的可视化
   - 添加分支标签显示

5. Diff 视图显示
   - 实现 `DiffView` 组件
   - 实现文件 diff 获取
   - 添加语法高亮

6. Tab 切换功能
   - 实现 `TabBar` 组件
   - 实现 Tab 切换逻辑
   - 实现 Tab 关闭功能

**预计时间：** 5-7 天

### Phase 3: 增强功能（可选）

**目标：** 提升用户体验

7. Git Graph 交互
   - 点击提交查看详情
   - 显示提交的完整信息
   - 支持复制 commit hash

8. 文件操作
   - 文件暂存（stage）
   - 文件取消暂存（unstage）
   - 文件丢弃更改（discard）

9. 性能优化
   - 虚拟滚动（大量提交）
   - 延迟加载提交历史
   - Canvas 绘制优化

**预计时间：** 3-5 天

## 依赖项

需要在 `Cargo.toml` 中添加：

```toml
[dependencies]
git2 = "0.19"
```

Iced 0.13 已经在项目中，支持多窗口功能。

## 风险和限制

### 技术风险

1. **Iced 多窗口 API 的限制**
   - 风险：Iced 的多窗口支持可能有限制
   - 缓解：提前验证多窗口 API，必要时使用单窗口 + 覆盖层方案

2. **Git Graph 布局算法复杂度**
   - 风险：复杂的分支结构难以布局
   - 缓解：从简单算法开始，逐步优化

3. **大仓库性能问题**
   - 风险：大量提交导致加载和渲染缓慢
   - 缓解：实现分页加载和虚拟滚动

### 功能限制

1. **只读功能**
   - Phase 1 和 2 只支持查看，不支持 git 操作
   - Phase 3 才添加基本的 git 操作

2. **简化的 Git Graph**
   - 初期版本可能无法完美处理所有复杂的分支结构
   - 优先支持常见的分支模式

## 未来扩展

- 支持更多 git 操作（commit、push、pull）
- 支持查看远程分支
- 支持搜索提交
- 支持 blame 视图
- 支持 stash 管理
- 支持子模块显示

## 参考资料

- [git2-rs 文档](https://docs.rs/git2/)
- [Iced 文档](https://docs.rs/iced/)
- [GitUI 项目](https://github.com/extrawurst/gitui) - 参考实现
- [gitgraph-core](https://lib.rs/crates/gitgraph-core) - Git graph 算法参考
