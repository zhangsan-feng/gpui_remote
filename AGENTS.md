# 项目开发约束

不要编写设计文档、规格文档或实施计划书；开始实现前，只需与用户在对话中讨论并确认方案。
不要新增或编写测试用例。
不要写mod sss {} 我希望是mod.rs 是入口 功能是单独的rs 文件
不要写 pub (crate in xxx)
引入依赖统一在代码文件顶部 
不要在代理逻辑里面写 use super::函数 或者 use crate::
UI 样式参考 [Tailwind CSS](https://tailwindcss.com/)，尽量用 size_full 和 flex_1  
图标设计与语义参考 [Lucide](https://lucide.dev/)；优先复用项目已有的 Lucide 图标资源，避免引入重复图标依赖。

gui 模块 职责边界收敛 5个责任边界
core.rs (核心功能)
ui.rs渲染
internal.rs (给ui使用)
external.rs(给外部用的)
mod.rs(类型定义,子模块,初始化与 Render 入口, start_subscribe, init_component_data 等) 

应用和基础设施 责任边界 
core.rs (核心功能) 
external.rs(给外部用的) 
mod.rs(类型定义、子模块声明、初始化)

单个文件行数代码维护在600-800行 超过800行 
就按照目录拆分 4个目录 目录里面按照功能拆分文件
core
ui
internal
external
mod.rs 


   

