# 项目架构

## 架构目标

项目采用 GPUI 官方推荐的 `App -> Global -> Entity/Store` 数据流。GPUI `App` 是全局状态的唯一所有者；`ApplicationContext` 和 `InfrastructureContext` 是注册到 GPUI 的应用级 Global 根句柄；SSH、SFTP 和会话状态由各自 application entity/store 持有。

GUI 与 MCP 都只能通过 application 层访问 SSH/SFTP 数据面：

```text
GPUI App
  ├─ Global<ApplicationContext>
  │    ├─ Entity<SessionStore>
  │    ├─ Entity<SshStore>
  │    └─ Entity<SftpStore>
  └─ Global<InfrastructureContext>
       └─ storage / profile query / proxy / MCP runtime

GUI 使用层
  -> 本层 cx.read_global<ApplicationContext>()
  -> Entity<T>.read/update/observe/subscribe
  -> application entity/store
  -> SSH/SFTP 数据面

MCP Tokio task
  -> McpBridgeEndpoint(CommandEnvelope)
  -> GPUI bridge adapter
  -> adapter 本层 cx.read_global<ApplicationContext>()
  -> application entity/store
  -> ResponseEnvelope / NotificationEnvelope
  -> MCP protocol result
```

Global 只保存全局根句柄，不保存终端输出、目录列表或传输进度等高频业务数据。业务状态通过 entity 的 `read/update/notify/observe/subscribe` 流动；IO 由 `cx.spawn + tokio` 执行，完成后回到 GPUI 上下文更新 entity。

MCP 不持有 `App`、`AsyncApp`、`ApplicationContext`、`InfrastructureContext`、GUI entity 或 UI channel。MCP 只持有线程安全的 `McpBridgeEndpoint`，bridge 只传递命令、响应、通知、摘要和 correlation ID，不传递上下文、service 地址或 GUI 对象。

## 分层职责

- `domain`：会话、协议、终端等稳定领域类型，不依赖 GUI 和基础设施。
- `application`：应用级用例、entity/store、共用返回模型、会话生命周期、SSH/SFTP 数据面和 application 事件。
- `infrastructure`：SQLite、profile 查询、代理、运行时适配和 MCP 协议边界。
- `gui`：GPUI view/entity、交互、订阅和渲染；GUI 不持有 SSH/SFTP service 内部地址。
- `global_state`：窗口和 UI 生命周期状态，不承载 application 业务数据。
- `component`：主题、列表、面板、窗口等通用 UI 组件。

application 和 infrastructure 的公共边界遵守：`core.rs` 放核心功能，`external.rs` 放外部 API，`mod.rs` 放类型、子模块声明和初始化。GUI 遵守：`core.rs` 放核心功能和数据流，`ui.rs` 只渲染，`external.rs` 放外部入口，`mod.rs` 放类型、初始化、Render、订阅和 component data。

## 目录与文件职责

- `Cargo.toml` / `Cargo.lock`：依赖和可复现构建配置。
- `src/main.rs`：日志、资源、GPUI App 初始化、两个 Global 注册，以及通过 `InfrastructureContext` 启动 GUI/MCP。
- `src/domain/`：会话、协议和终端领域模型。
- `src/global_state.rs`：UI GlobalState entity 与 UI 事件，不承载业务数据。
- `src/component/`：通用主题、列表、面板、窗口和布局组件。

### `src/application/`

- `mod.rs`：application 子模块声明、初始化和稳定类型导出。
- `core.rs`：`ApplicationContext`、GPUI `Global` 实现和应用根句柄组合。
- `state.rs`：`ApplicationStoreGraph` 以及 session/SSH/SFTP 的 GPUI typed store handle；运行时数据仍由各自 application 模块持有。
- `external.rs`：GUI 和 GPUI bridge adapter 使用的 application command、快照和返回 API。
- `model.rs`：GUI/MCP 共用的 profile、终端、SFTP 目录、传输和 watch 摘要。
- `mapping.rs`：entity/store 状态到共用返回模型的映射。
- `validation.rs`：application 命令参数校验和统一错误转换。
- `event.rs`：application entity 的领域事件和会话生命周期事件。
- `session/`：SessionStore、会话创建/关闭/选择和会话摘要。
- `ssh/`：SSH store、终端快照、PTY、输入、resize、滚动和通知。
- `sftp/`：SFTP store、目录、路径、传输、删除、取消、重试和 watch。

### `src/infrastructure/`

- `mod.rs`：基础设施子模块声明和 `InfrastructureContext` 初始化入口。
- `context.rs`：统一持有 storage、profile query、MCP bridge/runtime，并实现 GPUI `Global`。
- `profile_query.rs`：profile 查询 port 和具体实现适配。
- `storage/`：SQLite session/profile、SFTP 路径和已知主机密钥存储。
- `proxy/`：网络代理与异步双向流。
- `agent_mcp/mod.rs`：MCP 子模块声明和启动入口。
- `agent_mcp/bridge.rs`：命令、响应、通知 envelope 及线程安全 bridge endpoint；GPUI adapter 在此读取 application Global 并执行 application API。
- `agent_mcp/core.rs`：MCP controller 与 bridge 生命周期。
- `agent_mcp/external.rs`：MCP 启动入口，只接收 bridge endpoint 和 MCP 配置。
- `agent_mcp/server.rs`：MCP server 生命周期和协议适配。
- `agent_mcp/tools.rs`：MCP tool 参数解析、命令构造和 application 结果序列化。

### `src/gui/`

- `home/`：首页、根窗口和应用级 GUI entity 初始化。
- `sidebar_session/`：会话侧栏和 profile 操作。
- `title_bar/`：标题栏、设置和连接表单。
- `workspace/`：工作区 UI 投影、布局和会话生命周期。
- `workspace/ssh/`：SSH 终端输入、选区、滚动和快照投影。
- `workspace/sftp/`：SFTP 列表、路径、选择、拖拽和传输投影。
- `workspace/top_session/`：顶部会话标签和会话选择。

GUI 组件在自己的 GPUI `cx` 中读取需要的 Global，不通过父组件构造函数传递 context。UI 使用 entity 快照进行渲染，交互调用 application API，application 完成状态更新后通知 UI 重绘。

## 生命周期

```text
main
  -> 创建并注册 InfrastructureContext（内部初始化 storage / proxy / MCP runtime）
  -> 创建 application entity/store 图
  -> 创建 ApplicationContext
  -> cx.set_global(InfrastructureContext)
  -> cx.set_global(ApplicationContext)
  -> InfrastructureContext 启动 GPUI bridge adapter 和 MCP Tokio task
  -> 创建 GUI
```

会话打开由 application 层统一协调：读取 profile、创建对应 SSH/SFTP entity、注册 SessionStore、生成 workspace/session ID，并发布 application 事件。会话关闭由 application 层停止对应 runtime、传输和 watcher，再移除 session 状态。GUI/MCP 都不自行生成业务 ID，也不直接回滚底层 service。

## 维护约束

- 不编写测试用例；只进行格式、编译、diff、静态引用和手工回归检查。
- 涉及 IO 的代码使用 `cx.spawn + tokio`，阻塞磁盘/SQLite/密钥加载使用 `spawn_blocking`。
- 服务端接口仅使用 GET 和 POST。
- 单文件维护在 600–800 行；超过 800 行按职责拆为目录和功能文件。
- UI 样式参考 Tailwind CSS，图标优先复用项目已有 Lucide 资源。
