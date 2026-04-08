# Add Project Native Folder Picker 设计文档

**日期：** 2026-04-08
**项目：** teminal-panel
**目标：** 将左侧面板中的 `Add Agent` 交互完整迁移为 `Add Project`，并通过系统原生目录选择窗口选择项目目录。

## 背景

当前左侧面板使用 `Agent` 语义，新增入口为 `+ Add Agent`，表单包含 `Name` 与手工输入的 `Directory`。这与当前产品语义不一致，且目录输入容易出错。

本次变更的目标是：

- 将用户可见文案从 `Agent` 统一改为 `Project`
- 保留项目名称手工输入
- 将目录录入改为系统原生目录选择窗口
- 将内部数据结构和配置主字段统一迁移到 `Project` 语义
- 保持旧配置可读取，避免升级后现有数据丢失

## 用户交互

左侧面板调整如下：

- 标题从 `Agents` 改为 `Projects`
- 底部按钮从 `+ Add Agent` 改为 `+ Add Project`
- 展开后显示：
  - `Name` 输入框
  - 当前已选择目录的只读文本
  - `Choose Folder` 按钮
  - `Add` / `Cancel` 按钮

交互流程：

1. 用户点击 `+ Add Project`
2. 用户输入项目名称
3. 用户点击 `Choose Folder`
4. 应用打开系统原生目录选择窗口
5. 用户选中一个目录后，表单更新为该目录路径
6. 用户点击 `Add`
7. 应用校验名称和目录有效后，创建一个本地 project 并写入配置

边界行为：

- 用户取消目录选择时，不报错，保持表单原状
- 重新选择目录时，覆盖此前已选目录
- 名称不自动取目录名，始终允许手工输入
- 不新增去重限制，保持与当前行为一致

## 数据模型变更

将内部命名从 `Agent` 迁移到 `Project`：

```rust
struct Project {
    id: Uuid,
    name: String,
    connection: Connection,
    working_dir: PathBuf,
    is_git_repo: bool,
    status: ProjectStatus,
}
```

对应状态命名同步迁移：

- `AgentStatus` -> `ProjectStatus`
- `AddAgentForm` -> `AddProjectForm`
- `selected_agent` -> `selected_project`
- `AddAgent` / `RemoveAgent` / `SelectAgent` 等消息改为 `Project` 语义

说明：

- `connection`、`working_dir`、`is_git_repo` 的实际能力不变
- 本次只做语义和入口升级，不改变本地终端启动方式

## 配置兼容策略

配置主字段从：

```toml
agents = [...]
```

迁移为：

```toml
projects = [...]
```

采用“向后兼容读取，向前统一写入”策略：

- 读取配置时同时兼容旧字段 `agents`
- 当 `projects` 为空且存在旧字段 `agents` 时，将旧数据映射为 `projects`
- 保存配置时只写新字段 `projects`

这样可以保证：

- 老用户升级后无需手动迁移配置
- 新版本保存过配置后，配置文件会自然收敛到新字段

## 目录选择实现

使用原生文件对话框库接入系统目录选择窗口，选择目录而不是单文件。

实现约束：

- UI 触发一个异步任务打开目录选择窗口
- 返回值为 `Option<PathBuf>`
- 只有用户实际选中了存在的目录时，才更新表单状态

表单状态建议为：

```rust
struct AddProjectForm {
    name: String,
    selected_dir: Option<PathBuf>,
    visible: bool,
}
```

展示层将 `selected_dir` 转换为字符串显示，避免路径字符串和真实路径状态重复维护。

## 视图与消息流

新增或调整消息：

- `ShowAddProjectForm`
- `HideAddProjectForm`
- `FormNameChanged(String)`
- `ChooseProjectFolder`
- `ProjectFolderSelected(Option<PathBuf>)`
- `SubmitAddProjectForm`
- `SelectProject(Uuid)`
- `RemoveProject(Uuid)`
- `OpenTerminal(Uuid)`

消息流：

1. `ChooseProjectFolder`
2. 启动原生目录选择任务
3. 任务完成后发出 `ProjectFolderSelected(...)`
4. 更新表单状态
5. `SubmitAddProjectForm`
6. 校验并持久化 project

## 终端区域影响

右侧终端能力保持不变，但全部文案和状态字段改用 `Project` 语义：

- `Terminal: {project.name}`
- `Project: {project.name}`
- `Project not found`
- `Select a project to open a terminal`

删除 project 时，仍需：

- 从配置集合中移除对应项
- 关闭对应终端生命周期
- 清理当前选中状态

## 测试策略

本次改动至少覆盖以下验证：

1. 配置兼容读取
   - 旧 `agents` 配置可以成功映射为 `projects`
2. 新配置写入
   - 保存时只输出 `projects`
3. 表单校验
   - 名称为空时不可创建
   - 未选择目录时不可创建
   - 目录不存在或不是目录时不可创建
4. 创建 project 逻辑
   - 合法目录可成功加入配置
   - `is_git_repo` 根据目录下 `.git` 是否存在正确推断

## 非目标

本次不包含以下内容：

- SSH project 交互改造
- 项目去重规则
- 多终端布局能力调整
- Git graph 相关行为变更
- 配置文件自动批量升级脚本

## 实施摘要

实现时将按以下顺序推进：

1. 增加配置兼容层，支持 `agents` -> `projects`
2. 将核心数据结构与应用状态统一迁移为 `Project` 语义
3. 接入系统原生目录选择窗口
4. 更新左侧表单与右侧文案
5. 补充单元测试并运行验证
