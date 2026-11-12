# MOD 工作文件能力

`src/features/workspace` 负责当前 MOD 工作目录的浏览、预览、编辑、文件操作、引用、离开保护及任务变更视图。`WorkspacePanel` 组合文件浏览和当前变更，工作台 `ContextPanel` 仅传入项目、任务、归档状态和可见性。交付版本与安装仍属于交付能力，不能把工作目录视为已交付产物。

- `useSourceBrowser` 保留目录展开偏好、分层加载及过期读取过滤；`useSourceManager` 保留编辑、保存/放弃确认、冲突决策、批量操作和项目切换保护。
- `sourceWorkspace` 持有项目文件引用、引用重映射和离开保护；聊天输入与工作台导航消费同一入口。
- `useTaskChanges` 持有项目失效 revision、任务操作记录和共享目录差异；应用协调入口按事件原项目使其失效。差异不等于当前会话独占的修改。
- `workspaceService` 集中声明工作文件及任务变更 IPC 的参数和结果，消费者不自行指定返回类型。`contracts/source.ts` 与 `contracts/changes.ts` 保留跨端数据契约。原生命令及其读写、链接、保留路径、归档、忙状态和磁盘冲突校验均不变。
- `useSourceFileActions.test.ts` 随能力迁入 workspace，Vitest 发现路径同步；既有测试继续验证切项目、卸载后迟到响应、部分失败及无效响应。业务服务替身只替换原生边界，不启动应用。

迁移没有改变产品交互、存储路径、原生命令名称或 source 业务标识。项目级类型检查与定向测试不代表桌面、真实文件拖放、回收站、图片预览或游戏运行验收。
