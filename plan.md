# SFTP 增量同步与目录监听去重计划

**目标:** 手动 SFTP 上传/下载以及本地目录自动监听都复用同一套“文件没有变化则不传输”规则。未变化文件不得重新创建、截断或写入目标端；目录同步只处理新增或已变化的子文件。

**当前进度:**

- [x] 已完成现状排查和方案评估。
- [x] 已确认本地扫描与远端扫描已有 `size`、`modified_at/mtime`，但传输流程尚未使用。
- [ ] 尚未修改 SFTP 业务代码。

## 项目约束

- GUI 和 MCP 只能通过 ApplicationContext API 访问业务数据，比较逻辑留在 application SFTP core。
- 文件和协议 IO 遵循项目约束：Tokio 异步任务，GUI 异步操作使用 `cx.spawn`，阻塞文件扫描使用 `spawn_blocking`。
- 不新增 Rust 单元测试或 GUI 测试用例；使用 `cargo fmt`、`cargo check`、`git diff --check` 和真实 SFTP/MCP 手工回归。
- 单个 Rust 文件维护在 600–800 行；超过 800 行按职责拆目录，`mod.rs` 只保留声明、类型和初始化 glue。
- 保留工作区已有未提交改动，不使用 reset、checkout 或覆盖式回退。

## 现状与根因

- `src/application/sftp/core/local.rs` 的本地扫描已经取得 `size` 和 `modified_at`，但 `remote.rs` 的 `LocalTransferEntry` 没有保存本地元数据。
- `src/application/sftp/core/remote.rs` 的远端扫描已经取得 `size` 和 `mtime`，但 `RemoteTransferEntry` 只保存路径和目录标记。
- `upload_path` 对文件无条件调用 `sftp.create` 后复制；`download_path` 对文件无条件调用 `tokio::fs::File::create` 后复制，重复操作会产生真实写入。
- `src/application/sftp/core/watcher.rs` 目前只有路径 + 2 秒 debounce，窗口结束后仍然无条件调用 `upload_path_to_remote`。
- SFTP 远端通常只有秒级 `mtime`，且没有直接返回远端 checksum；“元数据相同”与“字节内容绝对相同”需要区分。

## 方案评估

| 方案 | 优点 | 缺点 | 结论 |
| --- | --- | --- | --- |
| `size + mtime` 元数据比较 | 判断快，远端只需一次 metadata，大文件成本低 | 秒级 mtime 无法发现同秒同大小的内容替换 | 本期采用 |
| 每次计算 SHA-256 | 能确认字节级相同 | 每次都要完整读取本地和远端，SFTP 成本高 | 不作为默认路径 |
| 持久化同步 manifest | 可保留跨重启的本地精确状态 | 状态可能与远端失配，仍需处理远端 metadata | 本期不引入 |

## 推荐方案

采用“元数据快速判断 + 传输层最终兜底”：

- watcher 只负责 notify 事件合并、本地签名去重和稳定性等待。
- 上传/下载 core 在真正打开目标文件前重新读取目标端 metadata，手动传输和 watcher 传输共用同一判断。
- 文件类型相同、大小相同、两端 mtime 都存在且归一化到 Unix 秒后相同时，判定为 `Unchanged`，不执行目标端 create、truncate 或写入。
- 任一端不存在、类型不同、大小不同、mtime 不同或远端 mtime 缺失时，判定为 `Changed`；metadata 读取失败返回错误，不能静默跳过。
- 目录不比较自身大小/时间，递归处理子项，只创建缺失目录并同步变化文件。
- 本期不使用时间容差，不引入持久化 manifest。秒级 mtime 导致的同秒同大小内容替换属于已知边界；严格字节校验另立 checksum 方案。

## 实施任务

- [ ] **1. 建立统一签名与结果模型**
  - 新增 `src/application/sftp/core/sync.rs`，必要时拆成 `sync/` 目录。
  - 定义内部 `FileSignature`、`SyncDecision` 和目录汇总结果；本地 `SystemTime` 与远端 `mtime` 统一转换为 Unix 秒。
  - 结果至少能统计实际传输文件数、跳过文件数和传输字节数，不改变现有 MCP 请求/响应字段。

- [ ] **2. 上传路径增量判断**
  - 让 `LocalTransferEntry` 保存大小和修改时间，让远端条目保存 size/mtime。
  - 每个文件在 `sftp.create` 前重新读取远端 metadata；签名一致则跳过，否则才打开本地文件并覆盖远端文件。
  - 手动上传和 watcher 的 `upload_path_to_remote` 必须共用该实现。
  - 同一 runtime 内串行处理同一目标，避免并发任务同时比较后互相覆盖。

- [ ] **3. 下载路径增量判断**
  - 让 `SftpCommand::Download` 或内部远端传输条目保留每个文件的 size/mtime，而不是只传入口 `total_size`。
  - 在 `File::create` 前读取本地 `symlink_metadata`；签名一致则不创建、不截断、不覆盖。
  - 目录下载只创建缺失目录并处理变化子文件；只有实际写入时才刷新本地目录快照。

- [ ] **4. watcher 事件去重与稳定性**
  - 保留 HashSet + debounce；debounce 后通过 `spawn_blocking` 读取当前 metadata。
  - 每个 watcher 保存最近处理的本地签名；同一路径签名未变化时直接丢弃，不创建新传输记录。
  - 文件仍在写入时继续合并事件并等待稳定，避免上传半成品。
  - watcher 不执行 SFTP IO，不在 notify 回调中阻塞；稳定路径交给 application SFTP runtime 做最终比较。
  - 父目录任务覆盖的子路径不再重复排队；删除、重命名语义保持现状。

- [ ] **5. 传输记录和日志**
  - 增加“未修改”终态：进度 100%、传输字节 0、速度 0、无错误；现有 GUI 和 MCP 字段可以直接展示。
  - 目录全部未变化时显示“未修改”；部分变化时显示“已完成”，日志记录跳过数和实际传输数。
  - 增加 debug 日志区分 `changed/uploaded`、`unchanged/skipped`、`metadata-error` 和 `unstable/retry`。
  - 只有完整写入并 flush 成功后才更新 watcher 已处理签名；取消、失败或断开不能标记为已同步。

## 风险与边界

- 远端没有 mtime 时安全策略是不跳过并记录 warn，不仅比较大小。
- 同大小、同秒级时间的内容替换是 metadata-only 的固有盲区；严格内容一致性需要单独评估 checksum、远端读取成本和取消策略。
- 下载写入本地可能再次触发 watcher；传输层比较应阻止重复写入，实现时还需确认不会形成上传—下载循环。
- 不修改 known_hosts、会话存储或 MCP 对外协议；不引入新的持久化 manifest。

## 验收标准

- [ ] 同一文件连续上传两次：第二次不重新写远端，传输字节为 0，状态为“未修改”。
- [ ] 同一文件连续下载两次：第二次不截断或覆盖本地文件，本地 mtime 保持不变。
- [ ] 修改文件大小或 mtime 后再次同步：只传输变化文件。
- [ ] 混合目录包含未变化、已变化和新增文件时：只写入后两者，已有目录不重复创建。
- [ ] 同一批 notify Modify 事件经过 debounce 后最多形成一次有效处理，重复事件不产生实际 SFTP 写入。
- [ ] 文件持续写入期间不会上传中间状态，稳定后只执行一次最终上传。
- [ ] metadata 缺失/失败、取消或写入失败时不会错误显示“未修改”，失败任务仍可重试。
- [ ] 日志能明确判断是否实际写入，而不是只有“上传任务开始”。

## 实施后的验证

- [ ] 执行 `cargo fmt -- --check`、`cargo check`、`git diff --check`。
- [ ] 使用真实 SFTP 端点回归单文件重复上传/下载、混合目录、watcher 重复事件、持续写入、取消和失败重试。
- [ ] 检查应用日志及 MCP SFTP 传输记录，确认 skip 不产生目标端写入，并确认既有 SFTP 打开、目录读取、监听启停和并发流程无回归。
- [ ] 完成后更新 `project.md` 中 SFTP core/service/watcher 的职责说明，并回填本计划实际验证结果。
