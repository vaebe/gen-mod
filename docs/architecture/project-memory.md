# 项目记忆能力

`src/features/project-memory` 持有项目记忆展示、读取状态、失效通知和契约。工作台只向 `ProjectMemoryTab` 传入项目身份、可见性和只读状态；游戏调查仍属于游戏能力，其采纳操作通过 `projectMemoryService.applyInspection` 写入项目上下文。

- `useProjectMemory` 按 modId 接收应用协调入口的失效通知，同时保留原生 `project-memory-changed` 监听；可见时重新读取，切换项目、卸载或新请求后忽略旧响应。
- `projectMemoryService` 将 `get_project_context`、`apply_inspection` 的参数与 `ProjectContext` 返回值绑定；归档状态复用 `modService.get`。服务统一沿用原生错误转换，视图不声明任意 IPC 返回类型。
- `types.ts` 定义上下文、任务快照、来源证据和更新回执；会话消息契约与 reducer 使用同一更新回执类型。
- Rust `project_context` 继续拥有项目记忆仓储、来源验证及任务快照；`game_inspection::apply_inspection` 保留原项目检查、归档限制、revision 检查及引用指纹保护。Node 项目上下文工具及其任务身份校验保持原入口。

项目记忆不包含跨项目长期记忆或玩家反馈。本次为能力迁移，保留现有 UI、数据存储和刷新机制；复用会话业务测试验证原项目通知，静态检查与测试不代表真实桌面或模型运行验收。
