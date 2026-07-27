# 项目开发约束

1. 不要编写设计文档、规格文档或实施计划书；开始实现前，只需与用户在对话中讨论并确认方案。
2. 不要新增或编写测试用例。
3. 按照功能细分拆代码文件 尽量每个文件代码不要太多 
4. UI 样式参考 [Tailwind CSS](https://tailwindcss.com/)，图标设计与语义参考 [Lucide](https://lucide.dev/)；优先复用项目已有的 Lucide 图标资源，避免引入重复图标依赖。

新的需求
src/gui/workspace/session 这里的session应该管理 DraggableList 和 选中的id 才对里面怎么写了一堆的事件
workspace session 给父组件暴露当前选中的id 父组件拿id 去渲染终端才对 
然后event 事件 WorkspaceSessionEvent 不能用全局的吗 怎么又自己定义了个
WorkspaceSessionEvent::Change 这里也没有用到  Open 和 Close 直接复用全局的