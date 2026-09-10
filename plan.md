# SSH/SFTP 数据面架构重构计划

## 目标

建立并落地以下调用链：

```text
GUI -> DataContext.GuiContext -> ApplicationContext API -> SSH/SFTP application module
MCP -> DataContext.McpContext -> ApplicationContext API -> SSH/SFTP application module
```

`DataContext` 负责中转、会话校验和 DTO 转换；`ApplicationContext` 负责应用级会话生命周期和模块协调；SSH/SFTP 模块负责真实连接、任务、快照、传输和监听；GUI 只保留交互状态和渲染投影。

## 约束

- 不新增单元测试或局部测试用例；这是 GUI 架构重构，采用编译检查、静态调用链检查、日志和实际 GUI 回归验证。
- GUI 异步操作从 `cx.spawn` 进入 DataContext；数据面内部使用 Tokio，阻塞 IO 使用 `spawn_blocking`。
- `gui` 不直接访问 `ApplicationContext`、SSH/SFTP 模块、Storage 数据面或文件监听器。
- 单文件控制在 600–800 行，超过 800 行按目录和职责拆分。
- 服务端保留 DDD 方向，HTTP 只使用 GET/POST。
- 问题定位优先查看日志，连接、目录、传输和监听生命周期保留上下文日志。

## 已完成进度

- [x] 已完成重构前 Git 检查点提交：`aae93e7 chore: checkpoint before application architecture refactor`。
- [x] 新增 `src/application/`，建立 `ApplicationContext`、会话生命周期、SSH 应用模块和 SFTP 应用模块。
- [x] SSH runtime、PTY、ANSI 缓冲、输入、尺寸、滚动、读取和状态通知已迁移到 `src/application/ssh/`。
- [x] SFTP 连接、远程目录、上传、下载、删除、取消、重试、本地扫描和目录监听已迁移到 `src/application/sftp/`。
- [x] SFTP 本地路径和远程路径的持久化由 `ApplicationContext` 通过 Storage port 使用 `spawn_blocking` 处理。
- [x] `DataContext` 已持有共享 `ApplicationContext`；`GuiContext` 与 `McpContext` 共用同一应用句柄。
- [x] 删除旧的 GUI MCP 命令队列和 GUI Entity 执行桥接。
- [x] `TerminalView` 改为终端快照投影；`SftpView` 改为远程、本地、传输和监听摘要投影。
- [x] GUI 的 SSH/SFTP 输入、目录、传输、删除和监听操作均经 `GuiContext` API 转发。
- [x] 删除 GUI 侧旧 SSH/SFTP 数据面文件，避免旧 runtime、连接和文件监听链路被误用。
- [x] `agent_mcp::start` 使用 `OnceLock` 复用同一 `DataContext/ApplicationContext`。
- [x] 已完成 `project.md` 和本计划文档的架构说明同步。

## 当前实现结构

### ApplicationContext

- `src/application/core.rs`：组装基础设施、会话、SSH、SFTP；提供打开、关闭、选择会话和路径持久化 API。
- `src/application/session.rs`：会话元数据和选择状态。
- `src/application/event.rs`：应用会话事件类型。
- `src/application/ssh/core/service.rs`：SSH runtime 管理和终端 API。
- `src/application/ssh/core/pty.rs`、`ssh.rs`、`buffer.rs`、`key.rs`：PTY 生命周期、连接读写、终端缓冲和按键编码。
- `src/application/sftp/core/service.rs`：SFTP runtime、目录、传输、删除、重试和监听编排。
- `src/application/sftp/core/conn.rs`、`remote.rs`、`local.rs`、`watcher.rs`、`path.rs`：SFTP 连接、远程/本地 IO、监听和路径工具。

### DataContext

- `src/data_context/core.rs`：组合 `ApplicationContext`、`GuiContext`、`McpContext` 和 InfrastructureContext。
- `src/data_context/gui.rs`：GUI facade，负责会话校验、应用 API 调用和 DTO 转换。
- `src/data_context/mcp.rs`：MCP facade，与 GUI 使用同一个应用句柄。
- `src/data_context/model.rs`：只读 DTO，不暴露应用内部模型。
- `src/data_context/infrastructure.rs`、`query.rs`：基础设施 port 和查询服务。

### GUI

- `src/gui/workspace/mod.rs`：创建一次 DataContext，并把同一个 `GuiContext` 注入 Workspace、TerminalView、SftpView。
- `src/gui/workspace/external.rs`：将 GUI 会话事件转成 DataContext 会话 API 调用。
- `src/gui/workspace/ssh/`：终端渲染、键盘、选区、滚动和快照投影。
- `src/gui/workspace/sftp/`：列表、拖拽、路径弹窗、传输菜单和快照投影。

## 验证记录

- 基线 `cargo test` 已在重构前检查点通过：10 passed，0 failed；本次不新增测试用例。
- 重构过程中 `cargo check` 已通过。
- 已完成静态检查：`src/gui` 不再引用 `russh`、`russh_sftp`、`notify`、`spawn_blocking` 或文件系统扫描实现。
- 待提交前执行：`cargo fmt`、`cargo fmt -- --check`、`cargo check`、`git diff --check`。
- 待应用启动后进行一次手工 GUI 回归：SSH 打开/输入/滚动/关闭；SFTP 打开/切换目录/上传/下载/删除/取消/重试/监听；MCP 与 GUI 同时操作同一会话。

## 后续收尾

- [x] 已执行 `cargo fmt`、`cargo fmt -- --check`、`cargo check` 和 `git diff --check`；剩余 warning 均来自既有 UI/API 未使用代码。
- [ ] 检查打开/关闭/选择会话的竞态和异常日志，特别是连接建立期间快速关闭会话。
- [ ] 启动最新构建做一次 GUI 手工回归，确认真实 SSH/SFTP 连接、投影刷新和任务回收。
- [ ] 审核变更范围，只提交本次架构重构相关文件，提交最终重构结果。
