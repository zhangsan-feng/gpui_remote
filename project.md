## 目录 / 文件职责

- `Cargo.toml`：项目依赖、构建配置和开发构建优化；维护 gpui-kit 迁移后的直接依赖。
- `Cargo.lock`：可复现的依赖解析结果；锁定 GPUI、platform、macros 和 HTTP client 的同批次版本。
- `project.md`：记录项目目录和文件职责。
- `src/build_info.rs`：提供构建时间等构建信息。
- `src/main.rs`：应用入口、日志初始化、合并本地图标与 Kit 资源、HTTP client 注入、主窗口初始化。
- `src/domain/`：会话、协议和终端等业务领域类型。
- `src/global_state.rs`：全局会话状态类型、事件和 GPUI 全局状态访问。
- `src/component/`：通用 UI 组件、颜色、可拖拽列表、可调整面板和窗口辅助工具。
- `src/component/theme/`：主题数据、主题初始化、颜色配置和窗口背景外观。
- `src/gui/`：主界面渲染入口和页面级 UI 组织。
- `src/gui/sidebar_session/`：会话侧栏、会话输入、连接/编辑/删除操作和交互状态。
- `src/gui/title_bar/`：标题栏、会话创建、设置入口及相关操作窗口。
- `src/gui/title_bar/session_operation_window/`：SSH/SFTP 会话表单、输入状态和创建/编辑逻辑。
- `src/gui/title_bar/settings_operation_window/`：应用设置表单、主题与 MCP 设置界面。
- `src/gui/workspace/`：会话工作区、标签页、终端和 SFTP 内容布局。
- `src/gui/workspace/ssh/`：SSH 终端 UI、输入、文本选择和运行时状态。
- `src/gui/workspace/ssh/core/`：终端缓冲区、PTY、SSH 连接和终端核心数据流。
- `src/gui/workspace/sftp/`：SFTP 列表、文件操作、选择与传输界面。
- `src/gui/workspace/sftp/core/`：SFTP 本地/远程数据、连接和文件操作核心逻辑。
- `src/gui/workspace/sftp/core/delete.rs`：本地与远程批量删除编排、目录刷新和错误汇总。
- `src/gui/workspace/sftp/ui/`：SFTP 本地/远程列表、选择和路径弹窗渲染。
- `src/gui/workspace/sftp/ui/path_dialog_title_bar.rs`：SFTP 路径弹窗的私有标题栏和窗口控制。
- `src/gui/workspace/top_session/`：工作区顶部会话标签及会话切换状态。
- `src/data_context/`：GUI、MCP 与基础设施之间的核心中转层、命令总线、查询服务和中立 DTO。
- `src/data_context/core.rs`：DataContext 组合根、上下文持有和核心生命周期。
- `src/data_context/bus.rs`：命令、查询和事件通道的容量与生命周期配置。
- `src/data_context/mcp.rs`：McpContext 对外提供的协议无关能力接口。
- `src/data_context/gui.rs`：GuiContext、GUI receiver 和 GUI 命令适配。
- `src/data_context/infrastructure.rs`：InfrastructureContext 与基础设施 ports 的组合。
- `src/data_context/command.rs`：中立的数据上下文命令类型。
- `src/data_context/query.rs`：查询 port 与查询服务。
- `src/data_context/model.rs`：跨 GUI/MCP/Infrastructure 使用的中立 DTO。
- `src/data_context/event.rs`：数据上下文事件类型和订阅扩展点。
- `src/infrastructure/agent_mcp/`：MCP HTTP/RPC adapter、server、tools 和控制器。
- `src/infrastructure/data_context/`：DataContext ports 的 SQLite 等基础设施适配器。
- `src/infrastructure/proxy/`：代理连接建立和异步双向流抽象。
- `src/infrastructure/storage/`：SQLite 会话存储、已知主机密钥和持久化基础设施。

## 当前项目架构

项目采用“GUI 主导、DataContext 中转、MCP 外部适配、Infrastructure 提供基础能力”的分层结构。`DataContext` 是 MCP 与 GUI 之间的核心边界，不直接替代 Workspace，也不承载 GUI 的普通交互逻辑。

### 分层职责

- `domain`：定义会话、终端和协议等业务基础类型，不依赖 GUI、MCP 或具体基础设施。
- `gui`：负责桌面界面和用户交互。普通 GUI 输入直接在 `Workspace`、`Ssh`、`Sftp` 等实体内处理，保持实时交互，不经过 `DataContext`。
- `data_context`：负责跨边界的数据协调。`DataContext` 持有 `GuiContext`、`McpContext` 和 `InfrastructureContext`，向外提供中立 DTO、命令、查询和事件类型。
- `infrastructure/agent_mcp`：MCP 的 HTTP/RPC 适配层，负责认证、服务生命周期、工具注册和协议转换，不直接操作 GUI Entity。
- `infrastructure/data_context`：实现 DataContext 的基础设施 ports，当前通过 SQLite Repository 提供连接配置查询。
- `infrastructure/storage`：提供 SQLite 持久化、会话配置和 SSH 已知主机密钥等能力。
- `component`：提供主题、布局、列表、面板和窗口等通用 UI 能力。

### DataContext 内部关系

```text
DataContext
├── GuiContext
│   ├── 有界 Tokio mpsc 命令队列
│   └── oneshot 请求响应
├── McpContext
│   ├── 对 MCP 暴露协议无关的操作接口
│   └── 将操作转发给 GuiContext 或 QueryService
└── InfrastructureContext
    └── 持有基础设施查询 ports 和 QueryService
```

当前 `GuiContext` 命令队列容量为 256，单次请求超时为 10 秒。`McpContext::list_profiles` 通过 `QueryService` 查询基础设施，不进入 GUI 命令队列；会话、终端和 SFTP 操作则通过 `GuiContext` 进入 Workspace。

### 主要数据流

#### GUI 用户操作

```text
用户输入
  → GUI UI
  → Workspace / SSH / SFTP Entity
  → SSH、SFTP 或本地状态
  → GPUI 刷新界面
```

GUI 自己的输入和展示仍在 Workspace 内部闭环，保证输入实时性；`DataContext` 不介入普通 GUI 操作。

#### MCP 请求

```text
MCP Agent
  → Streamable HTTP /mcp
  → AgentTerminalMcp tools
  → McpContext
  ├── QueryService → Infrastructure → SQLite
  └── GuiContext → bounded mpsc → Workspace::start_data_context
                              → SSH / SFTP Entity
                              → oneshot 最终结果
```

MCP 工具当前采用单次请求、单次最终结果模型。HTTP 层可以使用 Streamable HTTP，但工具不会返回增量结果流。SFTP 上传和下载会先排队并返回传输信息，后续通过 `list_sftp_transfers` 查询状态、进度和错误。

### 初始化与生命周期

```text
main
  → 初始化日志、HTTP Client、GPUI 和全局 Storage
  → 创建 HomeView
  → 创建 Workspace
  → infrastructure::agent_mcp::start
      → 创建 InfrastructureContext
      → 创建 DataContext 和 GuiContextReceiver
      → 启动 MCP Controller / Streamable HTTP Server
  → Workspace::start_data_context 消费 GUI 命令
```

MCP 服务由 `AgentMcpController` 管理启停和配置；Workspace 持有 GUI 侧 receiver，并在 GPUI 生命周期内消费来自 MCP 的命令。MCP 服务关闭或 Workspace 不可用时，命令通过超时、取消或错误返回结束。
