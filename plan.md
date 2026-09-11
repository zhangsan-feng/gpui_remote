# MCP 并发路由与会话级 Channel 重构实施计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 消除 MCP bridge 当前的全局串行等待，使不同远程会话可以并发执行，同时保证同一远程会话内的操作顺序、响应关联和生命周期安全。

**Architecture:** MCP HTTP transport 继续只持有线程安全的 `McpBridgeEndpoint`。`InfrastructureContext` 内部维护 MCP runtime；runtime 通过一个全局入口接收命令，再由 `McpCommandRouter` 按 `workspace_id` 分发到独立的会话 worker。每个会话 worker 顺序处理本会话命令，不同会话 worker 并行处理；无会话命令走独立 control lane。GPUI bridge adapter 的命令处理和应用事件通知拆成独立异步任务，避免长时间 application 操作阻塞通知转发。

**Tech Stack:** Rust、GPUI/gpui-kit、Tokio `mpsc`/`oneshot`/`broadcast`、Axum、rmcp Streamable HTTP、现有 application SSH/SFTP service。

**Spec:** `AGENTS.md`、`project.md` 和本计划。

## Global Constraints

- 不编写测试用例；验证只使用 `cargo fmt -- --check`、`cargo check`、`git diff --check`、日志检查和 GUI/MCP 手工回归。
- GUI 模块职责：`core.rs` 核心功能和数据流向，`ui.rs` 渲染，`external.rs` 外部调用，`mod.rs` 类型定义、子模块、初始化、Render 入口、`start_subscribe`、`init_component_data` 等。
- application 和 infrastructure 模块职责：`core.rs` 核心功能，`external.rs` 外部 API，`mod.rs` 类型定义、子模块声明和初始化。
- 单个文件维护在 600–800 行；超过 800 行时按功能拆成目录，`mod.rs` 只保留声明、导出和初始化。
- 涉及 IO 的异步流程使用 `cx.spawn + tokio`；阻塞存储操作使用 `spawn_blocking`，不得在 GPUI 渲染路径等待网络 IO。
- 服务端只使用 GET 和 POST；本次不引入其他 REST method。
- MCP 不持有 `App`、`AsyncApp`、`ApplicationContext`、`InfrastructureContext`、GUI entity、UI channel 或渲染对象；ApplicationContext 只在 infrastructure 内的 GPUI bridge adapter/worker 中读取和使用。
- GUI 和 application 使用本层可用的 `cx.read_global` 获取 Global；不通过父组件、controller、server 或 tool 构造函数传递上下文。
- `InfrastructureContext` 是 storage、profile query、MCP runtime 的唯一基础设施根句柄；不得恢复 `DataContext`、`GuiContext` 或 `McpContext` facade。
- MCP bridge 使用 correlation ID 和 `oneshot` 返回结果；命令响应不得依赖请求到达顺序。
- 每个任务完成一个可独立检查的变更后提交 Git；不提交用户已有的 `AGENTS.md` 修改。

## 当前代码基线

- `InfrastructureContext` 已经持有 `Storage`、profile query、MCP bridge receiver 和 MCP runtime，`main.rs` 只负责注册 Global 并调用 `start_mcp`。
- `McpBridgeEndpoint` 使用一个全局 `mpsc::channel`；`start_mcp_bridge` 在单个循环中直接 `await dispatch()`，当前所有 MCP 请求因此全局串行。
- SSH application 已经按 `workspace_id` 维护终端 command channel；SFTP application 也按 `workspace_id` 维护 runtime command channel。
- application 的部分 MCP API 仍从全局 selected session 推导目标会话：`list_sftp_local`、`read_terminal`、`send_text`、`send_key`。
- `select_terminal` 会修改 application 的全局 selected session，不能作为并发 MCP 请求之间的隐式上下文。

## 目标数据流

```text
MCP HTTP client A ─┐
MCP HTTP client B ─┼─> McpBridgeEndpoint.clone()
MCP HTTP client C ─┘            │
                                ▼
                    global ingress mpsc
                                │
                    McpCommandRouter
              ┌─────────────────┼─────────────────┐
              ▼                 ▼                 ▼
       control lane       workspace-A lane   workspace-B lane
       profiles/open      A command worker   B command worker
       list commands      Application API   Application API

Application events ─> independent notification forwarder ─> MCP broadcast
```

路由键使用远程业务会话的 `workspace_id`，不是 MCP Streamable HTTP 的 transport session ID。transport session 仍然可以共享同一个 endpoint；只有拥有明确远程会话 ID 的业务命令才进入对应 session lane。

## 文件职责与目标接口

### MCP bridge

- `src/infrastructure/agent_mcp/bridge/mod.rs`：bridge 子模块声明、公共重导出和 `new()` 入口。
- `src/infrastructure/agent_mcp/bridge/types.rs`：`ApplicationCommand`、`ApplicationResponse`、通知 envelope、`CommandEnvelope`、`ResponseEnvelope` 以及 `RouteKey`。
- `src/infrastructure/agent_mcp/bridge/endpoint.rs`：`McpBridgeEndpoint`、`McpBridgeReceiver`、请求发送和各 typed helper；不包含 application dispatch。
- `src/infrastructure/agent_mcp/bridge/router.rs`：`McpCommandRouter`、会话 lane registry、会话 worker 创建/回收和 backpressure。
- `src/infrastructure/agent_mcp/bridge/dispatch.rs`：单个命令到 `ApplicationContext` API 的映射；只被 bridge worker 调用。
- `src/infrastructure/agent_mcp/bridge/adapter.rs`：GPUI bridge adapter 启动入口、命令入口任务和 application notification 转发任务。
- 删除 `src/infrastructure/agent_mcp/bridge.rs`，用上述目录替代，避免 bridge 文件继续接近 800 行。

### application API

- `src/application/external.rs`：把 MCP 依赖 selected session 的 API 改成显式接收 `workspace_id`。
- `src/application/validation.rs`：复用或补充显式 workspace/protocol 校验；MCP 路径不得通过 `selected_id()` 推导目标会话。
- `src/application/core.rs`：只在现有 application service 已能安全并发的前提下调整调用签名，不在此层引入 MCP channel 或 transport 类型。

### MCP tool 与 server

- `src/infrastructure/agent_mcp/tools.rs`：更新 MCP input schema、tool 描述和 endpoint helper 调用；会话操作的 `workspace_id` 改为必填。
- `src/infrastructure/agent_mcp/server.rs`：保持现有 GET/POST Streamable HTTP 配置和 endpoint clone 方式，不把 router 或 application context 放进 MCP handler。
- `src/infrastructure/agent_mcp/core.rs`、`external.rs`：只维护 MCP server controller 和 settings；会话命令路由由 bridge runtime 负责。

### 基础设施与文档

- `src/infrastructure/context.rs`：继续持有 MCP runtime；只增加 router/worker 生命周期所需的初始化和停止入口，不向 GUI/MCP 暴露内部 registry。
- `src/infrastructure/agent_mcp/mod.rs`：更新 bridge 目录导出，保持 `InfrastructureContext` 对 MCP runtime 的唯一拥有关系。
- `src/main.rs`：不直接创建 channel、router 或 worker；只保持 Global 注册顺序和 `InfrastructureContext::start_mcp` 调用。
- `project.md`：同步新的 command router、session lane 和 notification lane 数据流。
- `plan.md`：按任务完成情况勾选当前计划，不恢复旧的迁移阶段记录。

---

### Task 1：拆分 bridge 文件并保留现有行为

**Files:**

- Create: `src/infrastructure/agent_mcp/bridge/mod.rs`
- Create: `src/infrastructure/agent_mcp/bridge/types.rs`
- Create: `src/infrastructure/agent_mcp/bridge/endpoint.rs`
- Create: `src/infrastructure/agent_mcp/bridge/dispatch.rs`
- Create: `src/infrastructure/agent_mcp/bridge/adapter.rs`
- Modify: `src/infrastructure/agent_mcp/mod.rs`
- Delete: `src/infrastructure/agent_mcp/bridge.rs`

**Interfaces:**

- `bridge::new() -> (McpBridgeEndpoint, McpBridgeReceiver)` 保持调用语义不变。
- `McpBridgeEndpoint::request` 和现有 typed helper 保持返回类型不变。
- `bridge::start_mcp_bridge(cx: &mut App, bridge: McpBridgeReceiver)` 继续作为 `InfrastructureContext::start_mcp` 使用的入口。
- `dispatch(application, command)` 的响应映射保持现有 `ApplicationResponse` 类型和 correlation ID 行为。

- [x] 将现有 envelope、命令、响应、通知类型移入 `types.rs`，保持字段名和可见性。
- [x] 将 endpoint 的发送和 typed helper 移入 `endpoint.rs`，只保留 command channel 操作。
- [x] 将 `dispatch` 移入 `dispatch.rs`，不改变 application API 调用顺序。
- [x] 将 GPUI adapter 移入 `adapter.rs`，暂时保留单 worker 行为作为拆分后的基准。
- [x] 更新所有 `use super::bridge::*` 和 `use crate::infrastructure::agent_mcp::bridge::*` 路径。
- [x] 运行 `cargo fmt -- --check`、`cargo check`、`git diff --check`，确认拆分没有行为变化。
- [x] 提交：`refactor: split mcp bridge responsibilities`。

### Task 2：增加命令耗时和排队观测

**Files:**

- Modify: `src/infrastructure/agent_mcp/bridge/types.rs`
- Modify: `src/infrastructure/agent_mcp/bridge/endpoint.rs`
- Modify: `src/infrastructure/agent_mcp/bridge/adapter.rs`
- Modify: `src/infrastructure/agent_mcp/bridge/dispatch.rs`

**Interfaces:**

- `CommandEnvelope` 增加内部 `queued_at: Instant`，不进入 MCP JSON。
- 增加 `ApplicationCommand::name(&self) -> &'static str`，用于统一日志字段。
- 日志字段固定包含 `request_id`、`command`、`workspace_id`（无会话命令记录 `control`）、`queue_ms`、`application_ms`、`total_ms`。

- [x] 在 `McpBridgeEndpoint::request` 创建 envelope 时记录 `Instant::now()`。
- [x] 在 adapter 或 router 收到命令时记录全局入口排队时间。
- [x] 在 application dispatch 前后记录 application 执行时间和总耗时。
- [x] 让响应发送失败只记录 debug 日志，不影响其他 worker。
- [x] 对 channel 满载、receiver 关闭、application 返回错误分别记录 warn/error，并带上 request ID。
- [x] 运行一次 `cargo check`，确认日志计时不持有任何 GPUI 上下文到 MCP handler。
- [x] 提交：`chore: instrument mcp command latency`。

### Task 3：把 MCP 会话操作改成显式 workspace_id

**Files:**

- Modify: `src/application/external.rs`
- Modify: `src/application/validation.rs`
- Modify: `src/infrastructure/agent_mcp/bridge/types.rs`
- Modify: `src/infrastructure/agent_mcp/bridge/endpoint.rs`
- Modify: `src/infrastructure/agent_mcp/bridge/dispatch.rs`
- Modify: `src/infrastructure/agent_mcp/tools.rs`

**Interfaces:**

- `ApplicationContext::list_sftp_local(&self, workspace_id: String) -> ApplicationResult<SftpDirectorySummary>`。
- `ApplicationContext::read_terminal(&self, workspace_id: String, offset: usize, limit: usize) -> ApplicationResult<TerminalReadPage>`。
- `ApplicationContext::send_text(&self, workspace_id: String, text: String) -> ApplicationResult<()>`。
- `ApplicationContext::send_key(&self, workspace_id: String, key: String, control: bool, alt: bool, shift: bool) -> ApplicationResult<()>`。
- `ApplicationCommand::ListSftpLocal` 携带 `workspace_id: String`。
- `ApplicationCommand::ReadTerminal`、`SendText`、`SendKey` 的 workspace 字段改为 `String`，不再使用 `Option<String>`。
- `ReadTerminalInput`、`SendTextInput`、`SendKeyInput` 的 `workspace_id` 改为必填字段。

- [x] 在 application API 内直接校验 workspace 存在、协议正确，再调用 SSH/SFTP service。
- [x] 删除 MCP 路径对 `selected_sftp_workspace` 和 `resolve_terminal_id` 的依赖；MCP 使用显式 workspace 校验，GUI selected 状态仍由 application 自己维护。
- [x] 删除 MCP 的 `select_terminal` tool、`SelectTerminalInput`、`ApplicationCommand::SelectTerminal` 和对应 dispatch/API，因为 MCP 不应操作 GUI selected 状态。
- [x] 更新 tool 描述，明确所有终端和 SFTP workspace 操作必须使用 `workspace_id`。
- [x] 保持 `list_sftp_sessions`、`list_terminals`、`list_profiles` 等全局查询不需要 workspace ID。
- [x] 运行 `rg` 确认 MCP tools 不再发送 `Option<String>` 会话 ID或调用 GUI selection API。
- [x] 运行格式、编译和 diff 检查。
- [x] 提交：`refactor: make mcp workspace routing explicit`。

### Task 4：实现全局入口与 per-workspace command router

**Files:**

- Create: `src/infrastructure/agent_mcp/bridge/router.rs`
- Modify: `src/infrastructure/agent_mcp/bridge/types.rs`
- Modify: `src/infrastructure/agent_mcp/bridge/adapter.rs`
- Modify: `src/infrastructure/agent_mcp/bridge/dispatch.rs`

**Interfaces:**

- `enum RouteKey { Control, Workspace(String) }`。
- `fn route_key(command: &ApplicationCommand) -> RouteKey`：所有带 workspace ID 的命令返回 `Workspace(id)`，profile/open/list 命令返回 `Control`。
- `struct McpCommandRouter`：持有 `ApplicationContext`、control lane 和按 workspace ID 的 session lane registry。
- `McpCommandRouter::new(application: ApplicationContext) -> Self`。
- `McpCommandRouter::route(&mut self, command: CommandEnvelope) -> impl Future<Output = Result<(), String>>`：只负责入队，不在入口任务中等待 application dispatch。
- `McpCommandRouter::remove(&mut self, workspace_id: &str)`：关闭或外部会话关闭后回收 lane。
- 每个 worker 使用 `mpsc::Receiver<CommandEnvelope>`，逐个调用 `dispatch` 并通过 envelope 自带的 `oneshot::Sender<ResponseEnvelope>` 返回结果。

- [x] 为 control lane 建立独立 bounded channel，处理 profile、open、全局 session/terminal list 等命令。
- [x] 首次收到 workspace 命令时懒创建该 workspace 的 bounded channel 和 worker；registry 只保护 map，不跨 await 持有锁。
- [x] session worker 内部严格按接收顺序调用 `dispatch`，保证同一 workspace 的输入、目录切换和关闭顺序。
- [x] workspace A 和 workspace B 的 worker 使用独立 Tokio task，互不等待对方的 dispatch。
- [x] `CloseSession` 入队后标记 workspace 为 closing，关闭响应完成后 worker 退出并回收 lane；关闭期间的后续命令返回明确错误。
- [x] 为 ingress、control lane、session lane 定义 bounded capacity；满载时在有限时间内返回“会话命令排队超时/队列已满”，不得无限等待。
- [x] 保持 request ID 校验和 response envelope 结构不变。
- [x] 通过 router 的独立 worker 与耗时日志确认一个慢 workspace 不会在 bridge 入口阻塞另一个 workspace。
- [x] 提交：`refactor: route mcp commands by workspace`。

### Task 5：拆分 command adapter 与 notification forwarder

**Files:**

- Modify: `src/infrastructure/agent_mcp/bridge/adapter.rs`
- Modify: `src/infrastructure/agent_mcp/bridge/router.rs`
- Modify: `src/infrastructure/context.rs`
- Modify: `src/infrastructure/agent_mcp/external.rs`

**Interfaces:**

- `start_mcp_bridge(cx: &mut App, bridge: McpBridgeReceiver)` 启动两个独立的 GPUI async task：command router task 和 notification forwarder task。
- command router task 读取 `ApplicationContext` Global 后创建 `McpCommandRouter`，不把 GPUI `App` 或 `AsyncApp` 捕获到 MCP endpoint/server。
- notification forwarder task 独立订阅 `ApplicationContext::subscribe()`，只将 application event 映射成 `NotificationEnvelope` 并发送到 broadcast channel。
- `InfrastructureContext::start_mcp` 仍是唯一启动入口，负责 receiver 一次性消费和 MCP server controller 启动。

- [x] 将当前 `tokio::select!` 中的 command dispatch 和 application event receive 拆成两个任务，避免 application 命令执行期间停止通知转发。
- [x] command router task 退出时记录 ingress 关闭、lane 数量和退出原因。
- [x] notification forwarder 遇到 lagged 时保留当前 warn 日志，遇到 closed 时退出并记录原因，不重启 command router。
- [ ] 确认 MCP server 仍然只持有 `McpBridgeEndpoint` clone，server/tool 文件不引用 router、ApplicationContext 或 InfrastructureContext。
- [ ] 运行 `rg` 检查 MCP server/tool 源码没有 GPUI 上下文和 GUI entity 字段。
- [ ] 运行格式、编译和 diff 检查。
- [ ] 提交：`refactor: isolate mcp command and notification tasks`。

### Task 6：完善会话 lane 生命周期、背压和取消语义

**Files:**

- Modify: `src/infrastructure/agent_mcp/bridge/router.rs`
- Modify: `src/infrastructure/agent_mcp/bridge/adapter.rs`
- Modify: `src/application/session.rs`
- Modify: `src/infrastructure/agent_mcp/bridge/dispatch.rs`

**Interfaces:**

- `McpCommandRouter` 为每个 lane 保存发送端和 worker 状态，不保存 UI 对象。
- `SessionClosed` application event 能触发 registry 删除；已经取出的命令按 response channel 返回“会话已关闭”。
- 队列发送使用明确的 bounded backpressure；响应 receiver 被 MCP 客户端取消时，worker 不 panic，继续处理其他请求。

- [ ] 处理 GUI 关闭会话、MCP 关闭会话和 SSH/SFTP runtime 异常三种 lane 回收路径。
- [ ] 防止 close 与同一 workspace 后续命令乱序：close 进入该 workspace lane，lane 在 close 响应完成后停止接收并从 registry 删除。
- [ ] 为长期占用的 lane 增加空闲清理策略，空闲清理只回收 registry 和 channel，不关闭仍由 application 持有的 SSH/SFTP runtime。
- [ ] 对 `mpsc::Sender::send`、`oneshot::Sender::send`、worker receiver 关闭分别返回可识别的 application result 或 MCP error。
- [ ] 保持 transfer 操作的现有语义：上传/下载立即返回 transfer summary，后续通过 list transfers 查询状态，不让 MCP worker等待完整传输结束。
- [ ] 检查 application session event 通知不会再次调用 GUI。
- [ ] 提交：`refactor: harden mcp session lane lifecycle`。

### Task 7：按日志结果优化同一会话内的操作 lane

**Files:**

- Modify: `src/infrastructure/agent_mcp/bridge/router.rs`
- Modify: `src/infrastructure/agent_mcp/bridge/types.rs`
- Modify: `src/application/ssh/core/service.rs`
- Modify: `src/application/sftp/core/service.rs`
- Modify: `src/infrastructure/agent_mcp/tools.rs`

**Interfaces:**

- 默认保留一个 workspace serial lane，确保状态型命令有序。
- 只有延迟日志证明同一 workspace 的读操作被不必要阻塞时，才增加：`ControlLane`、`ReadLane`、`TransferLane` 三类 lane。
- Transfer lane 只提交 upload/download 请求并立即返回 summary；Read lane 只读取已发布快照；Control lane 负责目录变更、watch、terminal input 和 close。

- [ ] 使用 Task 2 的 `queue_ms` 和 `application_ms` 日志区分 bridge 排队、application 排队、远端 IO 和快照读取延迟。
- [ ] 如果同一 workspace 的慢操作来自 SFTP remote handler，优先复用现有 SFTP runtime command channel 和快照，不在 MCP 层重复建立网络连接。
- [ ] 如果同一 workspace 内需要读写并发，先定义命令之间的顺序规则，再扩展 `RouteKey`，不能仅按命令类型无序 spawn。
- [ ] 保证 terminal 输入不会绕过 SSH runtime 的 per-session command channel。
- [ ] 保证 SFTP transfer 进度仍通过 application snapshot/event 对外提供，不将高频进度塞进 MCP request response。
- [ ] 仅在明确存在收益时提交该优化；若单 workspace serial lane 已满足延迟目标，记录测量结果并保持简单实现。
- [ ] 提交：`perf: separate mcp workspace operation lanes`。

### Task 8：文档、手工回归与最终验收

**Files:**

- Modify: `project.md`
- Modify: `plan.md`
- Inspect: `src/main.rs`
- Inspect: `src/infrastructure/context.rs`
- Inspect: `src/infrastructure/agent_mcp/bridge/`

**Interfaces:**

- `project.md` 的架构图必须体现 `InfrastructureContext -> MCP runtime -> router -> workspace lanes`。
- `plan.md` 勾选项必须与源码和验证结果一致。

- [ ] 更新 `project.md` 的 bridge、router、session worker、notification forwarder 文件职责。
- [ ] 更新 `project.md` 的数据流，明确 MCP 不操作 GUI，GUI selected 状态不作为 MCP 隐式上下文。
- [ ] 运行 `cargo fmt -- --check`。
- [ ] 运行 `cargo check`；不运行 `cargo test`，不新增测试文件。
- [ ] 运行 `git diff --check`。
- [ ] 使用 `rg` 确认不存在旧的 `bridge.rs` 单文件入口、MCP `Option<String>` workspace 路径和 MCP GUI selection 调用。
- [ ] 手工验证两个不同 workspace 同时执行终端读取、SFTP 列目录和 transfer 查询时互不阻塞。
- [ ] 手工验证同一 workspace 的 terminal input、目录切换、close 按发送顺序完成。
- [ ] 手工验证一个慢 SFTP 操作期间，另一个 workspace 的 profile 查询和 terminal 操作可以返回。
- [ ] 手工验证 MCP 客户端取消请求、关闭 workspace、GUI 关闭 workspace 后没有 panic 或 worker 泄漏日志。
- [ ] 手工验证 application event 通知在慢命令执行期间仍然能够转发。
- [ ] 完成最后一个阶段提交：`docs: record mcp concurrency architecture`。

## 完成判定

- bridge 不再使用单个 task 顺序 `await dispatch()` 处理所有 MCP 请求。
- 不同 `workspace_id` 的命令由不同 session worker 并发处理。
- 同一 `workspace_id` 的命令保持明确的 FIFO 顺序；close、取消、channel 满载和 runtime 关闭都有可观测日志和明确错误。
- MCP 会话操作全部使用显式 `workspace_id`，不依赖 application 全局 selected session。
- MCP 不再提供操作 GUI selected 状态的 tool。
- application 的 SSH/SFTP 数据仍由各自模块维护，MCP 只经过 application API，不创建第二套 SSH/SFTP 连接。
- `InfrastructureContext` 继续是 MCP runtime 和 storage 的唯一基础设施所有者；`main.rs` 不组装 MCP channel/router/worker。
- notification forwarder 与 command router 独立运行，慢 command 不阻塞 application event 通知。
- 所有文件符合项目模块边界和 600–800 行维护约束，验证通过且没有新增测试用例。
