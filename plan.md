## 当前完成的进度

- [x] 检查工作区状态和依赖声明，确认更新前工作区无未提交修改。
- [x] 执行 `cargo update`，更新兼容范围内的依赖解析结果。
- [x] 生成新的 `Cargo.lock`：本次共锁定 142 个包到 Rust 1.96.1 兼容版本，包含 GPUI、MCP、SSH/SFTP 和异步运行时相关依赖更新。
- [x] 执行 `cargo build --locked` 完整编译。
- [x] 编译返回退出码 0；当前有 21 条未使用字段/方法警告，没有编译错误。
- [x] SFTP 本地和远程面板增加向左箭头后退按钮。
- [x] 本地、远程后退历史均按 `workspace_id` 隔离，后退操作不会重复压入历史。
- [x] 统一本地和远程路径保存入口，确保上级、后退、双击目录、路径弹窗确认都会触发保存。
- [x] 执行 `cargo fmt --all -- --check` 和默认 target 的 `cargo build --locked`。
- [x] SFTP 路径弹窗标题栏仅保留关闭按钮，保留底部路径提交操作。
- [x] 适配 `russh 0.63` 的 `PublicKeyOrCertificate` 主机密钥回调，并保留已知主机公钥指纹校验。
- [x] 依赖升级后重新执行 `cargo check --all-targets` 与 `cargo build --locked`，编译通过。
- [x] 修复关闭当前会话后选中状态被清空的问题，按会话打开顺序回退到相邻会话。
- [x] 修复 SSH 终端把远端 `EOF` 误判为完整会话关闭的问题，继续等待退出状态或 `Close`，并增加关闭超时兜底。
- [x] SSH 主动关闭改为发送 channel `EOF` 与 `Close`，移除正常关闭路径上的立即任务中止。
- [x] 增加 SSH 连接阶段、PTY/Shell、远端退出、EOF/Close、清理和命令投递失败日志，便于后续区分服务端关闭、客户端关闭和传输异常。
- [x] 远端会话关闭后保留 `TerminalBuffer`，进入只读历史 runtime，继续支持终端滚动条、鼠标滚轮和历史读取。
- [x] 修复 SSH 输入写入 channel 时错误递增终端 `mcp_snapshot_version` 的问题，输入日志仅记录字节数，版本只随远端终端内容变化递增。
- [x] 将 SSH 终端版本命名统一为 `mcp_snapshot_version` 和 `gui_snapshot_version`，并同步 MCP 增量读取参数与返回字段。
- [x] 将 MCP 终端读取链路统一命名为 `mcp_read_terminal`，同步工具、bridge、Application 和返回页类型命名。
- [x] 将保存的连接类型与打开的工作区视图分离，新增 `ConnectionProtocol::SshAndSftp`，为 MySQL、pgsql、Redis 预留扩展枚举。
- [x] 新建会话窗口顶部增加 Tailwind 风格 Select 协议选择框，列出 SSH、mysql、pgsql、redis；左侧保留 SSH 下的“连接 / 代理”二级配置入口。
- [x] MySQL、pgsql、Redis 先作为禁用的协议占位项展示，避免进入尚未实现的数据库连接表单。
- [x] 优化协议选择器视觉层次，触发器显示协议图标，下拉项只显示协议名称和选中状态。
- [x] 在基础设施代理层增加 `ConnectionAdapter` 连接适配接口，完成直连和 SOCKS5 两种实现，保留 SSH/SFTP 原有调用入口。
- [x] 增加 `SshTunnelAdapter`，通过 SSH `direct-tcpip` 通道把连接流转发到远程目标，并复用 SSH 主机密钥校验和密码/私钥认证。
- [x] 增加 SSH 本地端口转发监听器，默认绑定 `127.0.0.1:随机端口`，将每个本地连接转发到远程数据库目标，并提供可控关闭句柄。
- [x] 左侧会话右键菜单按连接类型显示 SSH/SFTP 打开入口，编辑和删除始终保留。
- [x] 兼容旧数据库中 `SSH` 和 `SFTP` 的配置值，读取时统一映射为 `SshAndSftp`。
- [x] 执行 `cargo check --all-targets`，编译通过；执行 `cargo fmt --all` 完成格式化。

## 当前的计划

- [ ] 后续运行时联调时重点检查 GUI 启动、SSH/SFTP 连接、终端输入输出和 MCP 服务端流程。
- [ ] 使用真实 SSH 服务端复现并核对 `remote_eof`、`ExitStatus`、`ExitSignal`、`Close` 的日志顺序。
- [ ] 联调确认远端关闭后可以拖动滚动条查看完整 scrollback，且输入命令会被拒绝而不会重新发送到已关闭会话。
- [ ] 在不改变功能边界的前提下，逐步清理当前 21 条未使用代码警告。
- [ ] 下次依赖变更继续执行 `cargo update` 后的 `cargo build --locked`，并复核 `Cargo.lock` 的变更范围。
- [ ] GUI 实际启动后确认新建会话窗口的协议栏、SSH/SFTP 右键菜单和旧配置读取效果。

本次没有新增测试用例；GUI 整体行为仍需通过实际启动和功能联调验证。
