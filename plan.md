## 当前完成的进度

- 主窗口偶发右侧透明：已添加窗口尺寸事件和实际布局绘制诊断，记录窗口边界、视口、根布局、缩放和最大化状态；绘制日志按状态去重，沿用异步日志写入。
- 诊断代码已通过 `cargo check --locked --offline`（20 项警告）；尚未复现，未判定根因或实施猜测性修复。

## 偶发透明问题：当前的计划

- 运行包含诊断代码的新版本；再次出现时结合发生时间，检查当天日志中的 `main_window_bounds_changed` 和 `main_window_paint_geometry`，区分布局未铺满与底层呈现异常。
- 未新增测试用例，未使用 computer use。

- 已移除 `top_session` 的固定 240px 宽度，横向标签按 GPUI 实际内容布局测量宽度。
- 虚拟列表、横向滚动、拖拽预览及换位动画复用各标签的测量宽度。
- 保留现有标签视觉层次、选中、悬停与关闭交互。

## 当前的计划

- `cargo check --locked --offline`、修改文件的 `rustfmt --check` 及 `git diff --check` 已通过；编译输出 20 项警告。
- 实际 GUI 中长地址、状态变化、横向滚动和拖拽效果待运行应用验证；未新增测试用例，未使用 computer use。

## 代理与 SSH 隧道配置自动命名：当前完成的进度

- 移除两类配置的名称输入框及输入状态。
- Application 在加载及保存时统一生成名称：代理为 `主机:端口`，隧道为 `远端端口 → 转发地址:转发端口`；已有配置同步使用新名称，列表与会话下拉框保持一致。
- 移除列表中与自动名称重复的详情行，重复配置提示改为对应的连接参数说明。

## 代理与 SSH 隧道配置自动命名：当前的计划

- `cargo check --locked --offline`、修改文件的 `rustfmt --check` 与 `git diff --check` 已通过（20 项编译警告）。
- 实际 GUI 的表单与列表显示待运行应用确认；未新增测试用例，未使用 computer use。

## 配置行编辑状态：当前完成的进度

- 代理与 SSH 隧道列表中，当前编辑行使用主题选中背景、主色边框、加粗名称及“编辑中”标识。
- 状态沿用当前编辑 ID，取消或保存成功后消失，切换编辑时同步移动至对应行。

## 配置行编辑状态：当前的计划

- `rustfmt --check`、`git diff --check` 和 `cargo check --locked --offline` 已通过（20 项编译警告）。
- 实际 GUI 显示效果待运行应用确认；未新增测试用例，未使用 computer use。
## 工作区终端分割线：当前完成的进度

- 行号栏右侧的 1px 边框改为主题边框色，在行号与正文之间显示分割线。
- `cargo check --locked --offline`、修改文件的 `rustfmt --check` 和 `git diff --check` 已通过（20 项编译警告）。

## 工作区终端分割线：当前的计划

- 实际 GUI 效果待运行应用确认；未新增测试用例，未使用 computer use。
## 内置 SSH server 反向隧道：当前完成的进度

- 实现 `tcpip_forward` 与 `cancel_tcpip_forward`，监听客户端指定的具体接口 IP，支持系统分配端口和并发双向转发。
- 每个 SSH 会话独立管理监听及转发任务；取消隧道或会话退出时中止任务并释放端口。添加监听、拒绝、取消、通道失败及转发结束日志。
- 自动审批拒绝了通配网卡监听版本；当前实现拒绝空地址、通配地址及组播地址，仅接受明确的接口 IP。
- `cargo check --locked --offline` 已通过（20 项编译警告）；未新增测试用例，未使用 computer use。

## 内置 SSH server 反向隧道：当前的计划

- 运行新版本后验证实际反向隧道连接、取消和断线后的端口释放；尚未进行端到端验证。
## 终端复制卡死：当前完成的进度

- 检查现有日志，复制流程原来只有 DEBUG 日志；未取得卡死时的线程栈，不能确认具体 Win32 阻塞点。
- 发现原有 `cx.spawn` 仍在 GUI 线程同步调用系统剪贴板；Windows 终端复制改为 `cx.spawn → tokio::spawn → Application → Infrastructure → spawn_blocking`。
- 使用现成 `arboard` 库在后台写入文本，移除手写 Win32 和 unsafe；增加剪贴板竞争重试、单次写入限制和 3 秒等待超时日志，超时不会强制取消系统调用。
- 增加选中文本提取、后台写入开始及完成日志，不记录复制的内容。
- `cargo check --locked --offline` 已通过；未新增测试用例，未使用 computer use。

## 终端复制卡死：当前的计划

- 库版本的 `cargo check --locked --offline`、`cargo build --release --locked --offline`、修改文件的格式检查及差异检查通过；Release 有 19 项既有警告。待复测终端复制，尚未验证实际剪贴板操作。

## Windows SSH 当前会话代理变量：当前完成的进度

- 确认内置 Windows SSH 启动 cmd，原先固定发送 Unix export 命令。
- 根据常见提示符识别 cmd、PowerShell 和 Unix Shell，分别发送 set、$env: 和 export 到现有交互通道；识别超时记录日志并沿用 Unix 行为。
- 反向隧道请求的监听地址改为 127.0.0.1，与代理环境变量保持一致。
- 已执行 cmd 和 PowerShell 环境变量设置命令，读取结果正确；未新增测试用例。

## Windows SSH 当前会话代理变量：当前的计划

- 已合并剪贴板修改编译 Release；待验证实际 SSH 会话及代理流量，自定义提示符可能无法识别。
## 终端右键即崩溃：当前完成的进度

- Windows Application 事件确认 2026-10-27 17:16:13 发生异常退出，路径为 target/debug/gpui_remote.exe，异常 0xc0000409，故障模块 ucrtbase.dll；这不能直接证明具体 Rust panic 位置。
- 现有应用日志未覆盖右键菜单构建；Windows 错误报告归档仅保留 Report.wer，临时 dump 已不可用，尚未确认根因。
- 用户确认选中文本后右键即触发；增加 tracing 与 tracing-subscriber，记录右键、菜单构建 span、工作区、选中状态、线程和源码位置，沿用异步日志文件。
- 增加 panic hook，将异常位置及强制捕获的 backtrace 同步保存到 logs/panic.log，另发 tracing error，保留原 panic hook 行为。原生异常若未经过 Rust panic hook，仍需转储定位。
- `cargo check --locked --offline` 已通过，未新增测试用例，未使用 computer use。

## 终端右键即崩溃：当前的计划

- tracing 接入后 `cargo check --offline`、修改文件格式检查及差异检查通过；`cargo rustc --release --locked --offline --bin gpui_remote -- -C debuginfo=1` 通过（19 项既有警告）。
- 需运行该 Release 再选中文本并右键，读取新增诊断和 panic.log 后继续定位；当前仅补充诊断，未实施猜测性菜单修复。


## Event bus：当前完成的进度

- 参考同级 `gpui_idea`，新增统一 `EVENT_BUS`：数据刷新请求、按组件合并的 GUI 刷新通知、MCP 会话生命周期事件。
- 数据刷新请求经有界队列交给 Application 的 Tokio 订阅任务执行，支持网络配置加载及 SFTP 本地/远程目录刷新；保留目录请求顺序、校验、错误回传和退出时已接受操作的等待屏障。
- DataContext 完成内存快照提交并释放锁后，定向通知工作区、终端、SFTP、服务状态或网络配置；终端内容和 SFTP 进度不再触发无关面板刷新。
- 移除 DataContext.notice，迁移全部 GUI 和 MCP 订阅；增加事件分发、刷新失败、订阅启动与停止日志。
- `cargo check --locked --offline` 通过（20 项既有代码警告，另有增量缓存硬链接回退警告）；修改文件的 `rustfmt --check` 通过。未新增测试用例，未使用 computer use，未修改 README.md。

## Event bus：当前的计划

- 待实际运行 GUI 验证：会话打开/切换/关闭、持续终端输出、SFTP 目录导航与传输进度、网络配置和服务状态更新，以及退出时任务清理。
