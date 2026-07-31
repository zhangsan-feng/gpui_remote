# 项目开发约束

不要编写设计文档、规格文档或实施计划书；开始实现前，只需与用户在对话中讨论并确认方案。
不要新增或编写测试用例。
UI 样式参考 [Tailwind CSS](https://tailwindcss.com/)，尽量用 size_full 和 flex_1  
图标设计与语义参考 [Lucide](https://lucide.dev/)；优先复用项目已有的 Lucide 图标资源，避免引入重复图标依赖。
gui 模块 职责边界收敛 core.rs (核心功能) internal.rs (给ui用的) external.rs(给外部用的) ui.rs渲染 mod.rs(类型定义、子模块声明、初始化与 Render 入口) 5个责任边界 
应用和基础设施 责任边界 core.rs (核心功能) external.rs(给外部用的) mod.rs(类型定义、子模块声明、初始化)
单个文件行数代码维护在600-800行 超过800行 就按照目录拆分 core internal external ui 4个目录 按照功能拆分文件
   
新需求 src/component/resizable_panel.rs 如果不设置宽度或者高度 就默认容器的上限
