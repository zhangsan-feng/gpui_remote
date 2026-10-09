# DataContext 与 Infrastructure 模块收敛计划（2026-10-08）

## 目标

按 Application 最近完成的模块收敛方式整理 `src/data_context` 和 `src/infrastructure`：两个根目录只保留 `mod.rs`；根 `mod.rs` 声明子模块、定义全局上下文和构造入口，具体数据能力和基础设施操作放入各自功能模块。保持现有调用语义、数据流和服务启动/关闭顺序，不在本计划中增加功能。

DataContext 继续只管理进程内热数据，不执行文件、网络或数据库 IO。Infrastructure 继续提供底层能力；Application 保留业务校验并直接通过 `DATA_CONTEXT` / `INFRASTRUCTURE` 使用对应功能。现有 GUI/MCP 热数据读取、写入与通知契约保持不变。

## 项目约束

- `src/data_context` 和 `src/infrastructure` 根目录最终只保留 `mod.rs`。
- 类型、模块声明、全局对象和纯内存构造归根 `mod.rs`；具体功能实现归相应子模块，避免把多种能力重新堆到新的大文件。
- DataContext 直接持有热数据字段，不保留 `Inner` 或 Reader/Writer 包装器；GUI 刷新通过 `DataContextNotice.gui_refresh_notification`，会话生命周期事件通过 `session_lifecycle_events` 发布，快照版本计数继续保留。
- Infrastructure 构造不执行 IO；SQLite 初始化、MCP 启动/关闭继续显式调用，启动与关闭顺序保持不变。
- 不新增 trait、依赖或测试用例；按项目要求只做格式、编译、行数和静态调用路径检查。
- Rust 文件保持在 800 行以内；新旧文件移动后同步更新模块路径及所有调用方。

## 目标模块归属

### DataContext

- `DataContext`：直接持有 workspace snapshot、terminal/SFTP workspace maps、transfer ID counter、提交锁及独立 `notice`；通过按功能拆分的 `impl DataContext` 方法访问，不设置 Reader/Writer 包装器或 `Inner`。
- `workspace`：工作区打开、关闭、选择、数据库工作区更新和工作区摘要查询。
- `terminal`：终端读取/历史、输出发布、buffer 初始化与访问、终端状态更新；保留现有 `terminal/buffer.rs` 并纳入该模块。
- `sftp`：将根 `sftp.rs` 改为 `sftp/` 目录，收拢 SFTP 状态、扫描代次、watch、传输队列、取消和快照映射；SFTP workspace model 持有 DataContext 静态引用并通过其功能方法访问状态。
- `service_status`：HTTP Proxy、SOCKS5 Proxy、SSH Server、端口转发运行状态写入及其快照提交。
- `notice`：集中持有 `session_lifecycle_events` 与 `gui_refresh_notification` 订阅及发布入口。
- `state`：按 workspace、terminal、SFTP 功能拆分当前根 `model.rs` 中的状态和快照类型；全局 `DataSnapshot`/`DataChange` 保留单一权威定义。

DataContext 根 `mod.rs` 保留 `DATA_CONTEXT`、直接持有状态字段并包含独立 `notice` 的 `DataContext`，以及纯内存构造；不得保留 `Inner`、Reader/Writer 访问句柄或根级 `core.rs`、`external.rs`、`model.rs`、`sftp.rs`。

### Infrastructure

- `lifecycle`：Infrastructure 初始化入口和持久化存储初始化协调；存储访问辅助方法归 `storage`。
- `agent_mcp`：将根 `core.rs` 的 MCP 启动/关闭入口及根 `external.rs` 的 MCP 设置读写/转换入口归入 MCP 模块。
- `connection`：将根 `connection.rs` 改为目录，连接流和当前连接适配实现归入该模块，不增加新的抽象接口。
- `ssh` / `sftp`：分别接管 `open_ssh_shell` / `connect_sftp` 上下文入口。
- `database`：接管数据库连接入口，保持目前未接入驱动时返回的错误语义。
- `storage`：接管 `storage()` 访问和会话配置、SFTP 持久化状态的 `spawn_blocking` 仓储操作。
- `port_test`、`theme`、`http_proxy`：接管 TCP 探测、主题设置/壁纸和 HTTP Proxy controller 访问入口；能力留在已有对应模块。
- 现有 `local_fs`、`port_forward`、`socks5_proxy`、`ssh_server`、`sftp_server` 等模块继续拥有各自基础设施操作，不迁回根层。

Infrastructure 根 `mod.rs` 保留全局 `INFRASTRUCTURE`、上下文/Inner 类型、子模块声明和纯内存构造；不得保留根级 `core.rs`、`external.rs` 或 `connection.rs`。

## 当前完成进度

- [x] 用户确认 DataContext 与 Infrastructure 也按 Application 的模块内聚方向整理，并要求先写入计划。
- [x] 只读盘点 DataContext、Infrastructure 根文件、已有功能模块和主要调用关系。
- [x] 实施 DataContext 子模块拆分和根文件收敛；后续重构已由 Reader/Writer + `Inner` 改为 DataContext 直接持有状态，详见文末 follow-up。
- [x] DataContext 验证：`cargo fmt --all -- --check` 与 `cargo check --locked --offline` 通过；cargo check 有 20 条 dead-code warning，DataContext 无编译警告；旧 Reader/Writer API 检索无结果、DataContext 根目录仅有 `mod.rs`、Rust 文件不超过 800 行、作用范围差异检查通过。独立代码审查未发现可行动问题。没有新增或运行测试。
- [ ] 全量 `git diff --check` 报告 `AGENTS.md` 文件末尾多余空行；排除该约束文件后的差异检查通过，未修改 `AGENTS.md`。
- [x] Infrastructure 根 API 已按能力归入 lifecycle、storage、agent_mcp、connection、database、ssh、sftp、theme、port_test、http_proxy；根目录收敛为 `mod.rs`。
- [x] 调用路径、格式、编译、行数和差异检查完成；DataContext 与 Infrastructure 的独立代码审查均未发现可行动问题。
- [ ] GUI 交互与服务运行时启动/关闭顺序尚未手工联调；静态检查无法覆盖。

## 当前计划

### Task 1：将 DataContext 根级数据能力归入功能模块

- [x] 按数据域拆分 DataContext 实现；最终入口由 `impl DataContext` 功能方法构成，不保留各域的 `reader` / `writer` 包装器。
- [x] 将 workspace、terminal、service status 的 Reader/Writer 方法从根 `external.rs` 拆入相应模块。
- [x] 将根 `sftp.rs` 改为 `sftp/` 目录；按类型、状态操作和快照映射拆分到该模块。
- [x] 将根 `model.rs` 状态类型按 workspace、terminal、SFTP 归位，保留唯一 DataSnapshot 聚合模型。
- [x] 删除 DataContext 根 `core.rs`、`external.rs`、`model.rs`、`sftp.rs`，根目录只留 `mod.rs`。

### Task 2：将 Infrastructure 根操作归入基础设施模块

- [x] 将 `connection.rs` 改为 `connection/` 子模块，并搬迁当前连接实现与类型。
- [x] 将根 `core.rs` 的初始化、storage accessor、MCP 生命周期及 HTTP Proxy controller accessor 分别迁入 `lifecycle`、`storage`、`agent_mcp`、`http_proxy`。
- [x] 将根 `external.rs` 按 MCP、主题、连接、SSH、SFTP、数据库、会话仓储、端口检测能力拆入对应模块的 `external.rs`。
- [x] 对数据库连接入口建立明确的 `database` 归属；对会话配置与 SFTP 持久化操作归入 `storage`。
- [x] 删除 Infrastructure 根 `core.rs`、`external.rs`、`connection.rs`，根目录只留 `mod.rs`。

### Task 3：更新入口和调用路径

- [x] 更新并检索 `main.rs`、Application、GUI、MCP 和 Infrastructure 内部调用到新的模块入口，保留现有全局对象使用方式。
- [x] 保持 GUI/MCP 直接读 DataContext 热数据、通过 Application 写业务数据的路径；不保留横跨数据域的 Reader/Writer 套壳。
- [x] 保持 SQLite、MCP、代理和 Server 初始化/关闭次序、错误传播与日志行为。
- [x] 全仓检索旧根级模块路径和失效 API，确认没有跨层绕过校验或重复存储状态。

### Task 4：静态验证与进度记录

- [x] 确认两个根目录都只剩 `mod.rs`，所有 Rust 文件不超过 800 行。
- [x] 运行 `cargo fmt --all -- --check`、`cargo check --locked --offline` 和本次变更范围的 `git diff --check`；不运行测试。
- [x] 在本节记录验证结果、编译 warning 和无法在静态检查中覆盖的 GUI/运行时手工联调项。

## 后续收敛：`main.rs` 入口职责归位（2026-10-08）

### 目标

`main.rs` 只保留进程入口、GUI 框架启动及其必须的依赖装配。日志初始化、应用服务启动与退出、主题设置后台持久化、图标资源合并、主窗口选项分别由对应模块负责；迁移时保持原有服务启停顺序和错误处理策略。

### 进度

- [x] 日志器配置归入 `infrastructure::logging`。
- [x] Application 初始化/关闭编排和工作区退出清理归入 `application::lifecycle`。
- [x] 主题设置加载、持久化任务启动与停止归入 `application::theme`。
- [x] RustEmbed 图标与组件资源合并归入 `component::assets`。
- [x] 主窗口选项归入 `gui::home`。
- [x] 主窗口创建、全局 GUI 状态注册及主题初始化归入 `gui::home`。
- [x] 完成格式、离线编译、作用范围差异检查；不添加或运行测试。编译通过，现有工程有 20 条 dead-code warning。
- [x] Infrastructure 中的 MCP 运行时组合与构造归入 `infrastructure::agent_mcp`，根模块只保留其字段引用。

## 后续收敛：DataContext 直接持有状态与集中通知（2026-10-08）

### 完成进度

- [x] 移除 DataContext `Inner` 和 workspace、terminal、SFTP、service status Reader/Writer 包装器；状态锁、snapshot、工作区 maps 和 transfer ID counter 由 DataContext 直接持有。
- [x] GUI 刷新订阅改为 `gui_refresh_notification`；该 watch 只发刷新信号，DataSnapshot 和工作区的数字版本计数继续承担快照版本职责。
- [x] 会话打开、关闭、选择事件集中由 `DataContextNotice.session_lifecycle_events` 发布，MCP 订阅方改用具名订阅方法。
- [x] 更新 Application、GUI、MCP 对 DataContext 的读写入口；SFTP model 使用 DataContext 静态引用。
- [x] `cargo check` 通过；未新增或运行测试。`cargo fmt --all` 已应用于本次改动涉及的格式差异。

### 当前计划

- [x] DataContext 状态所有权、功能方法和通知路径重构完成。
- [x] 更新设计说明与本计划中的 DataContext 结构描述，避免保留已废弃的 Reader/Writer/Inner 约定。

## 后续修复：TCP 端口测试结果可见性（2026-10-08）

### 完成进度

- [x] 定位状态文字不可见的原因：成功/失败前景色是彩色底上的浅色文字，直接显示在白色面板上导致低对比度。
- [x] 端口测试结果改为带匹配背景的“端口可达/端口不可达”状态标签，地址、耗时和错误详情保持可读。
- [x] 按项目约束不新增测试用例。

### 当前计划

- [x] `cargo fmt --all -- --check` 和 `cargo check --locked --offline` 通过；未新增或运行测试，编译输出包含 19 条现有 dead-code warning。
- [ ] GUI 手工确认运行时显示。

## 后续功能：SOCKS5 活跃连接链路展示（2026-10-08）

### 当前完成进度

- [x] 为 SOCKS5 连接状态记录客户端地址、代理监听地址和已连接的目的地址。
- [x] 建立目标连接后发布链路状态；对应客户端断开后从状态列表移除。
- [x] SOCKS5 服务页面按“客户端 → 我的代理 → 目的”展示 IP 和端口，并提供空列表状态。
- [x] 格式化目标 Rust 文件，`cargo check --locked --offline` 通过；没有新增或运行测试。
- [ ] GUI 运行时显示尚未手工确认。

### 当前计划

- [x] 打通 SOCKS5 连接链路状态和 GUI 展示。
- [x] 完成格式和离线编译检查。
- [ ] 如需确认视觉效果，再在 GUI 运行时手工查看活跃连接列表。

## 后续调整：服务操作窗口间距（2026-10-08）

### 当前完成进度

- [x] 服务操作窗口及端口转发规则窗口中的 `.gap_4()`、`.p_4()` 分别收紧为 `.gap_2()`、`.p_2()`。
- [x] SOCKS5 活跃连接无滚动条时只显示最新 5 条，并保留当前活跃总数提示。
- [x] 目标界面文件的格式检查与 `cargo check --locked --offline` 通过；未新增或运行测试。

### 当前计划

- [x] 完成服务窗口间距调整，并确认目标目录不再保留 `.gap_4()` 或 `.p_4()`。

## 后续功能：HTTP 代理活跃连接链路展示（2026-10-08）

### 当前完成进度

- [x] HTTP Proxy 状态记录客户端地址、实际本地代理地址和已连接的目的 IP。
- [x] 普通 HTTP 转发与 HTTPS CONNECT 隧道均上报连接，并在结束或任务取消时清理状态。
- [x] HTTP 代理页面按“客户端 → 我的代理 → 目的”展示最新 5 条连接，并保留客户端会话数。
- [x] 格式检查与 `cargo check --locked --offline` 通过；未新增或运行测试。
- [ ] GUI 运行时显示尚未手工确认。

### 当前计划

- [x] 打通 HTTP 代理连接状态和界面列表。
- [x] 完成格式与离线编译检查。
- [ ] 如需确认视觉效果，再在 GUI 运行时手工查看活跃连接列表。

## 后续调整：端口转发运行态操作区（2026-10-08）

### 当前完成进度

- [x] 将端口转发规则的运行状态和活动连接数移入规则卡片操作区，与该区域的操作按钮共同显示。
- [x] 启用规则时提供“停用”按钮并隐藏编辑、删除按钮；规则停用后显示编辑、删除操作。
- [x] 将活跃连接的客户端端点和目标端点放入规则状态，卡片区域最多展示最新 5 条连接。
- [x] 目标 Rust 文件格式检查、`git diff --check` 与 `cargo check --locked --offline` 通过；未新增或运行测试。

### 当前计划

- [x] 完成格式与离线编译检查，不新增或运行测试。
- [ ] 如需确认布局效果，再在 GUI 运行时手工查看。

## 后续调整：GUI 直接使用 APPLICATION（2026-10-09）

### 当前完成进度

- [x] 移除 `gui::home::open_main_window` 对 `ApplicationHandle` 的 GPUI global 注入及 `Application` 参数。
- [x] GUI 应用层调用改为直接克隆 `application::APPLICATION`；删除 `global_state` 中已废弃的包装类型和读取函数。
- [x] `cargo fmt --all -- --check`、`cargo check --locked --offline` 与排除 `AGENTS.md` 的差异检查通过；编译有 19 条现有 dead-code warning。不新增或运行测试。

### 当前计划

- [x] 移除 GUI 对 `ApplicationHandle` 的依赖，保留 `GlobalStateHandle`。
- [x] 完成格式、离线编译和静态调用路径检查；检索确认 GUI 不再依赖 `ApplicationHandle` 或 `read_application`。

## 后续修复：GPUI 0.7.1 API 兼容（2026-10-09）

### 当前完成进度

- [x] 移除旧版 `Root` 的 Dialog、Notification、Sheet 图层渲染调用；GPUI Component 0.7.1 通过 Root 插件自动挂载这些图层。
- [x] 移除新版本 `Theme` 中已不存在的 `tiles` 颜色赋值。
- [x] `cargo fmt --all -- --check` 与 `cargo check --locked --offline` 通过；编译输出有 19 条 `dead_code` 警告。按项目约束未新增或运行测试。
- [ ] GUI 运行时交互尚未手工确认。

### 当前计划

- [x] 修复 GPUI 0.7.1 API 不匹配并完成格式与离线编译检查。
- [ ] 如需确认图层显示效果，再在 GUI 运行时手工检查 Dialog、Notification 和 Sheet。

## 后续功能：会话右键菜单独立 SSH 反向隧道（2026-10-09）

### 当前完成进度

- [x] 会话配置持久化反向转发参数；远端监听地址读取连接主机，设置页配置远端端口、本机转发地址和端口。
- [x] 普通 SSH/SFTP 连接不再自动启隧道；基础设施提供独立的 SSH 隧道连接运行时，复用连接配置中的代理、账号、密码或私钥认证。
- [x] 左侧会话右键菜单仅在已配置隧道后显示“打开 SSH 隧道”，运行中显示“关闭 SSH 隧道”；未配置时隐藏隧道项，参数通过“编辑”进入设置；点击打开成功后同时打开可交互 SSH 终端工作区，并在 Shell 就绪后发送 `http_proxy` / `https_proxy` 指向远端回环地址；编辑或删除连接时清理运行中的隧道。
- [x] 通过 GUI MCP 检查远端监听实际落在 `127.0.0.1:11200` / `[::1]:11200`；使用 `127.0.0.1` 的 HTTP 代理请求成功（返回百度 `HTTP 301`）。自动发送的代理变量地址已调整为 `127.0.0.1`。
- [x] `cargo fmt --all` 与 `cargo check --locked --offline` 通过；输出包含 19 条既有 dead-code warning。按项目约束未新增或运行测试。
- [ ] GUI 运行时菜单、连接及转发效果尚未手工确认。

### 当前计划

- [x] 完成隧道配置、独立运行时和菜单启停操作。
- [x] 完成格式与离线编译检查。
- [ ] 如需确认实际转发效果，再连接可用 SSH 服务进行 GUI 手工验证。

## 后续功能：MCP 打开 SSH 隧道（2026-10-09）

### 当前完成进度

- [x] 新增 `open_ssh_tunnel` MCP 工具，按 `profile_id` 校验并复用已保存的 SSH 隧道配置与认证信息。
- [x] 工具启动（或复用）反向隧道，随后打开可交互 SSH 工作区，自动发送指向远端回环监听端口的 `http_proxy` / `https_proxy`，并返回 `workspace_id`、连接信息和端口。
- [x] `cargo fmt --all -- --check` 与 `cargo check --locked --offline` 通过；编译输出有 20 条 `dead_code` 警告。按项目约束未新增或运行测试。

### 当前计划

- [x] 完成 Application、MCP bridge、dispatch 和工具 schema 接入。
- [ ] GUI 重启后通过 MCP 工具列表和 `open_ssh_tunnel` 做一次运行时联调。
