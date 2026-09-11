# 项目架构

## 架构目标

项目使用 GPUI Global 作为应用级共享入口。`ApplicationContext` 是应用层根句柄，负责会话、SSH、SFTP 和应用事件；`InfrastructureContext` 是基础设施根句柄，负责存储和 MCP runtime。代理连接由 `infrastructure/proxy` 模块提供适配，协议 application 模块按需使用。

当前没有 `DataContext`、`GuiContext` 或 `McpContext` 中间 facade。每一层在自己的 GPUI 上下文中通过 `cx.read_global` 读取需要的 Global，不通过父组件或构造函数层层传递上下文。

核心原则：

- GUI 和 MCP 都只能通过 ApplicationContext API 访问数据面。
- SSH、SFTP 以及未来新增协议的数据和 runtime 仍由各自 application 模块维护。
- MCP 不操作 GUI，不读取 GUI selected 状态，也不持有 GUI entity、channel 或地址。
- InfrastructureContext 收敛 storage repository 和 MCP 服务生命周期；`main.rs` 只负责初始化、注册 Global 和启动入口。
- 发生问题时先查看日志；日志不足时先在调用边界补充 debug 日志，再根据证据修改实现，不凭猜测重构。

## 总体数据流

```text
GPUI App
  ├─ Global<ApplicationContext>
  │    ├─ SessionStore
  │    ├─ SshStore
  │    ├─ SftpStore
  │    └─ ApplicationEvent
  │
  └─ Global<InfrastructureContext>
       ├─ Storage
       ├─ SessionRepository
       └─ AgentMcpRuntime

infrastructure/proxy
  -> SSH/SFTP application 连接时按 profile 配置使用

GUI
  -> cx.read_global<ApplicationContext>()
  -> ApplicationContext API
  -> 对应 application 模块
  -> SSH/SFTP/未来协议 runtime
  -> entity/store 更新并通知 GUI

MCP Client
  -> HTTP GET/POST
  -> rmcp Streamable HTTP server
  -> AgentTerminalMcp tool
  -> McpBridgeEndpoint.clone()
  -> global ingress mpsc
  -> Tokio bridge adapter
  -> McpCommandRouter
       ├─ control lane
       ├─ workspace-A lane
       └─ workspace-B lane
  -> ApplicationContext API
  -> 对应 application 模块
  -> oneshot ResponseEnvelope
  -> MCP 返回结果

ApplicationEvent
  -> 独立 notification forwarder
  -> MCP broadcast notification
```

Application 层是 GUI 和 MCP 共享的唯一业务数据面。MCP 只负责协议输入输出和命令转发，不能绕过 application 层直接访问 SSH/SFTP service；GUI 也不直接持有基础设施内部地址。

## 并发模型

MCP 支持并发 HTTP 请求，但并发粒度是业务 `workspace_id`，不是 MCP transport session ID。当前 rmcp 使用无状态 Streamable HTTP，每个 POST 是一个独立请求。

- 不同 `workspace_id` 使用独立 bounded lane 和 FIFO worker，可以并行执行。
- 同一 `workspace_id` 使用一个 FIFO lane，保证目录切换、终端输入、watch 和 close 的顺序。
- profile、open session、全局列表等没有 workspace 的命令走独立 control lane。
- 每个请求生成 `request_id`，通过 `oneshot` 关联响应，不依赖请求到达顺序。
- lane 满载或 bridge 关闭时返回明确错误，不能无限等待。
- command router 和 notification forwarder 是两个独立 Tokio task，慢命令不会停止事件转发。

bridge 的命令/通知循环使用 `tokio::spawn` 运行在 Tokio runtime 中。GPUI `cx.spawn` 仍用于需要回到 GPUI 上下文更新 entity 的任务；纯 Tokio channel、timer 和网络服务不放到 GPUI 前台 executor 中等待。

## 分层职责

### domain

存放稳定的领域类型，例如会话、协议、终端等。domain 不依赖 GUI、GPUI 和基础设施实现。

### application

提供应用用例、业务状态、会话生命周期、SSH/SFTP 数据面、统一返回模型和 application event。`ApplicationContext::sftp_workspace_snapshot(workspace_id)` 聚合远程目录、本地目录、传输记录和 watcher 摘要；application API 必须显式接收 `workspace_id`，不能从 GUI selected 状态推导 MCP 目标。

### infrastructure

提供 SQLite 存储 repository、代理、MCP HTTP 协议边界和 bridge runtime。MCP bridge adapter 读取 ApplicationContext Global，并把命令转发到 application facade；GUI 不读取 InfrastructureContext。

### gui

负责 GPUI entity、用户交互、订阅、布局和渲染。GUI 在自己的 `cx` 中读取 Global，交互调用 application API；`ui.rs` 只负责渲染，不等待网络 IO。

### global_state

只保存窗口和 UI 生命周期状态，不承载 SSH/SFTP 业务数据。

### component

提供主题、列表、面板、窗口和布局等通用 UI 组件。样式参考 Tailwind CSS，图标优先复用 Lucide 资源。

## 目录与文件职责

### 根目录

| 目录/文件 | 主要功能 |
| --- | --- |
| `src/main.rs` | 初始化日志、资源和 GPUI App，注册 `InfrastructureContext`、`ApplicationContext`，启动 MCP 和 GUI。 |
| `Cargo.toml` / `Cargo.lock` | Rust 依赖和可复现构建配置。 |
| `AGENTS.md` | 项目约束和协作规则。 |
| `plan.md` | 当前全局 code review 的执行计划、完成进度和验证记录。 |
| `project.md` | 当前架构、数据流、目录职责和排查约定。 |
| `scripts/` | 手工回归和压力工具，不属于 GUI 运行时；`mcp_common.go` 提供公共 MCP 客户端，`mcp_ssh.go`、`mcp_sftp.go`、`mcp_concurrent.go` 分别提供单次 SSH、单次 SFTP 和并发场景入口。 |

### `src/application/`

| 文件/目录 | 主要功能 |
| --- | --- |
| `mod.rs` | application 子模块声明、初始化和稳定类型导出。 |
| `core.rs` | `ApplicationContext`、Global 实现和应用根句柄组合。 |
| `state.rs` | `ApplicationStoreGraph` 以及 session/SSH/SFTP 的 GPUI store handle。 |
| `external.rs` | GUI 和 infrastructure bridge 使用的 application command、快照和返回 API。 |
| `model.rs` | GUI/MCP 共用的 profile、终端、目录、传输和 watch 摘要。 |
| `mapping.rs` | entity/store 状态到共用返回模型的映射。 |
| `validation.rs` | workspace、协议和命令参数校验。 |
| `event.rs` | application entity 事件和会话生命周期事件。 |
| `session/` | 会话创建、关闭、选择和摘要。 |
| `ssh/` | SSH store、终端快照、PTY、输入、resize、滚动和通知。 |
| `sftp/` | SFTP store、目录、路径、传输、删除、取消、重试和 watch。`core/service/mod.rs` 负责生命周期和共享状态，`service/directory.rs` 负责目录与 listener，`service/transfer.rs` 负责传输；`remote.rs` 负责远程扫描，`watcher.rs` 负责 watcher runtime。GUI 使用 application 的 `SftpWorkspaceSnapshot`，只保留交互 projection。 |

### `src/infrastructure/`

| 文件/目录 | 主要功能 |
| --- | --- |
| `mod.rs` | 基础设施子模块声明和初始化入口。 |
| `context.rs` | `InfrastructureContext`，统一持有 storage repository 和 MCP runtime。 |
| `storage/` | SQLite session/profile、SFTP 路径和已知主机密钥存储。 |
| `proxy/` | 网络代理和异步双向流。 |
| `agent_mcp/mod.rs` | MCP 子模块声明和启动入口。 |
| `agent_mcp/core.rs` | MCP controller、配置和 server 生命周期。 |
| `agent_mcp/external.rs` | MCP 对基础设施外部暴露的启动和设置入口。 |
| `agent_mcp/server.rs` | MCP HTTP server、认证、GET/POST 路由和请求耗时日志。 |
| `agent_mcp/tools.rs` | MCP tool 参数解析、命令构造和结果序列化。 |
| `agent_mcp/bridge/mod.rs` | bridge 子模块声明、初始化和稳定导出。 |
| `agent_mcp/bridge/types.rs` | command/response/notification envelope、correlation ID 和 route key。 |
| `agent_mcp/bridge/endpoint.rs` | endpoint 入队、响应关联和 typed helper；不持有应用上下文。 |
| `agent_mcp/bridge/router.rs` | control/workspace lane、FIFO worker、背压和 lane 生命周期。 |
| `agent_mcp/bridge/dispatch.rs` | 将单个命令映射到 ApplicationContext API。 |
| `agent_mcp/bridge/adapter.rs` | 读取 ApplicationContext Global，启动 Tokio command router 和 notification forwarder。 |

### `src/gui/`

| 目录 | 主要功能 |
| --- | --- |
| `home/` | 首页、根窗口和 GUI entity 初始化。 |
| `sidebar_session/` | 会话侧栏和 profile 操作。 |
| `title_bar/` | 标题栏、设置和连接表单。 |
| `workspace/` | 工作区 UI 投影、布局和会话生命周期。 |
| `workspace/ssh/` | SSH 终端输入、选区、滚动和快照投影。 |
| `workspace/sftp/` | SFTP 列表、路径、选择、拖拽和传输投影。 |
| `workspace/top_session/` | 顶部会话标签和会话选择。 |

GUI 模块文件职责：

- `core.rs`：核心功能和数据流向。
- `ui.rs`：渲染。
- `external.rs`：外部调用和本层公开入口。
- `mod.rs`：类型定义、子模块、初始化、Render 入口、订阅和 component data 初始化。

application 和 infrastructure 模块文件职责：

- `core.rs`：核心功能。
- `external.rs`：外部 API。
- `mod.rs`：类型定义、子模块声明和初始化。

单个文件维护在 600–800 行；超过 800 行时按功能拆成目录和文件，`mod.rs` 只保留声明、导出和初始化。

## 生命周期与启动顺序

```text
main
  -> 初始化日志
  -> 创建 InfrastructureContext
  -> cx.set_global(InfrastructureContext)
  -> 创建 ApplicationContext
  -> cx.set_global(ApplicationContext)
  -> InfrastructureContext::start_mcp(cx)
       -> bridge adapter 读取 ApplicationContext Global
       -> 启动 command router task
       -> 启动 notification forwarder task
       -> 启动 MCP HTTP server
  -> 创建 GUI HomeView
```

`InfrastructureContext` 内部只允许一次性消费 MCP bridge receiver；MCP server 对外只持有 `McpBridgeEndpoint` 的 clone。会话打开、关闭、runtime 异常和 GUI 生命周期事件都由 application 层协调，并通过 application event 通知订阅方。

## 后续协议扩展

应用层后续会继续扩展 SSH、SFTP 之外的协议。扩展流程如下：

1. 在 `domain` 增加稳定协议类型和必要的领域模型。
2. 在 `src/application/<protocol>/` 增加协议自己的 `core.rs`、`external.rs`、`mod.rs`，由该模块维护连接、快照、传输和错误状态。
3. 在 `ApplicationContext` 增加协议无关的入口或协议能力 API，返回 GUI/MCP 共用的摘要模型。
4. storage/profile query 只增加协议配置的持久化和查询，不把协议 runtime 放入 storage。
5. MCP 通过新的 application API 暴露工具，命令携带显式 `workspace_id`，继续经过 bridge router，不直接创建协议连接。
6. GUI 通过 `cx.read_global<ApplicationContext>()` 使用相同 application API，并在对应 workspace 下增加投影和渲染。

协议特有的数据必须留在对应 application 模块；bridge 只传递命令、响应、通知和摘要，不能演变成第二套数据层。

## 日志与问题定位

日志由 `src/main.rs` 初始化，application 和 infrastructure 默认开启 debug 级别，文件输出到 `logs/YYYY-MM-DD.log`。

MCP 关键日志标记包括：

- `MCP HTTP request started/finished`：HTTP 入口和总耗时。
- `MCP tool handler started/finished`：工具处理耗时。
- `MCP bridge request started/enqueued/response received`：request_id、入队和响应关联。
- `MCP bridge ingress`、`route`、`lane`：路由、workspace 和队列状态。
- `SFTP application`、`SFTP local watcher`：SFTP runtime 和 watcher 生命周期。

GUI 侧出现问题时，先记录复现操作、时间点、workspace_id 和对应日志片段；若现有日志无法判断，再在 HTTP、GUI action、application API、runtime command channel 和状态通知边界补齐 debug 日志，至少包含操作名、目标 ID、状态转移和耗时。

禁止用“应该是某个 channel 阻塞”这类假设直接修改架构。先通过日志区分 HTTP、MCP tool、bridge 入队、lane 排队、application 调用、远端 IO 和 UI 更新各阶段，再针对实际阶段修复。

## 验证约定

本项目是 GUI 项目，不新增 Rust 单元测试或 GUI 测试用例；使用编译、格式检查、日志和手工回归验证整体行为。

MCP 手工回归工具：

三个独立入口支持 MCP profile 读取、SSH/SFTP workspace 链路和并发只读请求；每次运行会把请求结果和失败信息写入 `logs/`，后续 GUI 问题由实际使用记录，再按日志证据迭代。

独立 MCP 场景工具：

```powershell
$env:GO111MODULE='off'
go run scripts\mcp_ssh.go scripts\mcp_common.go -ip <profile-ip>
go run scripts\mcp_sftp.go scripts\mcp_common.go -ip <profile-ip>
go run scripts\mcp_concurrent.go scripts\mcp_common.go -ip <profile-ip> -rounds 4
```

本机 Go 安装需要先设置 `GO111MODULE=off` 才能解析标准库。三个入口都按串行方式完成准备和清理；只有 `mcp_concurrent.go` 在已建立的 SSH/SFTP workspace 上并发执行只读请求。完整接口回归还可使用 `stress_mcp.go` 与 `stress_mcp_types.go`，覆盖 18 个工具、watch/transfer、workspace 并发和 profile 压力读取。SFTP 入口会记录 workspace、首个远程目录、条目数量和轮询次数，便于和应用日志中的“初始远程目录扫描”次数对照。

常规检查：

```powershell
cargo fmt -- --check
cargo check
git diff --check
```
