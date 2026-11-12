# 会话业务接口迁移清单

任务 03–07 的交接记录；下文 03–06 的暂留说明描述对应批次，现行接口以末尾 07 为准。

## 已接通的扩展请求操作

`ExtensionDialog.vue` 从现有 `useSessionStore` 读取 `extensionRequest`，调用 `respondExtensionRequest(request, response)`。没有新建包装 store 或平行运行时。

- runtime 把经 client 过滤的扩展事件投影为不可替换的只读请求快照，保留项目、会话、generation、请求 ID 及原显示字段。
- store 按请求原项目和会话选择所属 runtime，不用当前选择的 client 回应。
- runtime 消费当前请求后再发送，重复点击、过期请求与旧代次不会重复发出回应；失败写回原 runtime 的 error 并向调用者抛出原错误，不自动重发。
- client 使用专用单向响应传输，携带原身份，原样传递 value、confirmed 或 cancelled，不创建普通 RPC pending 请求。
- 弹窗继续负责输入框、按钮和可见期间的超时计时；超时闭包绑定当时请求。后台页面卸载期间不新增超时执行行为。
- 原 `uiRequest` 和其他旧入口暂留兼容，最终统一由 07 收缩；本票后扩展弹窗已没有旧入口引用。

## 现有职责与后续迁移

输入业务继续归 `sessionRuntime` 的 send、abort、recover、clearPending 和 `followupQueue`；04 已将附件消耗、输入组装、提交/入队选择及失败补偿移到 runtime 的 submitInput。页面保留工作文件引用、文件读取和 useDraft 的持久化回调，不再写入 attachments 或 followups。会话选择、存续和按身份查询由 session store 管理；连接、代次和 RPC 匹配由 client 管理。恢复由 runtime 协调权威快照、待确认提交和队列，不交由页面改写确认状态。06 已删除 session/runtime 的四类 revision，数据读取及失效由所属能力维护。

## 完整生产消费者清单

对 `src` 中 `stores/session`、`sessionRuntime` 和 `createPiClient` 的导入做全量核对；包含模板、脚本及 stateFor 返回值的间接访问。下列清单覆盖全部生产页面；06 后 ModManagerPage 不再消费 session store，剩余 10 个页面/组件及 1 个日志 composable。

| 消费者 | 现有访问与迁移责任 |
| --- | --- |
| `src/features/chat/ExtensionDialog.vue` | 已仅使用 extensionRequest/respondExtensionRequest；后续保留业务入口 |
| `src/features/chat/ChatPage.vue` | 读取消息、工具、任务、模型、忙碌/恢复/队列/时间状态；open/recover/clearPending/abort/restart/selectModel/setThinking/compact；04 已使用 submitInput、addAttachments/removeAttachment、editFollowup/operateFollowup、restorePendingInput；提交/恢复按发起身份路由到原 runtime。05 已使用 refreshModel、summaryFor、initialized、selectionId/isSelected 与 syncProjectAccess；不再写 task/readOnly 或访问内部 runtime。06 已改用工作文件/交付能力投影及 useVerification，不再读取 session revision |
| `src/features/chat/ChatComposer.vue` | modId 关联工作文件引用；05 保留只读身份 |
| `src/features/delivery/ModManagerPage.vue` | 06 已移除 session store；版本操作结果交 deliveries.update，反馈由 useVerification 自主管理 |
| `src/features/projects/RemoveProjectDialog.vue` | 05 已使用 isProjectBusy 与 forgetProject；归档/删除成功后按项目释放，不遍历内部集合 |
| `src/features/skills/SkillsPage.vue` | anyBusy 限制 Skills 操作；05 核对并保留只读查询 |
| `src/features/workbench/ContextPanel.vue` | 保留 modId/sessionId/task/diagnostics/readOnly；15 工作文件视图交 WorkspacePanel（内部 useTaskChanges）、交付快照交 useDeliveriesStore、记忆交 useProjectMemory；日志读取/打开委托 useSessionLogs |
| `src/features/projects/ProjectSidebar.vue` | 05 已使用 summaryFor.activity、isBusy/isRunning；导航失败归本组件，不污染所选会话 error |
| `src/features/sessions/SessionManageDialog.vue` | 05 已使用 isSelected/isBusy/close，保持删除与选中会话协调 |
| `src/features/workspace/SourceBrowser.vue` | 05 已使用 isProjectBusy，保留 modId/task/chat 只读投影推导活动文件 |
| `src/features/workbench/WorkbenchLayout.vue` | 05 已使用 modId/sessionId/switching、selectionId/initialized；等待离开保护后仍核对同一选择 |

| `src/features/chat/useSessionLogs.ts` | 05 从工作台接管会话日志轮询、错误与打开操作；仅可见时读取，按原身份和请求代次丢弃过期结果 |

内部生产消费者只有 `src/features/sessions/useSessionStore.ts` 创建/持有 sessionRuntime，和 `src/features/sessions/sessionRuntime.ts` 创建/持有 client。现有 `sessionRuntime.test.ts` 与 `client.test.ts` 使用对应边界；05 改接口时保留恢复及身份过滤验证意图。

`session.uiRequest`、`session.client` 在本票后已无页面消费者，但暂不在本票删除兼容导出。05 后 stateFor、runtimes、selectedKey 已无生产页面消费者；旧导出仍暂留至 07，不能声称已隐藏全部内部状态。

## 本票验证边界

定向 `sessionRuntime.test.ts`、`client.test.ts` 验证恢复、身份/代次过滤、扩展回应完整参数、无需普通回执、重复回应抑制、切换会话后的失败归属。原生 IPC 为测试替身，未启动桌面应用、真实 pi 或游戏；弹窗实际交互、计时和原生传输仍需运行验收。

## 04 输入操作契约与验证

- `submitInput(target, text, captureDraft)` 在原 runtime 组合附件、选择提交或入队，并负责失败时附件恢复与草稿 finish；返回草稿清理是否成功。清理失败仍按已提交处理，页面提示勿重复发送。
- `captureDraft` 继续由既有 `useDraft` 捕获原项目/会话及草稿修订；新草稿和会话切换保护不迁入运行时。页面只将工作文件引用加入 text，成功后移除已提交引用。
- `editFollowup` 在队列串行操作内恢复文本和附件，沿用 editingId 与草稿持久化失败后的暂停；`operateFollowup` 按当前原会话状态选择引导或普通投递，错误归原队列。
- `restorePendingInput` 只恢复 rejected 记录，草稿保存成功后才删除保留提交；页面回调检查新草稿冲突。原生受理记录、generation 过滤、恢复和停止后的队列暂停保持原链路。
- 保留旧 runtime/store 导出供 05 收缩；本票不迁移跨能力 revision、会话选择、task/readOnly、模型刷新和其他消费者。
- 本票定向会话 runtime/client 用例覆盖未发送附件恢复、响应丢失后受理/未确认两类结果且不重复启动、队列保存失败、拒绝提交恢复持久化失败、切换后失败归属及停止通信失败仍暂停。使用 IPC 替身，未运行应用、真实模型或游戏。


## 05 选择、恢复与外围消费者

- 会话 store 提供 `SessionIdentity`、只读 `SessionSummary`、`isSelected`、`selectionId`、`initialized`、`summaryFor` 和 `isProjectBusy`。选择仍只切换展示槽，不关闭后台任务。
- `syncProjectAccess(project)` 按项目与会话元数据同步访问权限；页面不写 readOnly。活跃会话 task 来自权威运行时快照；只读历史会话由 runtime 读取本会话历史任务，页面任务列表只供展示，不覆盖恢复结果。
- 模型刷新由 store 捕获原 runtime 并记录其错误，切换会话不会把旧失败写入新选择。
- `forgetProject` 调用 runtime 的 release 撤销订阅身份；保留原有项目代次保护。业务删除/归档成功后才释放，源文件离开保护保持原链路。
- 工作台日志归会话能力 `features/chat/useSessionLogs.ts`；组件仅提供可见条件并展示只读结果。日志类型归 `contracts/session.ts`。
- `ChatComposer` 的身份读取、`SkillsPage` 的 anyBusy、扩展请求操作和 ContextPanel 的诊断读取已逐项核对，无额外写入。06 继续负责四类 revision；07 才删除无人使用的内部兼容导出并收紧剩余投影。

验证：定向 sessionRuntime/client 共 15 个用例通过，包含后台选择/释放与只读历史恢复；全项目 `vue-tsc --noEmit` 和变更源文件定向 ESLint 通过。使用原生 IPC 替身；未启动应用、真实模型或游戏，页面实际交互未验收。

## 06 原项目跨能力刷新

`application/projectRefresh.ts` 接收 client 已核对项目、会话、generation、subscription 和 sequence 的 PiEnvelope，按原 `event.modId` 转交。session store 将入口注入每个 runtime；没有全局事件总线，没有读取当前选择来判断后台结果归属。后台 runtime 继续存活，项目释放时原 client 身份失效，交付在途读取同时被撤销。

| 能力与稳定入口 | 数据、失效与消费者 |
| --- | --- |
| 工作文件 `useTaskChanges` / `invalidateSource` / `sourceRevision` | 能力持有任务操作、变更和差异读取及过期结果保护；ContextPanel 仅展示，SourceBrowser 的现有目录与预览能力消费同项目 revision；ChatPage 据此重新读取任务列表。原生 project-source-changed 仍由 SourceBrowser 原监听处理，保留忙碌/可见保护。15 已整体迁入 `features/workspace`，由 `WorkspacePanel` 组合文件与差异视图 |
| 交付安装 `useDeliveriesStore` | 按项目持有交付/安装快照、读取代次、错误；refresh 读取原项目，update 接受版本操作返回的权威结果，projectFor 同时尊重项目列表中更高 revision；更新继续投影到既有 projects store。ContextPanel、ModManagerPage、ChatPage 展示相同项目结果；旧请求和已释放请求不写回。13 迁目录时保持此接口 |
| 项目记忆 `useProjectMemory` / `invalidateProjectMemory` | 记忆能力管理自身失效、可见性、数据读取及失败状态；ProjectMemoryTab 不再接收 session revision。原生 project-memory-changed 监听仍由能力维护。16 已将实现、契约及类型服务迁入 features/project-memory |
| 玩家反馈 `useVerification` / `invalidateVerification` | 反馈能力管理按项目失效、项目/版本数据读取、错误与请求代次；ChatPage、ModManagerPage、ArtifactPanel 都订阅同项目更新；VerificationCard 保存/确认/拒绝后失效原项目。往返切换及卸载后旧请求不会写回。17 沿用入口 |

事件映射：artifact、exit、source_changed、changes_updated 更新工作文件并读取交付项目投影；agent_settled 读取交付项目投影；project_memory_updated 和 feedback_draft_updated 分别交给记忆/反馈能力。原生 client 的过滤链保持不变。能力失效使用按项目键跟踪的 Map，同项目多个视图共享，其他项目不因整个容器替换而刷新。

版本操作的两个 updateProject 回调统一接入 deliveries.update；InstallDialog 的预览/安装后读取接入 deliveries.refresh，卸载后仍更新发起项目。已移除 InstallDialog → DeliveryCard → MessageTimeline → ChatPage 的 installed 刷新通知链，避免安装结果反向读取当前项目。targetSelected 仍是原项目目录展示事件。

刷新失败保留在能力 error 中；交付错误同时返回原 runtime.error，ContextPanel 产物标签可见。陈旧读取失败不覆盖较新的结果/错误。文件差异、反馈与聊天任务读取继续按身份和请求代次丢弃旧结果。四类旧 session revision 已无声明、写入或消费者；client/runtimes/stateFor 等兼容导出仍留待 07。

验证：以既有 sessionRuntime/client/版本操作边界运行 27 个定向用例，覆盖后台反馈原项目、多视图共享、交付并发旧结果及错误归属、工作文件/记忆更新、往返切换和卸载的反馈旧结果保护；完整 vue-tsc --noEmit 和本次变更文件 ESLint 另行记录。原生 IPC 与事件监听使用替身，未运行应用、模型、游戏或打包；不等同真实桌面交互验收。

## 07 收回旧接口及能力归位

- 会话选择和存续归 `features/sessions/useSessionStore.ts`；单会话行为、待发送队列、待确认输入及会话管理弹窗整体归 sessions。聊天消息投影 reducer 归 chat，传输 client 与运行时协议仍归 services/pi。
- 删除公开 client、runtimes、selectedKey、stateFor、uiRequest、send、flush、refreshStats 及可写 computed。stateFor 仅作 store 内部身份查找；页面继续使用业务操作和只读摘要。
- runtime 的消息、任务、模型、附件、待确认输入及队列字段均为深只读投影。队列仅公开状态，不再暴露 load/add/deliver 等内部方法；输入通过 submitInput/editFollowup/operateFollowup/restorePendingInput 管理。附件入口复制传入记录，避免调用者保留可写引用。
- 模型刷新失败由所属 runtime 记录，保持切换后的原会话归属；公开恢复和提交行为不变。聊天展示 props 接受深只读输入，未改变协议 DTO 的写入语义。
- 旧测试通过公开 open 和既有原生/持久化替身准备恢复状态，不再写 runtime 内部字段；测试发现规则覆盖 sessions 和 chat 新路径。历史计划中的原文件路径保留为当时记录。

日志完整视图归 features/chat/SessionLogsPanel.vue；ContextPanel 只传入可见条件并组合，force-mount 保留原日志 composable 的存续与切换刷新行为。07 定向 runtime/client/reducer 25 项通过；未运行桌面应用、模型、游戏或打包。

## 整合契约补正

会话元数据、历史、任务列表、创建/重命名/删除、steering、提交确认、思考强度及日志读取/打开由 `features/sessions/service.ts` 绑定命令参数与结果；消息链接操作归 `features/chat/service.ts`。ChatPage、SessionManageDialog、useSessionLogs、MessageParts 与 sessionRuntime 不再自行指定 nativeCall 返回类型。传输层 client 继续拥有 RPC 通信，不因服务归位建立第二套会话运行时。SubmissionResult.error 接受 Rust Option<String> 实际序列化的 null；这是类型准确性补正，不改变协议或运行行为。
