# SSH/SFTP 数据面与 GUI 解耦实施计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use `superpowers:subagent-driven-development` (recommended) or `superpowers:executing-plans` to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 将 SSH、SFTP 数据面从 `gui` 解耦，建立 `GUI → DataContext → ApplicationContext API → SSH/SFTP 模块` 的统一数据流，并让 SSH/SFTP 模块各自持有运行时数据。

**Architecture:** `DataContext` 是 GUI 和 MCP 的统一中转层，负责命令路由、DTO 转换、事件分发、超时和取消；`ApplicationContext` 位于 `src/application`，负责应用级 API、会话生命周期和业务模块协调；SSH/SFTP 模块分别持有连接、缓存、任务和状态。GUI 只持有 UI 投影状态，不直接持有 SSH/SFTP runtime。

**Tech Stack:** Rust、GPUI/GPUI Kit、Tokio、`tokio::sync`、`russh`、`russh-sftp`、`alacritty_terminal`、`notify`、SQLite Storage。

**Spec:** 本计划实现本轮已确认的 `GUI → DataContext → ApplicationContext` 架构目标；现有架构说明见 `project.md`。

## 全局约束

- UI 样式参考 Tailwind CSS；图标优先复用项目已有的 Lucide 资源。
- `gui` 只负责 GPUI 渲染、交互和 UI 投影，不持有 SSH/SFTP 数据面状态。
- `DataContext` 是 GUI 与 MCP 进入应用能力的唯一中转层，GUI 不直接调用 `ApplicationContext`、SSH 模块或 SFTP 模块。
- `ApplicationContext`、DataContext、SSH 模块和 SFTP 模块不得依赖 `gpui::Context`、`Entity`、`Window`、`FocusHandle`、`ListState` 等 GUI 类型。
- SSH/SFTP 模块分别保存自己的连接、任务、快照、传输和监听状态，不暴露可变内部存储。
- GUI 边界通过 `cx.spawn + tokio` 调用异步 DataContext API；数据模块内部使用 Tokio 管理异步任务，阻塞 IO 使用 `spawn_blocking`。
- 服务端继续按照 DDD 方向组织，HTTP 只使用 GET 和 POST。
- 单个文件控制在 600–800 行；超过 800 行按职责拆分为目录和功能文件。
- 定位问题优先查看日志；新建数据流和生命周期时同步添加结构化日志。
- 不覆盖当前工作区已有的未提交修改；实现时只提交本阶段实际修改的文件。

## 目标模块关系

```text
GUI / MCP Adapter
        │
        ▼
DataContext
  ├── GuiContext API
  ├── McpContext API
  ├── command routing / DTO mapping
  └── event and snapshot bridge
        │
        ▼
ApplicationContext（src/application）
  ├── SessionApplication
  ├── SshApplication
  └── SftpApplication
        │
        ├── SSH module owns terminal runtime and terminal data
        └── SFTP module owns directory, transfer, watcher and path data
```

## 文件职责与迁移映射

### 新增 `src/application`

- `src/application/mod.rs`：声明子模块并导出应用级句柄和请求类型。
- `src/application/core.rs`：定义可克隆的 `ApplicationContext`，组装会话、SSH、SFTP 模块和应用事件总线。
- `src/application/event.rs`：定义会话、SSH、SFTP 状态变化事件，不依赖 GUI。
- `src/application/session.rs`：维护跨协议的会话元数据、打开/关闭/选择生命周期。
- `src/application/ssh/mod.rs`：SSH 应用 API 与 SSH 数据面状态入口。
- `src/application/ssh/model.rs`：终端快照、终端状态和 revision 管理。
- `src/application/ssh/buffer.rs`：从当前 GUI SSH core 移出的 ANSI/终端缓冲处理。
- `src/application/ssh/pty.rs`：PTY 生命周期、命令队列和任务取消。
- `src/application/ssh/ssh.rs`：SSH 连接、认证、读写循环和终端输入处理。
- `src/application/sftp/mod.rs`：SFTP 应用 API 与会话运行时入口。
- `src/application/sftp/model.rs`：SFTP 快照、目录条目、传输记录和状态存储。
- `src/application/sftp/connection.rs`：SFTP 客户端连接和主机密钥校验。
- `src/application/sftp/local.rs`：本地目录扫描和路径状态。
- `src/application/sftp/remote.rs`：远程目录读取、删除、上传和下载。
- `src/application/sftp/transfer.rs`：传输任务、进度、取消、重试和错误。
- `src/application/sftp/watcher.rs`：本地目录监听与自动上传任务。
- `src/application/sftp/path.rs`：本地/远程路径恢复和持久化编排。

### 修改现有模块

- `src/data_context/core.rs`：持有 `ApplicationContext` 并作为统一数据入口。
- `src/data_context/gui.rs`：将 GUI 请求路由到 `ApplicationContext`，不再把请求发送给 GUI Entity 执行。
- `src/data_context/mcp.rs`：将 MCP 请求路由到同一个 `ApplicationContext`。
- `src/data_context/command.rs`：保留中立请求类型，按会话、SSH、SFTP 职责拆分增长的命令定义。
- `src/data_context/model.rs`：维护跨 GUI/MCP 的只读 DTO，不泄露模块内部模型。
- `src/data_context/event.rs`：把应用事件转换为 DataContext 对外事件。
- `src/infrastructure/agent_mcp/external.rs`：初始化并注入 `ApplicationContext`，返回 DataContext/MCP 句柄。
- `src/gui/workspace/agent_mcp.rs`：迁移或删除当前直接操作 GUI Entity 的 MCP 命令分发逻辑。
- `src/gui/workspace/ssh/`：保留渲染、键盘、选区、滚动和 UI 交互适配；移除 `TerminalRuntime` 和终端数据存储。
- `src/gui/workspace/sftp/`：保留列表、选择、拖拽、弹窗和 UI 交互适配；移除 SFTP runtime、传输和监听数据存储。
- `src/gui/workspace/top_session/`：改为渲染 `ApplicationContext` 会话事件形成的 GUI 投影。
- `src/gui/workspace/mod.rs`、`src/gui/home/mod.rs`：初始化并注入同一个 DataContext 句柄。
- `src/global_state.rs`：逐步移除 SSH/SFTP 核心生命周期事件，保留纯 GUI 事件或主题相关状态。
- `project.md`：更新目录、分层职责、数据流和初始化流程。
- `plan.md`：记录每个阶段的完成状态、验证结果和后续计划。

## 当前完成进度

- 已拆分本地目录扫描逻辑，避免核心文件过长。
- 已阻止目录监听触发本地目录扫描。
- SFTP 打开或切换会话时，本地路径恢复后只扫描一次；远程连接只执行一次初始扫描。
- 已将本地、远程路径保存收敛到路径弹窗确认入口。
- 已在顶部菜单增加“关于”按钮，使用主窗口 Dialog 层展示 `BUILD_TIME` 编译时间。
- 已完成格式检查、静态调用链检查和全量测试。
- 已完成 SSH/SFTP 数据面与 GUI 解耦的架构评估。
- 已确认 `ApplicationContext` 放置在 `src/application`，由 DataContext 统一中转调用。

## 开发计划

### Task 1：建立 `src/application` 和 ApplicationContext 骨架

**Files:**

- Create: `src/application/mod.rs`
- Create: `src/application/core.rs`
- Create: `src/application/event.rs`
- Create: `src/application/session.rs`
- Modify: `src/main.rs`
- Modify: `src/data_context/core.rs`
- Modify: `src/data_context/mod.rs`
- Test: `src/application/core.rs` 的 `#[cfg(test)]` 模块

**Interfaces:**

- `ApplicationContext` 是 `Clone` 句柄，内部通过 `Arc` 共享应用状态。
- `DataContext` 保存 `ApplicationContext`，但只向 GUI/MCP 暴露 DataContext API。
- 应用层事件通过订阅接口发送给 DataContext，不携带 GPUI 类型。

建议先固定以下接口形状，再迁移具体数据面：

```rust
#[derive(Clone)]
pub struct ApplicationContext {
    inner: Arc<ApplicationContextInner>,
}

impl ApplicationContext {
    pub fn new(infrastructure: InfrastructureContext) -> Self;
    pub fn subscribe(&self) -> tokio::sync::broadcast::Receiver<ApplicationEvent>;
    pub fn sessions(&self) -> SessionApplication;
    pub fn ssh(&self) -> SshApplication;
    pub fn sftp(&self) -> SftpApplication;
}
```

- [ ] 记录当前工作区基线：执行 `git status --short --branch`，确认不覆盖现有未提交文件。
- [ ] 添加模块声明和最小 `ApplicationContext` 骨架，使 `cargo check` 可以通过。
- [ ] 定义 `ApplicationEvent` 的会话、SSH 状态、SFTP 状态和数据变化事件。
- [ ] 为 `ApplicationContext` 添加构造、句柄克隆和事件订阅测试。
- [ ] 执行 `cargo fmt -- --check`、`cargo check` 和 `cargo test application`。
- [ ] 将本任务的文件单独提交，提交信息使用 `refactor: add application context foundation`。

### Task 2：抽离 SSH 数据面

**Files:**

- Create: `src/application/ssh/mod.rs`
- Create: `src/application/ssh/model.rs`
- Create: `src/application/ssh/buffer.rs`
- Create: `src/application/ssh/pty.rs`
- Create: `src/application/ssh/ssh.rs`
- Modify: `src/domain/terminal.rs`
- Modify: `src/application/core.rs`
- Modify: `src/application/event.rs`
- Modify: `src/gui/workspace/ssh/mod.rs`
- Modify: `src/gui/workspace/ssh/ui.rs`
- Modify: `src/gui/workspace/ssh/internal.rs`
- Test: `src/application/ssh/model.rs`、`src/application/ssh/buffer.rs`

**Interfaces:**

`SshApplication` 只暴露异步命令和只读快照：

```rust
impl SshApplication {
    pub async fn open(&self, workspace_id: String, profile: SessionProfile)
        -> ApplicationResult<()>;
    pub async fn close(&self, workspace_id: &str) -> ApplicationResult<()>;
    pub async fn send_text(&self, workspace_id: &str, text: String)
        -> ApplicationResult<()>;
    pub async fn send_key(&self, workspace_id: &str, input: Vec<u8>)
        -> ApplicationResult<()>;
    pub async fn resize(&self, workspace_id: &str, columns: u32, rows: u32)
        -> ApplicationResult<()>;
    pub async fn read(&self, workspace_id: Option<&str>, offset: usize, limit: usize)
        -> ApplicationResult<TerminalHistoryPage>;
    pub fn snapshot(&self, workspace_id: &str) -> ApplicationResult<TerminalData>;
    pub fn subscribe(&self, workspace_id: &str)
        -> ApplicationResult<tokio::sync::watch::Receiver<TerminalData>>;
}
```

- [ ] 先为终端快照 revision、状态变更、历史读取边界和输入命令补充失败测试。
- [ ] 将当前 `gui/workspace/ssh/core/buffer.rs` 的终端解析和快照生成迁移到 `src/application/ssh/buffer.rs` 与 `model.rs`。
- [ ] 将 PTY、SSH 读写循环和任务取消迁移到 `src/application/ssh/pty.rs` 与 `ssh.rs`。
- [ ] 让 SSH runtime 存储在 `SshApplication` 内部，GUI 只能通过 API 读取快照和发送命令。
- [ ] 将终端状态变化转换为 `ApplicationEvent`，保留连接、断开和失败日志。
- [ ] 改造 `TerminalView`，移除 `HashMap<String, TerminalRuntime>`、`TerminalModel` 的所有权和直接命令发送。
- [ ] 用 `cx.spawn` 订阅 SSH 快照变化，在 GPUI 线程内更新渲染投影和 `cx.notify()`。
- [ ] 执行 `cargo test ssh`、`cargo check` 和现有全量测试。
- [ ] 将本任务的文件单独提交，提交信息使用 `refactor: move ssh data plane to application`。

### Task 3：抽离 SFTP 数据面

**Files:**

- Create: `src/application/sftp/mod.rs`
- Create: `src/application/sftp/model.rs`
- Create: `src/application/sftp/connection.rs`
- Create: `src/application/sftp/local.rs`
- Create: `src/application/sftp/remote.rs`
- Create: `src/application/sftp/transfer.rs`
- Create: `src/application/sftp/watcher.rs`
- Create: `src/application/sftp/path.rs`
- Modify: `src/application/core.rs`
- Modify: `src/application/event.rs`
- Modify: `src/gui/workspace/sftp/mod.rs`
- Modify: `src/gui/workspace/sftp/internal.rs`
- Modify: `src/gui/workspace/sftp/ui/mod.rs`
- Modify: `src/gui/workspace/sftp/ui/local.rs`
- Modify: `src/gui/workspace/sftp/ui/remote.rs`
- Modify: `src/gui/workspace/sftp/ui/select_path_dialog.rs`
- Test: `src/application/sftp/model.rs`、`src/application/sftp/path.rs`

**Interfaces:**

`SftpApplication` 负责所有 SFTP 数据和后台任务：

```rust
impl SftpApplication {
    pub async fn open(&self, workspace_id: String, profile: SessionProfile,
        initial_remote_path: Option<String>) -> ApplicationResult<()>;
    pub async fn close(&self, workspace_id: &str) -> ApplicationResult<()>;
    pub async fn list_local(&self) -> ApplicationResult<SftpDirectorySummary>;
    pub async fn change_local_directory(&self, workspace_id: &str, path: PathBuf)
        -> ApplicationResult<SftpDirectorySummary>;
    pub async fn list_remote(&self, workspace_id: &str)
        -> ApplicationResult<SftpDirectorySummary>;
    pub async fn change_remote_directory(&self, workspace_id: &str, path: String)
        -> ApplicationResult<SftpDirectorySummary>;
    pub async fn upload(&self, workspace_id: &str, paths: Vec<PathBuf>)
        -> ApplicationResult<SftpTransferSummary>;
    pub async fn download(&self, workspace_id: &str, paths: Vec<String>)
        -> ApplicationResult<SftpTransferSummary>;
    pub async fn list_transfers(&self, workspace_id: &str)
        -> ApplicationResult<SftpTransferSummary>;
    pub async fn watch_local(&self, workspace_id: &str, path: PathBuf)
        -> ApplicationResult<SftpWatchSummary>;
    pub async fn stop_watching_local(&self, workspace_id: &str, path: PathBuf)
        -> ApplicationResult<()>;
    pub async fn list_local_watches(&self, workspace_id: &str)
        -> ApplicationResult<Vec<SftpWatchSummary>>;
    pub fn snapshot(&self, workspace_id: &str) -> ApplicationResult<SftpSnapshot>;
    pub fn subscribe(&self, workspace_id: &str)
        -> ApplicationResult<tokio::sync::watch::Receiver<SftpSnapshot>>;
}
```

- [ ] 为 SFTP 快照、目录切换、路径恢复、单次扫描、传输取消和监听生命周期补充失败测试。
- [ ] 将当前 `sftp/mod.rs` 中的 `SftpEntry`、`SftpSnapshot`、`TransferRecord`、runtime 和数据状态迁移到 `src/application/sftp/model.rs`。
- [ ] 将 `core/remote.rs`、`core/local.rs`、`core/watcher.rs`、`core/delete.rs` 和 `core/path.rs` 按职责迁移到 application SFTP 子模块。
- [ ] 把 Storage 调用封装在应用 API 或注入的 port 后面，避免 GUI 方法直接读取 `Storage`。
- [ ] 让 SFTP 模块内部负责连接关闭、传输取消、监听停止和任务回收。
- [ ] 将目录、传输、监听和连接状态通过应用事件或快照订阅发布给 DataContext。
- [ ] 改造 `SftpView`，移除 runtime、传输表、监听表和路径缓存的所有权，只保留选择、拖拽、列表和弹窗状态。
- [ ] 用 `cx.spawn` 将 DataContext 快照转成 GUI 更新，保持本地和远程列表刷新节流行为。
- [ ] 执行 `cargo test sftp`、`cargo check` 和现有全量测试。
- [ ] 将本任务的文件单独提交，提交信息使用 `refactor: move sftp data plane to application`。

### Task 4：让 DataContext 成为唯一中转层

**Files:**

- Modify: `src/data_context/core.rs`
- Modify: `src/data_context/command.rs`
- Modify: `src/data_context/gui.rs`
- Modify: `src/data_context/mcp.rs`
- Modify: `src/data_context/model.rs`
- Modify: `src/data_context/event.rs`
- Modify: `src/data_context/mod.rs`
- Modify: `src/infrastructure/agent_mcp/external.rs`
- Test: `src/data_context/core.rs`、`src/data_context/gui.rs`、`src/data_context/mcp.rs`

**Interfaces:**

`DataContext` 统一持有应用句柄，并为 GUI/MCP 提供不同的输入适配，但两个适配最终调用同一个 `ApplicationContext`：

```rust
#[derive(Clone)]
pub struct DataContext {
    application: ApplicationContext,
    gui: GuiContext,
    mcp: McpContext,
}

impl DataContext {
    pub fn new(application: ApplicationContext) -> Self;
    pub fn gui(&self) -> GuiContext;
    pub fn mcp(&self) -> McpContext;
    pub fn subscribe(&self) -> tokio::sync::broadcast::Receiver<DataContextEvent>;
}
```

- [ ] 先新增 DataContext 到 ApplicationContext 的路由测试，验证 GUI/MCP 请求不会直接访问模块内部存储。
- [ ] 将 `GuiContext` 的请求实现改为调用 ApplicationContext API；保留请求超时、取消和队列满日志。
- [ ] 将 `McpContext` 的请求实现改为调用同一个 ApplicationContext API，不再依赖 GUI Entity 的 oneshot 回复。
- [ ] 将模块内部错误映射为统一 `DataContextResult<T>`，保留 workspace、profile 和 command 上下文。
- [ ] 将应用事件转换为中立 `DataContextEvent`，快照只通过只读 DTO 返回。
- [ ] 调整 `agent_mcp::start`，初始化一次 ApplicationContext，并向 MCP Controller 和 GUI 返回同一个 DataContext 句柄。
- [ ] 在兼容迁移阶段保留 `GuiContextReceiver`，只允许它转发到 DataContext；确认新 GUI 适配稳定后再删除。
- [ ] 执行 `cargo test data_context`、`cargo check` 和全量测试。
- [ ] 将本任务的文件单独提交，提交信息使用 `refactor: route gui and mcp through data context`。

### Task 5：改造 Workspace 和 GUI 投影

**Files:**

- Modify: `src/gui/home/mod.rs`
- Modify: `src/gui/workspace/mod.rs`
- Modify: `src/gui/workspace/agent_mcp.rs`
- Modify: `src/gui/workspace/ssh/mod.rs`
- Modify: `src/gui/workspace/ssh/external.rs`
- Modify: `src/gui/workspace/ssh/internal.rs`
- Modify: `src/gui/workspace/sftp/mod.rs`
- Modify: `src/gui/workspace/sftp/external.rs`
- Modify: `src/gui/workspace/sftp/internal.rs`
- Modify: `src/gui/workspace/top_session/mod.rs`
- Modify: `src/gui/workspace/top_session/core.rs`
- Modify: `src/gui/workspace/top_session/external.rs`
- Test: `src/gui/workspace/` 中的会话投影和命令适配测试

- [ ] 让 `HomeView` 或应用根初始化 DataContext，并把同一个 `GuiContext` 句柄传给 Workspace、TerminalView 和 SftpView。
- [ ] 删除 Workspace 对 `TerminalView`/`SftpView` 数据模型的直接读取，状态栏只通过 DataContext 快照或事件获取状态。
- [ ] 将终端输入、终端尺寸、终端选择、SFTP 目录切换、上传、下载和监听操作改为调用 `GuiContext` API。
- [ ] 用 `cx.spawn` 接收 DataContextEvent，将事件转成 GPUI Entity 的最小 UI 状态更新。
- [ ] 把 `agent_mcp.rs` 中直接访问 `workspace`、`terminal`、`sftp` Entity 的分支迁移到 DataContext 调用；迁移完成后删除该桥接文件或只保留 GUI 订阅适配。
- [ ] 保持终端文本选区、滚动位置、焦点、SFTP 多选、拖拽和弹窗状态只存在 GUI 层。
- [ ] 验证 GUI 和 MCP 同时操作同一 workspace 时，快照和事件来自同一 ApplicationContext。
- [ ] 执行 `cargo test workspace`、`cargo check` 和全量测试。
- [ ] 将本任务的文件单独提交，提交信息使用 `refactor: make gui consume application snapshots`。

### Task 6：统一会话生命周期和事件来源

**Files:**

- Modify: `src/application/session.rs`
- Modify: `src/application/event.rs`
- Modify: `src/data_context/event.rs`
- Modify: `src/gui/workspace/top_session/core.rs`
- Modify: `src/gui/workspace/top_session/external.rs`
- Modify: `src/gui/workspace/external.rs`
- Modify: `src/global_state.rs`
- Modify: `src/gui/sidebar_session/`
- Modify: `src/gui/title_bar/session_operation_window/`
- Test: `src/application/session.rs`、`src/gui/workspace/top_session/`

- [ ] 定义 `SessionApplication` 的打开、关闭、选择 API，并让会话元数据只在应用层维护一份。
- [ ] 将侧栏连接、标题栏创建连接、Workspace 关闭和 tab 选择全部改为 DataContext API 调用。
- [ ] 由 ApplicationContext 发布 `SessionOpened`、`SessionClosed`、`SessionSelected` 事件，WorkspaceSession 只维护 GUI 投影。
- [ ] 删除 SSH/SFTP 对 `GlobalEvent::OpenWorkspaceSession`、`CloseWorkspaceSession` 的数据面依赖。
- [ ] 保留 `GlobalState` 中仍属于 GUI 的事件，确认无 SSH/SFTP 逻辑绕过 DataContext。
- [ ] 测试打开、切换、关闭、连接失败和 MCP/GUI 并发操作的事件顺序及最终快照。
- [ ] 执行 `cargo test session`、`cargo check` 和全量测试。
- [ ] 将本任务的文件单独提交，提交信息使用 `refactor: unify session lifecycle in application`。

### Task 7：生命周期、IO 和错误回归验证

**Files:**

- Modify: `src/application/ssh/pty.rs`
- Modify: `src/application/sftp/transfer.rs`
- Modify: `src/application/sftp/watcher.rs`
- Modify: `src/data_context/gui.rs`
- Modify: `src/data_context/mcp.rs`
- Modify: `src/infrastructure/storage/`
- Test: `src/application/` 和 `src/data_context/` 的异步测试

- [ ] 验证所有 GUI 发起的异步 IO 都从 `cx.spawn` 进入 DataContext，UI 线程不执行网络、磁盘扫描和阻塞 SQLite 操作。
- [ ] 验证 SSH/SFTP 任务在关闭会话、DataContext 句柄释放和应用退出时都能取消并回收。
- [ ] 验证 DataContext 请求超时、接收端关闭、模块不存在、连接失败和传输失败都返回明确错误。
- [ ] 验证 SFTP 本地扫描、远程初始扫描、路径保存和目录监听不会重复触发。
- [ ] 补充关键日志：请求进入/完成、ApplicationContext API 调用、模块任务启动/结束、会话关闭和错误原因。
- [ ] 执行 `cargo fmt -- --check`、`cargo check`、`cargo test`，并运行一次最新构建进行手工回归。
- [ ] 将本任务的文件单独提交，提交信息使用 `test: verify application data plane lifecycle`。

### Task 8：同步项目文档和最终验收

**Files:**

- Modify: `project.md`
- Modify: `plan.md`
- Test/Review: `src/application/`、`src/data_context/`、`src/gui/workspace/`

- [ ] 更新 `project.md` 的目录结构，补充 `src/application` 各目录和文件职责。
- [ ] 更新 `project.md` 的分层职责，明确 GUI 只能调用 DataContext，DataContext 再调用 ApplicationContext API。
- [ ] 更新 `project.md` 的 SSH/SFTP 数据流、事件流和初始化生命周期。
- [ ] 在 `plan.md` 中勾选已完成任务，记录测试命令和关键日志验证结果。
- [ ] 检查 `src/application`、`src/data_context` 和 GUI 文件没有反向依赖或绕过 DataContext 的调用。
- [ ] 检查所有新增文件行数不超过项目约束，超限文件继续按职责拆分。
- [ ] 执行最终命令：`cargo fmt -- --check`、`cargo check`、`cargo test`。
- [ ] 对当前工作区变更执行 `git diff --check`，确认没有空白错误和误改文件。
- [ ] 将文档和最终验收结果单独提交，提交信息使用 `docs: update application architecture plan`。

## 当前计划

- [ ] 完成 `src/application` 的 `ApplicationContext`、事件和会话生命周期骨架。
- [ ] 完成 SSH 数据面迁移，并让 `TerminalView` 只保留 GUI 状态。
- [ ] 完成 SFTP 数据面迁移，并让 `SftpView` 只保留 GUI 状态。
- [ ] 完成 DataContext 对 GUI/MCP 的统一中转。
- [ ] 完成 Workspace、GlobalState 和会话生命周期迁移。
- [ ] 完成 IO 生命周期、日志和全量回归验证。
- [ ] 完成 `project.md` 架构文档更新。
