# 项目状态

## 未完成任务

- [进行中] MCP 数据流层解耦：先将 `list_profiles` 只读查询移出 GUI bridge，建立 MCP Query/Command 分流；其余依赖实时 GUI 状态的命令继续通过有序 GUI command channel 执行。
- [计划] MCP 查询路径增加分层耗时日志，区分 HTTP、查询执行和 GUI command queue 等待时间。
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

### 数据流与并发规则

- `list_profiles`：MCP Adapter → Data Flow QueryService → profile repository → DTO → MCP Adapter，不经过 `Workspace.update()`。
- `open/select/change/upload/download/send`：MCP Adapter → Data Flow CommandService → 有序 GUI command actor → GUI/基础设施 → 结果。
- 查询服务中的同步 SQLite 调用必须放在 Tokio blocking pool；不得在 GUI executor 或普通 Tokio worker 中长时间持有数据库锁。
- GUI 状态查询在本次范围内仍通过现有 command actor；后续用不可变快照替代直接读取 GUI Entity。
- 所有路径记录 `command_name`、阶段耗时和错误；请求取消后不得向已关闭的 oneshot receiver 强制发送结果。

### 实施计划

- [ ] 建立 `AgentMcpProfileQuery` port、`AgentMcpQueryService` 和 `AgentMcpDataFlow`，先写查询与委托测试。
- [ ] 将 SQLite `SessionStorageRepository` 包装为 profile query adapter，并在启动时注入 Data Flow。
- [ ] 修改 MCP server/tools 依赖 Data Flow，移除 GUI 对 `ListProfiles` 的处理。
- [ ] 更新 GUI bridge command enum、初始化与日志，确保 UI 命令顺序和取消行为不变。
- [ ] 运行 `cargo test agent_mcp`、`cargo check` 和脚本压测；对比单请求、128 并发和 2000 请求结果。
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
