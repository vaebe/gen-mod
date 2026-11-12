# 跨项目长期记忆

`src/features/knowledge/` 拥有长期记忆查询、编辑和展示能力：

- `types.ts` 定义记忆条目、手动编辑输入、分页查询和自动保存回执契约；聊天 reducer 复用回执类型并继续校验跨进程数据。
- `knowledgeService.ts` 将四个原生命令绑定到明确参数和结果，页面与 composable 不自行指定 IPC 返回类型。
- `useLongTermMemory.ts` 管理分页、选择和保存状态，保留请求代际、卸载保护、保存互斥及失败输入保留；`LongTermMemorySettings.vue` 和 `MemoryEntryEditor.vue` 负责展示与编辑。
- 设置页组合长期记忆界面；聊天组合 `MemorySaveNotice.vue` 展示保存回执，并导航到设置页查看对应条目。长期记忆不绑定当前项目，也不加入项目会话刷新体系。

原生 `src-tauri/src/knowledge/` 继续拥有持久化、检索、证据和用户编辑保护。手动修改与状态变更使用 revision 校验；自动保存不能覆盖用户维护的内容。数据保存在全局 `knowledge_notes`，来源项目是来源信息而非数据所有者；删除项目保留长期记忆，项目生命周期既有测试覆盖这一约束。

验证复用聊天 reducer 的实时/历史回执用例以及原生 knowledge、project_lifecycle 用例。类型检查与静态引用检查不等同于设置页、模型或游戏运行验收。
