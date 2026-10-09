# gpui_remote 项目说明

## 项目架构

项目采用领域、应用、基础设施、数据上下文和 GUI 分层：

- `domain`：领域模型与协议类型，不依赖上层实现。
- `application`：用例、业务校验、资源生命周期编排；通过全局 `INFRASTRUCTURE` 执行 IO，通过全局 `DATA_CONTEXT` 读取和提交热数据。全局 `APPLICATION` 的 clone 共享同一组运行时资源，不保存下层服务端口。
- `data_context`：内存热数据中心。全局 `DATA_CONTEXT` 保存工作区摘要、终端历史、SFTP 目录、传输和 watcher 快照，提供 Reader/Writer/Notice；三者共享同一份数据，不执行文件或网络 IO。
- `infrastructure`：SQLite、文件系统、SSH/SFTP、代理和 MCP 适配器；阻塞 IO 放入 `spawn_blocking`，异步 IO 经 Tokio 运行时执行。
- `gui`：GPUI 投影和交互。持有全局 Application 的 clone 执行业务操作；持久化配置通过 Application 读写；工作区热数据直接通过 `DATA_CONTEXT.reader()` 读取，通知直接通过 `DATA_CONTEXT.notice()` 订阅。GPUI 仅注册 ApplicationHandle，不注册下层对象。
- `component`：可复用 GPUI 组件、主题、颜色和窗口工具。

### 数据流向

- 持久化配置：`gui/mcp -> application -> infrastructure`。
- 热数据写入：`gui/mcp -> application -> data_context`。
- 热数据读取：`gui/mcp -> data_context`。
- 变化订阅：`gui/mcp -> data_context::DataContextNotice`。
- Application 负责输入合法性、资源是否存在、生命周期顺序和资源释放结果的校验；`data_context` 只负责热状态的一致读写。
- Writer 完成提交后再推进 revision 并发布通知；Notice 对外只有订阅能力。每个窗口/消费者独立订阅，先订阅再读取初始快照；落后时重新读取快照恢复。

### 全局初始化与关闭

`APPLICATION`、`INFRASTRUCTURE`、`DATA_CONTEXT` 各自使用 `LazyLock::new` 创建全局对象；构造只创建内存对象、锁和通道，不执行 IO、不启动任务。Application 构造不访问下层全局，Infrastructure 构造只取得 DataContext 读/通知句柄，不反向访问 Application。

main 先调用 `INFRASTRUCTURE.initialize().await`，通过 `spawn_blocking` 初始化 SQLite，并将成功资源放入 `OnceCell`；初始化失败可以重试，初始化前仓储访问返回错误。然后取得 APPLICATION clone，显式启动 MCP/SSH/SOCKS5 并加载主题，最后进入 GPUI。GUI 的 IO 用例通过 `cx.spawn` 等待 `run_application` 在 Tokio 中执行，完成后在 GUI 上更新投影。

静态对象退出时不会依赖 Drop 清理资源，因此 main 先关闭应用操作入口，等待已接收的 GUI/MCP 用例完成；未开始的后台用例和排队 MCP 命令返回退出错误。随后停止 MCP 监听并等待监听任务结束，完成主题落盘、关闭工作区，再停止 SSH/SOCKS5；停止 MCP 不改写用户的启用设置。

无 IO 的标签选择保留在 GUI 执行器；本地和远端 SFTP 导航在 GUI 收到请求时分别连接完成通知，按提交顺序进入后台，避免 Tokio 多线程调度颠倒连续导航的顺序。

GUI 模块按职责划分为 `core.rs`（核心功能和数据流向）、`ui.rs`（渲染）、`external.rs`（外部交互）、`mod.rs`（类型、子模块、初始化和 Render 入口）。Application 与 Infrastructure 模块遵循 `core.rs`、`external.rs`、`mod.rs` 的职责划分；单个 Rust 文件超过 800 行时按功能拆分为目录。

服务端 HTTP 入口只开放 GET 和 POST。MCP 的快照查询直接读取 `DataContextReader`，命令和写入操作通过 Application bridge 调度。

## 项目目录结构

| 目录/文件 | 主要功能 |
| --- | --- |
| `Cargo.toml` | 包信息、Rust 版本和依赖声明。 |
| `Cargo.lock` | 可复现构建使用的依赖解析结果。 |
| `build.rs` | 构建阶段处理项目构建信息。 |
| `src/main.rs` | 日志初始化、全局服务异步启动/关闭、Application clone 注册和 GPUI 入口。 |
| `src/domain/` | 会话、终端、SFTP 和基础设施状态类型。 |
| `src/application/` | 全局应用上下文、校验、会话生命周期、SSH/SFTP 用例及既有协议资源契约。 |
| `src/data_context/` | 工作区热数据、终端历史、SFTP 快照及版本通知。 |
| `src/infrastructure/` | 存储、文件系统、代理、协议适配器、MCP 服务和运行时设施。 |
| `src/infrastructure/storage/` | SQLite 会话仓储、已知主机和设置持久化。 |
| `src/infrastructure/agent_mcp/` | MCP 认证、桥接、工具、HTTP 路由和通知。 |
| `src/gui/mod.rs` | GUI 子模块声明和 Render 入口。 |
| `src/gui/sidebar_session/` | 脱敏会话摘要列表、选择和交互逻辑。 |
| `src/gui/workspace/` | 工作区布局以及 SSH/SFTP 视图。 |
| `src/component/` | 可复用列表、面板、主题和窗口组件。 |
| `logs/` | 运行时日志输出目录。 |
| `data/` | 应用运行数据目录。 |
| `docs/` | 项目补充文档。 |

## 主要依赖

- `gpui-kit`：GPUI 组件和应用基础能力。
- `alacritty_terminal`：终端解析与终端状态处理。
- `russh`、`russh-sftp`：SSH/SFTP 连接与文件操作。
- `axum`、`tokio`：异步服务端和运行时。
- `rmcp`：MCP 服务端传输和协议支持。
- `rusqlite`：内置 SQLite 存储。
- `serde`、`serde_json`：数据序列化。
