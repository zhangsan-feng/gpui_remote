# 项目状态

## 当前状态

- GPUI 依赖已迁移到 `gpui-kit = "0.6"`，锁定到 `gpui-kit 0.6.0` 与同一套 `gpui-pre 0.3.3` 家族。
- `reqwest_client` 使用 `gpui-pre-reqwest-client 0.3.3`，`alacritty_terminal` 使用 crates.io `0.26.0`。
- 已完成旧 Zed GPUI、旧 gpui-component 和 Alacritty Git 依赖清理。
- 已完成 GPUI、组件库、应用初始化、资源加载和 Action 宏路径迁移。
- SSH 终端保留默认文本选区、复制和滚动行为；选区边缘自动上下滚动功能暂缓，不纳入当前版本。
- 已通过 `cargo fmt -- --check` 与 `cargo check --locked`；按项目约束未新增 GUI 测试。

## 目录 / 文件职责

- `Cargo.toml`：项目依赖、构建配置和开发构建优化；维护 gpui-kit 迁移后的直接依赖。
- `Cargo.lock`：可复现的依赖解析结果；锁定 GPUI、platform、macros 和 HTTP client 的同批次版本。
- `project.md`：记录项目当前状态、目录职责和已完成的迁移工作。
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
- `src/gui/workspace/sftp/ui/`：SFTP 本地/远程列表、选择和路径弹窗渲染。
- `src/gui/workspace/top_session/`：工作区顶部会话标签及会话切换状态。
- `src/infrastructure/storage/`：SQLite 会话存储、已知主机密钥和持久化基础设施。
