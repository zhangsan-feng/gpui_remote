# Application 模块化与 Server 分层设计

## 目标

重构 `src/application`，让每类应用用例由独立模块和服务对象承载，并由顶层 `Application` 按模块组合。SSH/SFTP Server 的监听、协议适配和文件系统 IO 归入 Infrastructure；Application 继续负责业务校验、会话编排及 SFTP 根目录访问规则。整理 `application/ports`，只保留当前架构确实需要的边界。

## 当前问题

- `ApplicationContextInner` 目前只组合部分服务；SSH Server、SOCKS5、HTTP Proxy、主题和 MCP 设置的用例方法散落在 `application/core.rs`。
- 端口转发、端口检测、HTTP Proxy 等目录已有模块，但操作仍以 `ApplicationContext` 的方法暴露，调用端无法按模块聚合使用。
- `application/sftp_server` 承担 SFTP 会话、句柄和根目录约束；Infrastructure 已承载 SSH/SFTP 协议处理和底层文件系统，但绑定关系不够清晰。
- `application/ports` 有未被消费的类型，也有跨 Application/Infrastructure 实际使用的接口，不能按目录整体删除。

## 模块结构

`Application` 组合当前所有应用用例模块：

```text
Application
├── sessions
├── database
├── ssh
├── sftp
├── ssh_server
├── sftp_server
├── http_proxy
├── socks5_proxy
├── port_forward
├── port_test
├── theme
└── mcp
```

`src/application` 根目录只保留 `mod.rs`，用于声明模块并组合 `Application`。每个功能模块的 `mod.rs` 定义并组合服务类型，`core.rs` 放核心流程，`external.rs` 放供其他模块调用的用例入口；模型、映射和校验代码也归入对应模块。会话打开/关闭编排位于 `session`，应用关闭屏障位于 `lifecycle`。各子服务继续共享其内部 `Arc` 状态，克隆 `Application` 不会复制运行时业务数据。

将现有 `ApplicationContext` 重命名为 `Application`，并迁移 `global_state`、MCP、GUI 和 `main.rs` 的类型引用。GUI、MCP 和 `main.rs` 按功能通过 `Application` 的模块字段调用；不保留旧名称的类型别名。

## 职责边界

### Application

- 应用层负责校验请求数据是否合法、操作资源是否存在以及对应业务规则；校验通过后才调用 Infrastructure。
- `sessions` 管理会话配置用例、运行时会话映射和打开/关闭会话所需的跨模块协调。
- `ssh`、`sftp`、`database` 管理各自客户端连接的应用生命周期和工作区行为。
- `ssh_server` 管理服务配置的验证、启动/更新/关闭用例以及状态向 DataContext 的同步。
- `sftp_server` 管理 SFTP 会话范围、句柄及根目录授权规则；这些规则决定客户端可访问的资源，继续由应用层执行。
- `http_proxy`、`socks5_proxy`、`port_forward`、`port_test` 各自承载对应的配置、验证、状态和操作用例。
- `theme`、`mcp` 管理对应配置的验证、读取和保存用例。
- GUI IO 仍通过 `cx.spawn` 和 Tokio 执行；DataContext 热数据继续按项目架构直接读写/订阅。

### Infrastructure

- `ssh_server` 持有 SSH 监听器、russh 服务处理器、主机密钥和配置持久化。
- `sftp_server` 持有 russh-sftp 协议适配、本地文件句柄和文件系统操作。
- InfrastructureContext 组合并初始化这些具体控制器；协议处理、网络和文件 IO 不放入 Application。
- Infrastructure 在启动 SSH/SFTP Server 时接收 Application 的具体 `SftpServerApplication` 实例；SFTP handler 直接调用其会话、句柄和根目录校验方法，不通过 trait 接口转发。
- SFTP 文件系统由 Infrastructure 的具体控制器实现并调用，不保留仅供 Infrastructure 内部使用的文件系统 trait。
- 其他 Infrastructure 服务继续提供底层能力和持久化；业务校验留在 Application。

## Ports 清理规则

- 逐项通过全仓引用确认未使用的类型或导出后再删除，不删除仍被基础设施或应用模块使用的 Port。
- 删除 `SftpServerGateway` 和 `SftpServerFilesystem` trait；SFTP Server 使用具体 Application 服务与 Infrastructure 文件系统控制器。
- 删除经全仓检索确认无消费者的 `PortTestFuture`、`SftpServerFuture`、`SftpFilesystemFuture` 等类型及导出；其余现有端口逐项按实际调用关系保留或删除。
- 不新增 trait 或依赖。

## 行为与约束

- 保留现有启动顺序、配置持久化、运行状态发布、工作区打开/关闭和服务关闭行为。
- 用户可见的 GUI、MCP 参数和交互行为不改变；只调整应用入口的模块归属和调用路径。
- 不添加测试用例；使用格式检查、编译/构建和差异检查验证改动。
- Rust 文件保持在 800 行以内；超过时按职责拆分。
- 保留当前工作区已有的未提交改动，只修改本次模块化所需文件。

## 验收条件

1. `Application` 有明确的模块字段，所有现有应用服务均有归属；代理和端口转发等模块不能遗漏。
2. 各模块的用例入口位于对应 Application 模块，根 `core.rs` 不再承载各服务的独立配置操作。
3. SSH/SFTP Server 的协议、监听和文件 IO 由 Infrastructure 直接拥有；SFTP 根目录和会话授权仍由 Application 校验。
4. 不存在 SFTP Server Gateway/FileSystem trait；`application/ports` 中被删除的每个类型均无全仓消费者。
5. GUI、MCP、`main.rs` 与关闭流程完成迁移，格式和 `cargo check --locked --offline` 通过；不增加或运行测试。
