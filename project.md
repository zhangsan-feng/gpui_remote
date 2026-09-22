# gpui_remote 项目说明

## 项目架构

项目采用分层设计：

- `domain`：领域模型与领域类型，不依赖界面实现；会话配置使用 `ConnectionProtocol` 表示连接类型，SSH 与 SFTP 共用 `SshAndSftp` 配置。
- `application`：应用层核心流程、会话管理、SSH/SFTP 用例与状态编排。
- `infrastructure`：外部基础设施，包括 SQLite 存储、代理、MCP 服务和运行时上下文。
- `gui`：GPUI 界面入口、窗口、侧边栏、工作区及 SSH/SFTP 视图；按 `core`、`external`、`internal`、`ui` 划分职责。新建会话窗口顶部使用带图标和说明项的 Select 展示 SSH、mysql、pgsql、redis 协议入口，数据库项作为后续扩展的禁用占位。
- `component`：可复用的 GPUI 组件、主题、颜色和窗口工具。

IO 操作由异步运行时承载；服务端相关能力位于 `infrastructure`，通过应用层服务连接领域模型与外部资源。

## 项目目录结构

| 目录/文件 | 主要功能 |
| --- | --- |
| `Cargo.toml` | 包信息、Rust 版本和直接依赖声明。 |
| `Cargo.lock` | 可复现构建所使用的完整依赖解析结果。 |
| `build.rs` | 构建阶段处理项目构建信息。 |
| `src/main.rs` | 应用启动、日志初始化、GPUI 窗口和根组件入口。 |
| `src/domain/` | 会话、终端等领域类型。 |
| `src/application/` | 应用上下文、状态图、会话及 SSH/SFTP 核心服务。 |
| `src/infrastructure/` | 存储、代理、MCP 和外部运行时设施。 |
| `src/infrastructure/proxy/` | 统一连接适配接口及直连、SOCKS5、SSH 隧道、本地端口转发实现，供 SSH/SFTP 和后续数据库驱动复用。 |
| `src/infrastructure/storage/` | SQLite 会话仓储、已知主机和存储派生实现。 |
| `src/infrastructure/agent_mcp/` | MCP 认证、桥接、路由、工具与服务端。 |
| `src/gui/mod.rs` | GUI 子模块声明和 Render 入口。 |
| `src/gui/title_bar/` | 标题栏、关于窗口、会话操作窗口和设置窗口。 |
| `src/gui/sidebar_session/` | 会话列表、选择和交互逻辑。 |
| `src/gui/workspace/` | 工作区布局及 SSH、SFTP、顶部会话视图。 |
| `src/component/` | 可复用列表、可调整面板、主题和窗口组件。 |
| `logs/` | 运行时日志输出目录。 |
| `data/` | 应用运行数据目录。 |
| `docs/` | 项目补充文档。 |

## 主要依赖

- `gpui-kit`：GPUI 组件和应用基础能力。
- `alacritty_terminal`：终端解析与终端状态处理。
- `russh`、`russh-sftp`：SSH/SFTP 连接与文件操作。
- `axum`、`tokio`：异步服务端和运行时。
- `rmcp`：MCP 服务端传输和协议支持。
- `rusqlite`：内置 SQLite 存储。
- `serde`、`serde_json`：数据序列化。
