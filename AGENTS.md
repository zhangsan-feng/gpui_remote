
## 项目约束
跳过编写设计文档，确认好方案直接开始实现
gui 不写测试用例
给项目配置好日志记录方便定位问题

UI 样式参考 [Tailwind CSS](https://tailwindcss.com/)
图标设计与语义参考 [Lucide](https://lucide.dev/)；优先复用项目已有的 Lucide 图标资源，避免引入重复图标依赖。

gui 模块 个责任边界
core.rs (核心功能 数据流向)
ui.rs渲染
external.rs(给外部用的)
mod.rs(类型定义,子模块,初始化与 Render 入口, start_subscribe, init_component_data 等) 

应用和基础设施 责任边界 
core.rs (核心功能) 
external.rs(给外部用的) 
mod.rs(类型定义、子模块声明、初始化)
定位问题优先看日志

单个文件行数代码维护在600-800行 超过800行 
就按照目录拆分 目录里面按照功能拆分文件
把功能文件改成目录 比如 core.rs 改成core 文件夹里面按照功能拆分

服务端 按照ddd的架构设计实现
服务端 使用 get 和 post 禁止使用其他的rest 规范

## 项目进度 用于记录项目状态。 在project.md 中
每次修改完成功能并验证完成之后修改 project.md
 project.md 中的要求
目录/
    文件名 主要做什么功能
