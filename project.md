# 项目状态

## 未完成任务

- [完成] MCP 数据流层解耦第一阶段：将 `list_profiles` 只读查询移出 GUI bridge，建立 MCP Query/Command 分流；其余依赖实时 GUI 状态的命令继续通过有序 GUI command channel 执行。
- [进行中] MCP 查询路径增加分层耗时日志：当前已记录 QueryService 查询开始、结束、数量、错误和耗时；HTTP 入口与完整 queue 等待耗时仍需继续补齐。
- [计划] 为 SFTP/SSH 实时状态建立线程安全快照，逐步移出 `Workspace.update()` 读取路径。
- [计划] 将 SSH/SFTP 长耗时 IO 核心从 GUI 组件中继续下沉到应用服务和基础设施层，GUI 只负责输入、状态订阅和展示。

## MCP 数据流层解耦设计（2026-09-10）

### 目标

GUI 只负责用户输入和状态展示；MCP 只负责协议适配；应用数据流层统一接收 GUI/MCP 输入、编排用例、调用基础设施，并通过状态快照或事件刷新 GUI。纯查询不能因为 GUI 主线程或 GUI command channel 繁忙而排队。

### 目标架构

```text
GUI 输入 ───────────────┐
                        ▼
MCP HTTP/RPC ─► MCP Adapter ─► Application Data Flow
                                  ├─ QueryService ─► Application Ports ─► Infrastructure
                                  ├─ CommandService ─► GUI Command Actor
                                  └─ State/Event Store ─► GUI 展示
```

静态依赖方向为适配层依赖应用层接口，基础设施层实现应用层定义的 port；运行时数据可以双向返回，但基础设施不能反向依赖 GUI。数据流层不能持有 GPUI `Context`、`Entity`、`Window` 或 rmcp/axum 类型。

### 职责边界

- `GUI`：采集用户输入、发送应用命令、订阅状态/事件、渲染页面；不直接访问 MCP、SQLite、SSH 或 SFTP。
- `MCP Adapter`：处理 HTTP、Bearer 鉴权、JSON/RPC 参数和结果转换；只调用应用数据流服务。
- `Application Data Flow`：定义 DTO、Query/Command 用例、校验、超时、取消、错误映射和状态发布；不依赖具体 GUI 或基础设施实现。
- `Infrastructure`：实现 SQLite、文件系统、SSH、SFTP 等 port；只返回领域数据或基础设施错误。
- `GUI Command Actor`：处理必须访问 GPUI 实时状态的操作，保持 FIFO 顺序；不会与只读查询共用执行通道。

### 本次实现范围

1. 在 `application::agent_mcp` 增加 profile 查询 port 和 `AgentMcpDataFlow` 门面。
2. 在 `infrastructure::agent_mcp` 增加 SQLite profile 查询适配器，使用 `spawn_blocking` 执行同步 rusqlite 查询。
3. MCP tools 改为依赖 `AgentMcpDataFlow`；`list_profiles` 走 QueryService，其余命令通过 Data Flow 委托给 GUI command actor。
4. GUI bridge 删除 `ListProfiles` 的数据读取职责，只保留 GUI 状态相关 command 处理。
5. 保留当前 GUI command channel 的有界和有序特性；`CHANNEL_CAPACITY=256` 仅作为回滚前的 checkpoint 实验结果，不作为本次性能结论。
6. 增加应用层查询测试、Data Flow 委托测试，并运行现有 MCP 单元测试、`cargo check` 和 `scripts/stress_mcp.go`。

### 第一阶段实测结论（2026-09-10）

- `list_profiles` 已由 `AgentMcpQueryService` 通过 `spawn_blocking` 调用 SQLite profile adapter，不再进入 GUI command channel。
- smoke 测试 `1 请求 / 1 worker / 1 connection`：`1/1` 成功，P50 `4.1ms`。
- 高并发测试 `2000 请求 / 2000 workers / 128 connections`：`2000/2000` 成功，P50 `177.8ms`、P95 `317.2ms`、最大 `348.7ms`。
- 有界并发测试 `2000 请求 / 128 workers / 128 connections`：`2000/2000` 成功，P50 `19.6ms`、P95 `39.1ms`、最大 `84.1ms`。
- 日志确认请求进入 `application::agent_mcp::query`；本轮未发现 GUI bridge queue-full、响应超时或 MCP server 停止错误。高并发延迟仍主要受客户端连接并发、线程调度和 SQLite 同步读取影响，不能仅靠继续扩大 channel 容量解决。

### 数据流与并发规则

- `list_profiles`：MCP Adapter → Data Flow QueryService → profile repository → DTO → MCP Adapter，不经过 `Workspace.update()`。
- `open/select/change/upload/download/send`：MCP Adapter → Data Flow CommandService → 有序 GUI command actor → GUI/基础设施 → 结果。
- 查询服务中的同步 SQLite 调用必须放在 Tokio blocking pool；不得在 GUI executor 或普通 Tokio worker 中长时间持有数据库锁。
- GUI 状态查询在本次范围内仍通过现有 command actor；后续用不可变快照替代直接读取 GUI Entity。
- 所有路径记录 `command_name`、阶段耗时和错误；请求取消后不得向已关闭的 oneshot receiver 强制发送结果。

### 实施计划

- [x] 建立 `AgentMcpProfileQuery` port、`AgentMcpQueryService` 和 `AgentMcpDataFlow`，先写查询与委托测试。
- [x] 将 SQLite `SessionStorageRepository` 包装为 profile query adapter，并在启动时注入 Data Flow。
- [x] 修改 MCP server/tools 依赖 Data Flow，移除 GUI 对 `ListProfiles` 的处理。
- [x] 更新 GUI bridge command enum、初始化与日志，确保 UI 命令顺序和取消行为不变。
- [x] 运行 `cargo test`、`cargo check`、格式检查和脚本压测；对比单请求、128 并发和 2000 请求结果。
- [ ] 根据结果决定是否继续拆 SFTP/SSH 状态快照；不以扩大 channel 容量替代数据路径解耦。

### 验收标准

- MCP 模块不直接引用 GPUI `Context`、`Entity` 或 `Workspace`。
- `list_profiles` 不再进入 GUI command channel，应用进程高并发下 GUI bridge 日志不出现该命令的 queue-full。
- 依赖注入、错误、超时和取消行为有测试覆盖；现有 GUI 不因本次改造改变用户操作顺序。
- `cargo test agent_mcp` 和 `cargo check` 退出码为 0；压测脚本报告成功率、分位延迟和服务端阶段耗时。


## 目录 / 文件职责

- `Cargo.toml`：项目依赖、构建配置和开发构建优化；维护 gpui-kit 迁移后的直接依赖。
- `Cargo.lock`：可复现的依赖解析结果；锁定 GPUI、platform、macros 和 HTTP client 的同批次版本。
- `project.md`：记录未完成任务和目录职责。
- `src/main.rs`：应用入口、日志初始化、合并本地图标与 Kit 资源、HTTP client 注入、主窗口初始化。
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
- `src/infrastructure/storage/`：SQLite 会话存储、已知主机密钥和持久化基础设施。
- `src/application/agent_mcp/query.rs`：MCP 只读查询 port、查询服务和基础设施无关的 DTO 编排。
- `src/application/agent_mcp/data_flow.rs`：统一协调 MCP QueryService 与 GUI CommandService 的应用数据流门面。
- `src/infrastructure/agent_mcp/profile_query.rs`：基于 SQLite session repository 的 profile 查询适配器。

## 第二阶段：DataContext 核心中转层设计（2026-09-10）

### 目标

将数据流向从 `application::agent_mcp` 迁移到顶层 `data_context` 核心层。`DataContext` 统一持有 `McpContext`、`GuiContext` 和 `InfrastructureContext`，对外提供稳定的接口句柄，负责 GUI、MCP 与基础设施之间的命令、查询和结果路由。

### 核心架构

```text
GUI ──► GuiContext API ─┐
                        │
MCP ──► McpContext API ─┼──► DataContext / CoreBus ───► InfrastructureContext
                        │                    │
                        └──── GuiCommandBus ◄─┘
                                      │
                                      ▼
                                GUI Actor / GPUI
```

`DataContext` 是组合根和中转层，不把业务实现全部堆进 `core.rs`。三个 Context 是能力上下文，不互相持有具体实现；它们通过 `CoreBus`、查询服务和明确的接口句柄协作。

- `McpContext`：面向 MCP adapter 的协议无关调用接口，负责把 MCP 用例转交给 DataContext；不包含 rmcp、axum 或 JSON 类型。
- `GuiContext`：面向 GUI adapter 的命令入口、GUI command receiver 和后续事件订阅；不让核心层持有 GPUI `Context`、`Entity` 或 `Window`。
- `InfrastructureContext`：持有 SQLite、SSH、SFTP 等 ports 的实现；不依赖 GUI 或 MCP。
- `CoreBus`：执行路由、超时、取消、背压和日志；纯查询直接调用 Infrastructure，依赖 GUI 实时状态的命令进入 GUI actor FIFO 队列。

### 目录迁移

```text
src/data_context/
├── mod.rs          类型、构造和外部接口导出
├── core.rs         DataContext、CoreBus 和组合根
├── bus.rs          command/query 的通道与请求生命周期
├── mcp.rs          McpContext 对外能力接口
├── gui.rs          GuiContext、GUI receiver 和 GUI 命令适配
├── infrastructure.rs InfrastructureContext 与基础设施 ports
├── command.rs      中立的数据上下文命令
├── query.rs        查询 port 与查询服务
├── model.rs        跨 GUI/MCP/Infrastructure 的中立 DTO
└── event.rs        数据上下文事件类型和订阅扩展点
```

`application::agent_mcp` 中的 bridge、command、query、model 和 data flow 实现迁移到 `data_context`；`infrastructure::agent_mcp` 只保留 MCP HTTP/RPC adapter，profile query adapter 迁移到 `infrastructure::data_context`。

### 外部接口与生命周期

```rust
let (data_context, gui_receiver) = DataContext::new(infrastructure_context);
let mcp_context = data_context.mcp();
```

- Infrastructure 在应用启动组合阶段创建 `InfrastructureContext` 并注入 DataContext。
- MCP server/tools 只持有 `McpContext`，不能访问 DataContext 内部字段。
- GUI Workspace 只持有 `GuiContextReceiver`，负责 GPUI update、用户展示和状态变更。
- DataContext 本体由服务控制器和接口句柄共享的 `Arc` 内核保持生命周期。
- `DataContext` 不依赖具体外部协议或 UI 框架，外部 adapter 只能通过公开接口调用内核。

### 第二阶段实施计划

- [ ] 先提交第一阶段实现和本设计文档作为 DataContext 迁移 checkpoint。
- [ ] 创建 `src/data_context`，迁移中立 model、command、bus、query、McpContext 和 GuiContext。
- [ ] 创建 `src/infrastructure/data_context`，迁移 SQLite profile query adapter，构造 InfrastructureContext。
- [ ] 修改 MCP server/tools 只依赖 `McpContext`，修改 GUI Workspace 只依赖 `GuiContextReceiver`。
- [ ] 删除 `application::agent_mcp` 的实际实现和旧命名，确保核心层不再以 MCP 作为唯一入口。
- [ ] 保持查询绕过 GUI 队列、GUI 命令 FIFO、超时/取消语义和现有日志能力。
- [ ] 运行 `cargo test`、`cargo check`、格式检查和 MCP 脚本压测。

### 第二阶段验收标准

- `DataContext` 明确持有 `McpContext`、`GuiContext`、`InfrastructureContext`，外部只能拿到接口句柄。
- MCP adapter 不再依赖 `AgentMcpClient` 或 `AgentMcpDataFlow`。
- GUI 不再直接创建 MCP 专用 channel，GUI 只消费 `GuiContextReceiver`。
- `data_context` 不引用 GPUI、rmcp、axum 或具体 Infrastructure 实现。
- profile 查询继续绕过 GUI command channel；其他 GUI 状态操作顺序不变。
- 全量测试、编译检查、格式检查和 MCP 压测通过。
