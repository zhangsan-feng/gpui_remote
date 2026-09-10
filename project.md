# 项目架构

## 分层职责

项目采用 `GUI -> DataContext -> ApplicationContext -> SSH/SFTP` 的数据流。

- `domain`：会话、协议、终端快照等业务类型，不依赖 GUI 和基础设施。
- `gui`：GPUI 视图、交互、选区、滚动、拖拽、弹窗和只读投影。GUI 不持有 SSH/SFTP 连接、任务、传输或监听运行时。
- `data_context`：GUI 与 MCP 的统一中转层，负责调用 `ApplicationContext`、校验会话、转换 DTO 和暴露通知句柄。
- `application`：应用级 API 和会话生命周期。SSH、SFTP 子模块在这里持有各自的数据面、任务、快照、传输和监听状态。
- `infrastructure`：SQLite、代理、MCP HTTP/RPC 等基础设施适配器。
- `component`：主题、列表、面板、窗口等通用 UI 组件。

GUI 的异步操作从 `cx.spawn` 进入 `GuiContext`；应用模块内部使用 Tokio 管理任务，阻塞磁盘、SQLite 和密钥加载使用 `spawn_blocking`。

## 主要数据流

```text
用户操作 / MCP 请求
        |
        v
   DataContext
   |         |
 GuiContext McpContext
        |
        v
ApplicationContext
   |          |
   v          v
SshApplication SftpApplication
   |          |
 SSH runtime  SFTP runtime / transfer / watcher
```

SSH 和 SFTP 模块各自保存真实数据。GUI 通过快照、revision 和 `Notify` 获取投影，不反向访问模块内部存储。

会话打开、关闭和选择由 `ApplicationContext` 统一协调；GUI 的标签页和全局交互状态仍由 `global_state` 驱动，具体数据操作都经 `GuiContext` 转发。

## 项目目录与文件职责

- `Cargo.toml` / `Cargo.lock`：依赖和可复现构建配置。
- `src/main.rs`：应用入口、日志、资源、全局 Storage 和 GPUI 初始化。
- `src/domain/`：会话、协议、终端领域类型。
- `src/global_state.rs`：GUI 会话投影事件和全局状态访问。
- `src/component/`：通用 UI 组件、主题、列表、面板和窗口工具。

### `src/application/`

- `mod.rs`：声明应用子模块并导出 `ApplicationContext`、应用级类型。
- `core.rs`：组装基础设施、会话、SSH 和 SFTP，并提供打开/关闭/选择会话及路径持久化 API。
- `session.rs`：维护跨协议会话元数据和选择状态。
- `event.rs`：应用会话事件类型。
- `ssh/mod.rs`：SSH 模块入口。
- `ssh/core/mod.rs`：SSH 核心类型和子模块声明。
- `ssh/core/service.rs`：SSH runtime 管理、输入、尺寸、滚动、读取和通知 API。
- `ssh/core/pty.rs`：PTY runtime 生命周期和任务回收。
- `ssh/core/ssh.rs`：SSH 连接、认证、读写循环和终端输入处理。
- `ssh/core/buffer.rs`：ANSI 解析、终端缓冲、历史读取和帧快照生成。
- `ssh/core/key.rs`：终端按键编码。
- `sftp/mod.rs`：SFTP 模块入口和对外类型导出。
- `sftp/core/mod.rs`：SFTP 快照、传输模型、命令和运行时类型。
- `sftp/core/service.rs`：SFTP 会话、目录、传输、删除、重试和监听编排。
- `sftp/core/conn.rs`：SFTP SSH 连接、认证和主机密钥校验。
- `sftp/core/remote.rs`：远程目录读取、删除、上传和下载。
- `sftp/core/local.rs`：本地目录扫描和本地删除。
- `sftp/core/watcher.rs`：本地文件监听、防抖和自动上传。
- `sftp/core/path.rs`：本地默认路径和路径工具。

### `src/data_context/`

- `mod.rs`：中转层模块声明、DTO 导出和统一结果类型。
- `core.rs`：组装 `ApplicationContext`、`GuiContext`、`McpContext` 和基础设施句柄。
- `gui.rs`：GUI API facade；所有终端、SFTP、传输和监听调用转到 `ApplicationContext`。
- `mcp.rs`：MCP API facade；与 GUI 共用同一个 `ApplicationContext`。
- `infrastructure.rs`：基础设施查询和会话存储 port 的组合。
- `query.rs`：Profile 查询服务。
- `model.rs`：GUI/MCP 使用的只读 DTO。
- `event.rs`：数据上下文事件扩展类型。

### `src/gui/`

- `home/`：首页和应用级页面。
- `sidebar_session/`：会话侧栏和配置交互。
- `title_bar/`：标题栏、设置和连接表单。
- `workspace/mod.rs`：Workspace 初始化、注入共享 `GuiContext` 和 Render 入口。
- `workspace/core.rs`：Workspace 核心状态和布局协调。
- `workspace/external.rs`：Workspace 对外事件订阅，将全局会话事件转成 DataContext 调用。
- `workspace/internal.rs`：状态栏和 UI 内部协调。
- `workspace/ui.rs`：Workspace 渲染。
- `workspace/ssh/`：SSH 终端 UI、键盘、选区、滚动和 GUI 投影同步。
- `workspace/sftp/`：SFTP 列表、选择、拖拽、路径弹窗和 GUI 投影同步。
- `workspace/top_session/`：顶部会话标签和 GUI 会话选择。

`gui` 内的 `core.rs` 负责交互编排，`ui.rs` 负责渲染，`external.rs` 负责对外暴露的 GUI 适配，`mod.rs` 负责类型、子模块、初始化和 Render 入口。

### `src/infrastructure/`

- `storage/`：SQLite 会话、SFTP 路径和已知主机密钥存储。
- `data_context/`：为 DataContext 提供 SQLite 查询和会话存储实现。
- `proxy/`：代理连接和异步双向流。
- `agent_mcp/`：MCP controller、HTTP/RPC server、工具和设置；初始化共享 DataContext。

## 初始化与生命周期

```text
main
  -> Storage / GPUI
  -> Workspace
  -> infrastructure::agent_mcp::start
  -> DataContext::new
  -> ApplicationContext::new
  -> TerminalView / SftpView 共用 GuiContext
```

`agent_mcp::start` 使用 `OnceLock` 保证同一进程复用同一个 `DataContext` 和 `ApplicationContext`。会话关闭时由应用层先停止 SSH/SFTP runtime、传输和监听任务，再移除会话元数据。
