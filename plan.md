# GPUI 官方数据流架构重构实施计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 按照 GPUI/Zed 官方的数据流模型重构项目：由 GPUI `App` 持有全局根状态，使用 `Global` 暴露应用级句柄，使用 `Entity<T>` 持有可观察状态和模块间数据流；GUI 与 MCP 都通过 application 层访问 SSH/SFTP，MCP 不持有或操作任何 GPUI 上下文。

**Architecture:** `App` 是唯一的 GPUI 状态所有者。`ApplicationContext` 与 `InfrastructureContext` 作为注册到 `App` 的轻量 Global 根句柄；`InfrastructureContext` 内部组合 storage、profile query、MCP bridge/runtime 等基础设施依赖，application context 只读取并持有其共享句柄。两者都不把 UI 或 GUI 对象向下暴露。GUI 在自己的 `cx` 中读取 Global，再对 application entity 执行 `read/update/observe/subscribe`；异步 IO 由 GPUI 的 `cx.spawn` 启动，完成后回到 entity 更新状态并 `cx.notify()`。MCP 只拥有线程安全的类型化命令/响应 bridge，通过由 `InfrastructureContext` 启动的 GPUI 适配器触发同一套 application API。

**Tech Stack:** Rust、GPUI 0.2.x、gpui-kit 0.6、Tokio、现有 SSH/SFTP application 模块、现有 MCP server。

**Spec:** `AGENTS.md`、`project.md`（本计划第一阶段创建并同步），以及以下官方实现依据：

- [GPUI contexts](https://github.com/zed-industries/zed/blob/main/crates/gpui/docs/contexts.md)：`App`、`Context<T>`、`AsyncApp` 的边界与生命周期。
- [GPUI ownership and data flow](https://github.com/zed-industries/zed/blob/main/crates/gpui/src/_ownership_and_data_flow.rs)：`Entity<T>` 的所有权、读取、更新、观察和通知。
- [Zed glossary](https://github.com/zed-industries/zed/blob/main/docs/src/development/glossary.md)：`Global`、`Entity`、`App`、`AsyncApp` 的职责。
- [Zed AppState](https://github.com/zed-industries/zed/blob/main/crates/workspace/src/workspace.rs)：应用级根状态保存共享句柄，业务状态由 entity/store 组成。
- [Zed Project](https://github.com/zed-industries/zed/blob/main/crates/project/src/project.rs)：复杂数据面由多个领域 store/entity 分担，而不是一个万能 data facade。
- [gpui-kit architecture](https://github.com/longbridge/gpui-kit/blob/main/docs/ARCHITECTURE.md)：状态按行为归属，复杂状态使用 `Entity<State>`，展示层不拥有业务状态。

## 全局约束

- 不编写测试用例；验证只做格式、编译、diff、静态引用检查和 GUI/MCP 手工回归。
- 删除 `DataContext`、`GuiContext`、`McpContext`，不再创建同等职责的 facade 替代物。
- `cx.set_global(ApplicationContext)` 和 `cx.set_global(InfrastructureContext)` 只在 GPUI App 初始化阶段执行一次。
- 每个拥有 GPUI `cx` 的层，谁使用谁在本层 `cx.read_global`；不通过父组件、controller、server 或 tool 的构造函数传递上下文。
- MCP 进程/任务不持有 `App`、`AsyncApp`、`ApplicationContext`、`InfrastructureContext`、GUI entity、UI channel 或渲染对象。
- MCP 可以持有 `McpBridgeEndpoint`；该 endpoint 只包含线程安全的命令发送端和响应接收端，不包含 GPUI 上下文或 service 地址。
- `Global` 只保存全局根句柄；终端输出、目录列表、传输进度等高频数据必须由 application entity/store 持有并通过快照或事件通知。
- 状态变化采用 GPUI 官方模式：更新 entity 状态后调用 `cx.notify()`；跨模块语义事件使用 entity 的 `emit/subscribe`，不要把所有数据塞进一个全局事件枚举。
- IO 统一使用 GPUI `cx.spawn` 配合 Tokio 异步实现；IO 完成后在 GPUI 上下文中更新对应 entity。
- SSH/SFTP 的连接、终端、目录、传输、watch 等数据仍由各自 application 模块负责存储和生命周期管理。
- GUI 模块遵守 `core.rs`（核心功能和数据流）、`ui.rs`（渲染）、`external.rs`（外部入口）、`mod.rs`（类型、子模块、初始化、Render、订阅和 component data）。
- application/infrastructure 模块遵守 `core.rs`（核心功能）、`external.rs`（外部 API）、`mod.rs`（类型、子模块、初始化）。
- 单文件维护在 600–800 行；超过 800 行按职责改为目录并拆分功能文件。
- 服务端接口仅使用 GET/POST；本次重构不引入其他 REST method。
- 保留现有用户对 `AGENTS.md` 的修改，不使用 `git reset --hard` 或 `git checkout --` 覆盖工作区。

---

## 一、目标数据流

### 1. GPUI 内部数据流

```text
GPUI App
  ├─ Global<ApplicationContext>
  │    └─ Entity<SessionStore>
  │    └─ Entity<SshStore>
  │    └─ Entity<SftpStore>
  │    └─ application 级命令/通知端口
  └─ Global<InfrastructureContext>
       └─ storage / profile query / proxy / MCP bridge/runtime 等基础设施句柄

GUI view/action
  └─ 本层 cx.read_global<ApplicationContext>()
       └─ Entity<T>.read(cx)        读取快照
       └─ Entity<T>.update(cx, ...)  发起变更
       └─ cx.observe / cx.subscribe 订阅变化
       └─ cx.spawn + tokio            执行 IO
             └─ Entity<T>.update(cx, ...) 写回状态
                  └─ cx.notify()      驱动 UI 重绘
```

这里的 `ApplicationContext` 是应用根句柄，不是旧 `DataContext` 的万能转发层；每个业务模块的状态和行为由自己的 entity/store 负责。GUI 只读取 application 层公开的快照、命令和事件，不读取 SSH/SFTP service 的内部地址。

### 2. MCP 跨边界数据流

```text
MCP Tokio task
  └─ McpBridgeEndpoint.send(CommandEnvelope)
       └─ GPUI-owned McpBridge adapter
            └─ 本适配器 cx.read_global<ApplicationContext>()
                 └─ ApplicationContext / Entity<T> API
                      └─ SSH/SFTP application data plane
                 └─ ResponseEnvelope / NotificationEnvelope
       └─ McpBridgeEndpoint 收取响应和状态通知
            └─ MCP tool 转换为 MCP protocol result
```

`McpBridge` 是跨线程边界的端口，不是新的数据上下文。它只传输可序列化或明确 `Send + 'static` 的协议数据：命令、结果、错误、状态、快照摘要和 correlation ID。GPUI 适配器是唯一可以同时接触 `cx` 和 bridge 的位置；MCP 任务永远不调用 `cx.read_global`，也不接触 GUI。

### 3. 官方模型与本项目模型的对应关系

| 官方 GPUI/Zed 概念 | 本项目落地 |
| --- | --- |
| `App` 是全局状态所有者 | `gpui_kit::application().run` 创建的 GPUI App |
| `Global` 是 App 级 singleton | `ApplicationContext`、`InfrastructureContext` |
| `Entity<T>` 是受 App 管理的 typed handle | `SessionStore`、`SshStore`、`SftpStore` 及必要的 UI 可观察状态 |
| `read/update/notify` | GUI/application 对状态的读取、命令写入和重绘通知 |
| `observe/subscribe/emit` | 状态变化和会话生命周期事件 |
| `cx.spawn` + async context | SSH/SFTP/storage IO 的启动和完成回写 |
| 外部协议边界 | MCP bridge；不把 `App` 或 `AsyncApp` 带出 GPUI |

---

## 二、目标目录和职责

```text
src/
├─ application/
│  ├─ mod.rs                         # 类型导出、子模块声明、应用初始化入口
│  ├─ core.rs                        # ApplicationContext、Global 实现、根 entity 组合
│  ├─ external.rs                    # GUI 可调用的稳定 application API
│  ├─ model.rs                       # GUI/MCP 共用的返回模型、快照和摘要
│  ├─ mapping.rs                     # 领域状态/entity 状态到返回模型的映射
│  ├─ validation.rs                  # application 命令的参数校验
│  ├─ event.rs                       # application entity 的领域事件定义
│  ├─ session/                       # SessionStore 与会话生命周期
│  ├─ ssh/                           # SSH store、终端状态、PTY、快照和命令
│  └─ sftp/                          # SFTP store、目录、传输、watch、快照和命令
├─ infrastructure/
│  ├─ mod.rs                         # 基础设施子模块和初始化入口
│  ├─ context.rs                     # InfrastructureContext：storage、MCP runtime 与基础设施 Global
│  ├─ profile_query.rs               # profile 查询端口/实现
│  ├─ storage/                       # session/profile 持久化
│  ├─ proxy/                         # 网络代理
│  └─ agent_mcp/
│      ├─ mod.rs                     # MCP 模块声明和初始化
│      ├─ bridge.rs                   # 命令/响应 envelope、bridge endpoint、关联 ID
│      ├─ core.rs                     # MCP controller 和 bridge 生命周期
│      ├─ external.rs                 # MCP 启动入口，仅接收 bridge endpoint/config
│      ├─ server.rs                   # MCP server 生命周期和协议适配
│      └─ tools.rs                    # MCP tool 命令构造与结果转换
├─ gui/
│  ├─ home/                           # 根窗口和 GPUI entity/view 初始化
│  ├─ workspace/                      # workspace entity/view 与 application 投影
│  └─ ...                             # 只负责交互、订阅和渲染
├─ global_state.rs                    # UI GlobalState entity 和 UI 事件；不承载业务数据
└─ main.rs                            # 注册基础设施/application Global 并创建 GUI
```

`src/data_context` 和 `src/infrastructure/data_context` 已删除；profile query 组合位于 `src/infrastructure/context.rs` 与 `src/infrastructure/profile_query.rs`。如果某个目标文件超过 800 行，按 session、ssh、sftp、bridge 等责任拆成目录，`mod.rs` 只保留声明、导出和初始化。

---

## 三、当前进度

- [x] 已完成重构前 git checkpoint：`aae93e7 chore: checkpoint before application architecture refactor`。
- [x] 已有第一版 application 目录及 SSH/SFTP application 模块，可作为数据面迁移起点。
- [x] 已确认不采用 `GUI -> DataContext -> ApplicationContext`、`McpContext` 或 `GuiContext` 架构。
- [x] 已确认不让 MCP 持有 `App`、`AsyncApp`、`ApplicationContext` 或 `InfrastructureContext`。
- [x] 已完成官方 GPUI/Zed/gpui-kit 数据流调研，确认目标是 `Global + Entity + read/update/notify/observe/subscribe`。
- [x] 创建并同步 `project.md`。
- [x] 将基础设施上下文和 profile query 迁移到 `src/infrastructure/context.rs`、`profile_query.rs`，并删除旧兼容模块。
- [x] 将 application 共用模型、映射、校验和查询边界从 `src/data_context` 迁出。
- [x] 建立 `ApplicationStoreGraph`，由 `Entity<SessionStore>`、`Entity<SshStore>`、`Entity<SftpStore>` 作为 App 持有的根句柄，并完成两个 Global 注册。
- [x] 建立不携带 GPUI 上下文的 MCP 类型化 bridge 和 infrastructure 内 GPUI 适配器。
- [x] 将 storage、MCP bridge receiver 和 MCP runtime 的生命周期收敛到 `InfrastructureContext`，`main.rs` 不再直接组装它们。
- [x] 迁移 GUI 读写和订阅链，GUI component 不再保存 application context 字段。
- [ ] 完成 application entity 状态更新/订阅深化、通知过滤和 GUI/MCP 手工回归。

---

## 四、实施任务

### Task 1：建立架构基线文档

**Files:**

- Create: `project.md`
- Modify: `plan.md`

**Interfaces:**

- `project.md` 必须描述最终目录、模块职责、Global/Entity 数据流、GUI/MCP 边界。
- 后续任务以 `project.md` 的目标架构为准，不再引用旧 `DataContext` 设计。

- [x] 根据当前 `rg --files` 结果写出真实目录树，区分已有文件、迁移文件和计划新增文件。
- [x] 在项目架构章节写明：`App` 持有 Global，application entity/store 持有业务状态，GUI 通过本层 `cx.read_global` 进入 application。
- [x] 在 MCP 章节写明：MCP 只持有 `McpBridgeEndpoint`，GPUI adapter 才读取 Global，bridge 不传上下文和地址。
- [x] 在维护约束章节记录不写测试、IO 使用 `cx.spawn + tokio`、GET/POST、600–800 行文件限制。
- [x] 检查文档中不存在 `DataContext` 作为目标架构、MCP 持有 `AsyncApp` 或 GUI 直接操作 service 的描述。

### Task 2：迁移基础设施根上下文

**Files:**

- Create: `src/infrastructure/context.rs`
- Create: `src/infrastructure/profile_query.rs`
- Modify: `src/infrastructure/mod.rs`
- Modify: `src/application/core.rs`
- Delete after references are removed: `src/infrastructure/data_context/mod.rs`, `src/infrastructure/data_context/profile_query.rs`

**Interfaces:**

- Produces `InfrastructureContext`，包含 storage、profile query、proxy/runtime 等基础设施的共享句柄。
- `InfrastructureContext` 实现 GPUI `Global`，可由 `cx.set_global` 注册；clone 只复制轻量共享句柄。
- application 初始化只接收基础设施依赖一次，不把 storage/query 对 GUI 或 MCP 暴露。

- [x] 从旧 infrastructure facade 合并出唯一 `InfrastructureContext` 定义。
- [x] 让 `src/infrastructure/mod.rs` 提供明确的初始化函数，返回已完成依赖组装的 `InfrastructureContext`。
- [x] 把 `ProfileQuery` 的 trait/实现归入 infrastructure，保证 application 只依赖 profile 查询能力而非 repository 具体类型。
- [x] 更新 `ApplicationContext::new` 及所有调用点，消除旧 infrastructure context 引用。
- [x] 添加 `impl Global for InfrastructureContext`，并通过当前 GPUI 版本编译检查。
- [x] 删除重复 infrastructure facade 文件，并用 `rg` 检查没有旧路径引用。

### Task 3：把共享模型和 application 边界归位

**Files:**

- Create: `src/application/model.rs`
- Create: `src/application/mapping.rs`
- Create: `src/application/validation.rs`
- Create: `src/application/external.rs`
- Modify: `src/application/mod.rs`
- Modify: `src/data_context/model.rs`, `mapping.rs`, `validation.rs`, `query.rs` 的所有调用方

**Interfaces:**

- `application::model` 提供现有 `ProfileSummary`、`TerminalSummary`、`TerminalReadPage`、SFTP summary/transfer/watch 等共享返回模型。
- `application::mapping` 将 session/SSH/SFTP entity 快照转换为上述模型。
- `application::validation` 校验 GUI/MCP 共用命令输入并返回统一 `ApplicationResult` 错误。
- application 层不依赖 GUI/MCP 类型。

- [x] 迁移旧模型定义并保持字段名、序列化格式和错误语义不变。
- [x] 将 DTO 到 MCP JSON 的转换留在 infrastructure/agent_mcp，将 DTO 到 UI 投影留在 GUI，避免 application 依赖协议或渲染。
- [x] 将 profile 查询接口暴露为 application 所需的最小输入/输出，不允许 GUI/MCP 直接访问 storage repository。
- [x] 更新 `src/application/mod.rs` 的可见性：GUI/MCP 需要的模型公开，entity/store 内部实现保持 `pub(crate)`。
- [x] 删除 data_context 中已迁移文件，确保旧模块不再是模型入口。

### Task 4：构建官方模式的 application entity/store 图

**Files:**

- Modify or split: `src/application/core.rs`, `src/application/session.rs`, `src/application/event.rs`
- Modify or split: `src/application/ssh/core/*`
- Modify or split: `src/application/sftp/core/*`
- Create: `src/application/session/state.rs`, `src/application/ssh/state.rs`, `src/application/sftp/state.rs`
- Modify: `src/application/mod.rs`

**Interfaces:**

- `ApplicationContext` 是 GPUI Global 的根句柄，至少组合 `Entity<SessionStore>`、`Entity<SshStore>`、`Entity<SftpStore>` 或等价的模块 entity 句柄。
- `SessionStore` 负责打开会话、关闭会话、选择会话及其可观察摘要；`SshStore` 负责终端连接/快照/输入/resize/滚动；`SftpStore` 负责目录/传输/watch/状态快照。
- 每个 store 的状态由该模块内部拥有，外部只能通过 application API、快照和事件访问。
- `ApplicationContext` 暴露的 API 使用已有业务语义，例如 `open_session`、`close_session`、`select_session`、SSH terminal 操作和 SFTP transfer/path/watch 操作；具体签名以现有调用方为基线统一收敛。

- [x] 建立 `ApplicationStoreGraph`，由 GPUI entity 持有 session/SSH/SFTP application store 的 typed handle；保留 Tokio task 需要的线程安全 worker/port，不把 `App` 放进 worker。
- [ ] 为 session、SSH、SFTP 分别定义状态、命令、快照和事件，避免继续扩大一个 `ApplicationEvent` 万能枚举。
- [ ] 将 entity 状态变更集中到 `Entity<T>.update(cx, ...)`，变更完成后调用 `cx.notify()`；语义事件使用 `cx.emit`，订阅端使用 `cx.subscribe` 或 `cx.observe`。
- [ ] 对打开会话建立清晰事务顺序：读取 profile -> application 创建对应 SSH/SFTP entity -> 注册 SessionStore -> application 生成 workspace/session ID -> 发布 `SessionOpened`。
- [ ] 对关闭会话清理对应 SSH/SFTP worker/entity 状态，再从 SessionStore 移除并发布 `SessionClosed`；失败时返回 application 错误，不由 GUI/MCP自行回滚内部 service。
- [ ] 把高频终端输出、SFTP 目录和传输进度保留在各自 store 的快照/分页 API，不通过 GPUI Global 或 GUI channel 转发。
- [ ] 所有 storage、SSH、SFTP IO 从 application action 通过 `cx.spawn` 启动 Tokio future；future 完成后使用 entity handle 回到 GPUI 线程写回状态。
- [ ] 不在 `ApplicationContext` 中重新引入 `GuiContext`、`McpContext`、UI channel 或 controller 地址。

### Task 5：在 GPUI App 中注册 Global 并建立 application 访问入口

**Files:**

- Modify: `src/main.rs`
- Modify: `src/application/core.rs`, `src/application/mod.rs`
- Modify: `src/infrastructure/context.rs`, `src/infrastructure/mod.rs`
- Modify: `src/global_state.rs`（仅保留 UI 状态和 UI 事件）

**Interfaces:**

- App 初始化顺序固定为：创建 `InfrastructureContext`（内部组装 storage/MCP） -> `cx.set_global(InfrastructureContext)` -> 创建 application entity/store 图及 `ApplicationContext` -> `cx.set_global(ApplicationContext)` -> 从 `InfrastructureContext` 启动 bridge adapter/MCP -> 创建 GUI。
- GUI 使用示例统一为本层读取：

```rust
let application = cx.read_global::<ApplicationContext>().clone();
```

- 需要基础设施能力的 GPUI 层使用：

```rust
let infrastructure = cx.read_global::<InfrastructureContext>().clone();
```

- 以上 clone 只在当前调用边界取得轻量句柄；不得把它作为 view/controller/server/tool 的长期上下文字段向下传递。

- [x] 在 `main.rs` 的 GPUI app 初始化阶段完成两个 Global 的唯一注册。
- [x] 将 `GlobalState` 限定为窗口/UI 状态，移除其中的 application data mirror 和旧 data facade 依赖。
- [x] 调整 Home/Workspace 创建函数，使其不接收旧 context 或 application service 地址。
- [x] 检查启动闭包和 `cx.new` 闭包中的 GPUI 上下文生命周期，确保两个 Global 注册早于 MCP/GUI 创建。
- [x] 将 `main.rs` 的 storage/MCP 组装替换为 `InfrastructureContext::new` 与 `InfrastructureContext::start_mcp`，GUI 的 storage/MCP 配置访问统一从 `cx.read_global` 获取基础设施上下文。
- [x] 保留日志初始化，并补充 `application_global_registered`、`infrastructure_global_registered` 等关键启动日志。

### Task 6：建立不携带 GPUI 上下文的 MCP bridge

**Files:**

- Create: `src/infrastructure/agent_mcp/bridge.rs`
- Modify: `src/infrastructure/agent_mcp/mod.rs`, `core.rs`, `external.rs`, `server.rs`, `tools.rs`
- Modify: `src/main.rs`，加入 infrastructure/agent_mcp 内的 GPUI bridge adapter
- Modify: `src/global_state.rs` only if a UI-independent routing event must be declared

**Interfaces:**

- 定义可跨 Tokio/GPUI 边界传输的协议：

```rust
pub struct CommandEnvelope {
    pub request_id: String,
    pub command: ApplicationCommand,
}

pub struct ResponseEnvelope {
    pub request_id: String,
    pub result: ApplicationResult<ApplicationResponse>,
}

pub struct NotificationEnvelope {
    pub event: ApplicationNotification,
}

pub struct McpBridgeEndpoint {
    command_tx: tokio::sync::mpsc::Sender<CommandEnvelope>,
    response_rx: std::sync::Arc<tokio::sync::Mutex<tokio::sync::mpsc::Receiver<ResponseEnvelope>>>,
    notification_rx:
        std::sync::Arc<tokio::sync::Mutex<tokio::sync::mpsc::Receiver<NotificationEnvelope>>>,
}

impl McpBridgeEndpoint {
    pub async fn request(
        &self,
        command: ApplicationCommand,
    ) -> Result<ApplicationResponse, BridgeError>;
    pub async fn next_notification(&self) -> Option<NotificationEnvelope>;
}
```

- `McpBridgeEndpoint` 使用 Tokio `mpsc` 搭配 request ID；`request` 负责发送命令并从响应接收端匹配同 ID。协议字段不得含 `App`、`AsyncApp`、`Entity<T>`、context、service 地址或 UI 对象。
- GPUI adapter 持有 receiver 和自己的 GPUI task；它读取 Global 后执行 `ApplicationCommand`，向 request 的 oneshot/response stream 写入 application 生成的结果。
- MCP `server/tools` 只保存 endpoint、MCP 配置和请求关联状态。

- [x] 定义 command/response/notification 协议，覆盖 profile 查询、会话生命周期、SSH 终端操作、SFTP 目录/传输/watch 操作。
- [x] 为每个请求生成唯一 request ID，并保证成功、application 错误和 bridge 关闭能返回关联错误。
- [x] adapter 退出时会释放命令 receiver 和请求 response sender；等待中的 MCP 请求收到 bridge closed 错误，不会永久等待。
- [x] 在 GPUI 侧启动 adapter task；adapter 是唯一同时读取 `cx.read_global` 和消费 MCP command 的代码。
- [x] 在 MCP 侧删除 `OnceLock<ApplicationContext>`、`OnceLock<AsyncApp>`、旧 context 以及 GUI handle 注册。
- [x] 添加 bridge 关键日志：请求接收、request ID、响应发送、adapter 关闭；日志不打印密码、私钥或完整敏感连接信息。

### Task 7：迁移 GUI 调用链为 Global + Entity 订阅

**Files:**

- Modify: `src/gui/home/mod.rs`
- Modify: `src/gui/workspace/mod.rs`, `src/gui/workspace/core.rs`, `src/gui/workspace/external.rs`, `src/gui/workspace/internal.rs`, `src/gui/workspace/ui.rs`
- Modify: `src/gui/workspace/ssh/mod.rs`, `src/gui/workspace/ssh/core/*`, `src/gui/workspace/ssh/external.rs`, `src/gui/workspace/ssh/internal.rs`, `src/gui/workspace/ssh/ui.rs`
- Modify: `src/gui/workspace/sftp/mod.rs`, `src/gui/workspace/sftp/core/*`, `src/gui/workspace/sftp/external.rs`, `src/gui/workspace/sftp/internal.rs`, `src/gui/workspace/sftp/ui/*`
- Modify: `src/gui/workspace/top_session/mod.rs`, `src/gui/workspace/top_session/core.rs`, `src/gui/workspace/top_session/external.rs`, `src/gui/workspace/top_session/internal.rs`, `src/gui/workspace/top_session/ui.rs`
- Modify: `src/gui/sidebar_session/*`, `src/gui/title_bar/*`

**Interfaces:**

- 各 GUI component 的业务操作在自己的 GPUI `cx` 里读取 `ApplicationContext`，不从父 view 构造函数接收 context。
- UI 只保留 entity handle、订阅句柄和 UI projection；不保存 SSH/SFTP application service 的内部地址。
- Render 通过 `entity.read(cx)` 读取当前快照；交互通过 `entity.update(cx, ...)` 或 ApplicationContext 稳定 API 发起变更。

- [x] 删除 Workspace、SSH、SFTP、TopSession、Sidebar 等结构体上的旧 context 和直接 service 字段。
- [x] 将构造函数改为只接收 GPUI 所需的 window/cx、必要的 ID 和 UI 初始参数；业务 context 在操作点本层读取。
- [ ] 将 `OpenWorkspaceSession(profile)` 处理为 GUI 请求；Workspace 在本层读取 application Global，调用 `open_session`，使用 application 返回的 ID 创建/更新 UI entity。
- [ ] 保留 `WorkspaceSessionOpened` 作为 UI 生命周期通知时，事件内容只使用 application 生成的 ID、profile 摘要等协议数据，不携带上下文或 service 地址。
- [x] SSH UI 通过 application snapshot/notify 取得终端数据，处理输入、resize、滚动和刷新；不再通过旧 GUI data channel 取得终端数据。
- [x] SFTP UI 通过 application snapshot/notify 取得目录/传输/watch 快照，处理路径、上传、下载、删除、取消和重试；不直接访问 SFTP service。
- [ ] 遵守 GUI 文件边界：数据流放 core，渲染放 ui，外部入口放 external，初始化/Render/订阅放 mod。
- [ ] 检查所有 `cx.spawn` future 的回写路径，确保 entity 被销毁时任务能安全结束或丢弃，不在异步任务中直接操作 view。
- [ ] 保持 Tailwind 风格间距/层级和 Lucide 图标资源，不在架构迁移中引入重复 UI 依赖。

### Task 8：迁移 MCP tools 到同一套 application API

**Files:**

- Modify: `src/infrastructure/agent_mcp/server.rs`
- Modify: `src/infrastructure/agent_mcp/tools.rs`
- Modify: `src/infrastructure/agent_mcp/core.rs`, `external.rs`
- Modify: `src/application/external.rs`, `model.rs`, `mapping.rs`（只在缺少共享 API 时）

**Interfaces:**

- MCP tool 的调用链固定为：解析 MCP 参数 -> 构造 `CommandEnvelope` -> bridge 请求 -> 接收 `ResponseEnvelope` -> application model 转 MCP JSON。
- MCP tool 不调用 GUI API，不访问 Global，不持有 application/infrastructure context，不自行生成 workspace/session 的业务 ID。
- 返回结果由 application 生成共享模型；MCP 只负责协议字段和序列化。

- [x] 将 profile 查询、打开/关闭/选择会话改成 bridge command，并按 request ID 关联响应。
- [x] 将终端输入、分页读取和滚动等 tool 映射为 SSH application command；resize 保持 GUI 专用操作。
- [x] 将 SFTP 目录、路径、上传、下载、watch/stop-watch 映射为 SFTP application command；删除/取消/重试继续由 GUI application API 使用。
- [ ] 对异步状态通知定义订阅过滤：按 workspace/session/transfer ID 过滤，不让 MCP 收到无关状态；当前 bridge 已具备 notification channel，过滤策略待下一阶段接入。
- [ ] 将 application `Result` 和共享 summary 转换成稳定 MCP JSON，避免把内部 entity/store 类型序列化出去。
- [x] 记录 MCP 请求、bridge 响应和 application 错误日志；确认日志不包含凭据。

### Task 9：删除旧 facade，收敛模块可见性

**Files:**

- Delete: `src/data_context/*`
- Modify: `src/main.rs`
- Modify: `src/application/mod.rs`, `src/infrastructure/mod.rs`
- Modify all remaining imports found by `rg`

**Interfaces:**

- crate 根模块不再声明 `mod data_context`。
- 全仓库不得出现 `DataContext`、`GuiContext`、`McpContext`、`crate::data_context` 或 `infrastructure::data_context`。
- storage、SSH、SFTP 的内部 service 类型只在所属层可见；外部使用共享模型、entity handle 或 application API。

- [ ] 从 `src/main.rs` 移除 `mod data_context` 及其日志 target 配置，改为 application/infrastructure target。
- [x] 删除 `src/data_context` 中所有旧 facade 文件。
- [x] 删除已迁移的 `src/infrastructure/data_context` 目录。
- [x] 用 `rg -n "DataContext|GuiContext|McpContext|crate::data_context|infrastructure::data_context" src` 清理所有引用。
- [x] 检查 `pub`/`pub(crate)` 可见性，确保没有为了修复编译而重新暴露底层 service 地址。

### Task 10：文档同步、分阶段提交和验证

**Files:**

- Modify: `project.md`, `plan.md`
- No test files are added or expanded.

**Interfaces:**

- 文档完成状态必须与源码一致；`plan.md` 的任务勾选只在对应代码和检查完成后更新。

- [x] 在基础设施和 application entity 图迁移完成后更新 `project.md` 的真实目录和职责。
- [x] 在 Global/bridge/GUI/MCP 迁移完成后更新 `project.md` 的数据流图和边界说明。
- [x] 分阶段提交基础设施/模型、Global、bridge、GUI/MCP 和旧 facade 删除，并保留用户的 `AGENTS.md` 修改。
- [x] 执行 `cargo fmt`。
- [x] 执行 `cargo fmt -- --check`。
- [x] 执行 `cargo check`。
- [x] 执行 `git diff --check`。
- [x] 使用 ripgrep 确认两个 Global 实现、两个 `cx.set_global` 和 GUI/GPUI adapter 的 `cx.read_global` 都存在。
- [x] 使用 ripgrep 确认 MCP 源码没有 `App`、`AsyncApp`、`ApplicationContext`、`InfrastructureContext`、GUI entity 或 UI channel 的持有/传递。
- [ ] 查看启动、bridge、application、SSH、SFTP、MCP 日志，确认请求接收、application 执行、entity 状态更新和响应返回链路完整。
- [ ] GUI 手工回归：打开/关闭/切换会话、终端输入输出、终端滚动、SFTP 列目录、路径切换、上传、下载、删除、取消、重试和 watch。
- [ ] MCP 手工回归：profile 查询、会话生命周期、终端操作、SFTP 操作和异步状态通知；确认 MCP 全程不触发 GUI 调用链。
- [ ] 不新增测试用例；若编译系统已有测试模块，只做必要的模块引用修复，不扩展测试范围。

---

## 五、完成判定

- `ApplicationContext` 和 `InfrastructureContext` 均实现 GPUI `Global`，并在 App 初始化阶段注册一次。
- application 状态由 session/SSH/SFTP entity/store 持有；状态变更通过 `read/update/notify/observe/subscribe` 传播。
- GUI 每个使用层在自己的 `cx` 中读取 Global，不从父层传递上下文，不直连 SSH/SFTP service。
- MCP 不持有 `App`、`AsyncApp`、两个 Global 或任何 GUI 对象，只持有类型化 bridge endpoint。
- MCP、GUI 使用同一套 application command、snapshot、result API；MCP 只做协议适配。
- bridge 只传命令、结果、通知、快照摘要和关联 ID，不传上下文、entity、service 地址或敏感凭据。
- SSH/SFTP 数据仍存储在各自 application 模块，GPUI Global 不承载高频业务数据。
- `src/data_context` 和 `src/infrastructure/data_context` 被删除，旧类型和旧路径引用清零。
- `project.md` 与源码一致，并记录最终模块职责和数据流。
- `cargo fmt -- --check`、`cargo check`、`git diff --check` 通过，GUI/MCP 手工回归没有重大调用链问题。
