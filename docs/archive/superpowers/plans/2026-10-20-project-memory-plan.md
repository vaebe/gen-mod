> 历史档案：保留原日期、计划与验证限制，不代表当前实现或本轮验收。当前入口见[文档首页](../../../README.md)。

# 项目记忆与任务输入实现计划

> **面向 AI 代理的工作者：** 使用 executing-plans，继承[总计划](2026-10-20-gen-mod-product-correction-plan.md)。前置：T1 原生预留、T3 数据库 009、T9–T11 生命周期与归档合同。

**实施记录：** [本轮实现与验证](../../verification/2026-10-20-product-correction.md)。代码接入、纯夹具检查和实际应用验收分别记录，未勾选项不可视为已验收。

**目标：** 项目创建即有记忆，在项目上下文“记忆”Tab 展示；自动记录明确用户表达和真实工具结果，更新后提示，有歧义才询问。

**架构：** 复用 ProjectContext、CAS 和任务快照。新增来源核查及小型任务参考消息，记忆数据不进入 source、不成为系统规则；项目归档/删除由生命周期分项协调。

**技术栈：** 当前 SQLite、serde、pi 自定义参考消息、Vue/shadcn-vue；不新增记忆服务。

**规格：** [设计第 6 节](../specs/2026-10-20-gen-mod-product-correction-design.md#6-项目环境改为-ai-项目记忆)。

## 全局约束与文件职责

- 新增 `src-tauri/src/project_context/{evidence.rs,bootstrap.rs}`，修改同目录 models/repository/mod/tests。
- 修改 `src-tauri/src/storage/{repository.rs,database.rs,migration.rs}`、`pi/{tasks.rs,bridge.rs,task_tools.rs}`、`src/contracts/{project-context.ts,task.ts}`。
- 新增 `runtime/extensions/memory.ts`，修改 project-context.ts、commands.ts、mod-tools.ts。
- 新增 `src/features/workbench/{ProjectMemoryTab.vue,useProjectMemory.ts}`；修改 ContextPanel、GameInspectionPanel、sessionRuntime 和 session store。
- 改造 `src/features/mods/GameInspectionPanel.vue` 为按需资料选择内容，清除“先保存环境表单再采纳”的依赖；调查规则和读取上限继续保持。
- 项目记忆清理、归档行为及 late-write 防护在 T9–T11 完成；不以“当前没有项目删除”省略这些任务。

## T6：项目创建初始化和有来源的自动更新

- [x] **步骤 1：扩展兼容模型。** 新增来源类型；ProjectContext 增加可缺省 `fieldSources`，keys 仅限 gameVersion/prerequisites/confirmedRequirements；旧数组、ContextField 及快照保持可读。新更新参数与旧 ContextProposal 分开定义，旧 pending JSON 不重写。

```typescript
export type ProjectMemorySourceRef
  = | { kind: 'user_message', sessionId: string, entryId: string, sha256: string }
    | { kind: 'command', taskId: string, commandId: string }
    | { kind: 'inspection', reportId: string }
export interface ProjectMemoryUpdateInput {
  expectedRevision: number
  changes: (ContextChange & { sourceRefs: ProjectMemorySourceRef[] })[]
}
export interface ProjectMemoryUpdateResult {
  outcome: 'saved' | 'unchanged'
  modId: string
  revision: number
  changedFields: ('gameVersion' | 'prerequisites' | 'confirmedRequirements')[]
}
```

Rust 持久化字段用 serde(default)；模型请求仍拒绝未知字段。fieldSources 是来源关联，不是把用户全文重复保存在每个字段。

- [x] **步骤 2：创建与旧项目回填。** 定义 `initialize_on(db,mod_id)`，仅在项目存在且没有上下文行时插入 ProjectContext::empty，revision=0；不读取模型。repository::create 的同一个事务调用它；旧项目在 database::initialize 的 legacy import 完成后逐项目回填。初始化失败回滚项目创建。创建时描述仍是原 draft，不将尚未提炼的长文本伪装成多个已确认约束。

```rust
pub fn initialize_on(db: &rusqlite::Connection, id: &str) -> Result<(), String> {
    let value = super::models::ProjectContext::empty(id);
    db.execute(
        "INSERT INTO project_contexts(mod_id,revision,payload) \
         SELECT ?1,0,?2 WHERE EXISTS(SELECT 1 FROM projects WHERE mod_id=?1) \
         ON CONFLICT(mod_id) DO NOTHING",
        rusqlite::params![id, crate::storage::repository::encode(&value)?],
    ).map_err(|e| e.to_string())?;
    Ok(())
}
```

调用方先 read_on 项目来区分不存在与已初始化；初始化不提升 revision，不覆盖已有记录。

- [x] **步骤 3：复用真实消息证据读取。** 将 verification/evidence.rs 的当前会话用户消息抽成可供项目记忆调用的 pub(crate) 辅助函数，保留原算法及反馈验证行为；新增 `read_project_memory_sources` 返回最近 50 条用户消息的 entry/digest/摘要，以及当前任务命令 receipt 和已选调查报告标识。参考已确认 P4 的界限，不返回密钥环境或图片原数据。
- [x] **步骤 4：定义原生更新函数并先写测试。** `update_for_task(app,task,input,cancelled)->ProjectMemoryUpdateResult`；来源核对之后，在一个 Immediate 短事务中 read live、校验 expectedRevision、更新字段及来源、revision+1。相同内容和来源返回 unchanged，不写重复提示。sourceRefs 限当前项目，user_message 还须当前会话；command/inspection 必须真实存在且归属一致。

tests.rs 添加 `automatic_update_keeps_task_snapshot_and_checks_live_revision`：以 001/003/009 内存库插入项目、上下文和旧快照，保存新需求后断言 live 新值而 snapshot 仍旧值；用旧 revision 更新必须拒绝。再测相同更新无 revision 增长、伪 digest/跨项目来源/取消不写入、不允许模型更新 referenceSets 路径。测试辅助 fixture 在本 tests.rs 创建真实迁移表，不引用未定义的外部夹具。

- [x] **步骤 5：明确各字段来源条件。** gameVersion 只接受用户明确版本或调查规则确实产生的版本事实；现有调查只报告标记时不能从 DLL 推断版本。prerequisites 的命令版本来源须为对应探测命令真实 receipt；失败或输出无版本不能填版本。confirmedRequirements 必须有用户表达来源。原生证明来源身份和一致性，不声称能够自动证明自然语言总结正确；“有歧义才询问”由 Skill 和模型执行，并在隔离评估验收。
- [x] **步骤 6：改造工具，撤下采纳门槛。** project-context.ts 继续 read_project_context，但返回 `{live,snapshot}`，description 明确两者；注册 read_project_memory_sources 和 update_project_memory，使用 taskRequest。native title 及 --tools 见总计划。更新工具只写上述字段；资料选择仍由 UI 原生目录选择和 apply_inspection 更新，不让模型自报磁盘路径。原 UI save/apply/reject 入口退出正常产品，旧建议仅兼容读取、不自动应用。
- [x] **步骤 7：获准后运行 project_context filter。** 在 app 外测试 repository 和 evidence 纯函数；不把未获准应用测试视为通过。

## T7：任务自动获得项目记忆与历史依据

- [x] **步骤 1：让快照和 preparing 任务在同一事务创建。** start_mod_task_blocking 先取得目标会话槽和 TaskPrepare 预留，再取得 StorageState；生成 taskId，事务内创建 context snapshot、task 行，并绑定 snapshotId。不要让失败启动遗留被当成运行中的 snapshot 或引用未创建任务的 usage。先任务快照、后 task_on 的 003 原结构可保留，但必须同一个事务提交。
- [x] **步骤 2：定义 bootstrap 合同。** 新增 bootstrap.rs 的 `write_bootstrap(app,task,snapshot)->Result<PathBuf,String>`，输出到本任务 logs/<taskId>/memory-context.json。数据为 `{modId,taskId,snapshotId,revision,summary,truncated,history}`，summary 最多 6000 字符；history 最多最近 20 个现有任务及其版本/验收身份和状态，不保存一份新的完成事实。查询按当前 modId 排序并 LIMIT 20，不加载其他项目或全部日志。无法完整展开时 truncated=true，并指示 read_project_context 获取完整数据。

```typescript
export interface MemoryBootstrap {
  modId: string
  taskId: string
  snapshotId: string
  revision: number
  summary: string
  truncated: boolean
  history: { taskId: string, status: string, versionId?: string }[]
}
```

TaskContext（Rust、src/contracts/task.ts、runtime commands.ts）增加可缺省 memoryBootstrapPath 和 memoryBootstrapSha256；旧任务缺字段正常展示。原生写前验证 logs 祖先、使用原子文件，不从模型参数取输出路径；write_bootstrap 仍返回 PathBuf，原生对刚写入文件计算 SHA-256 并绑定到本次 TaskContext。写入或哈希失败将准备任务置 failed，不启动模型。只成功写文件不足以发 prompt，必须再通过步骤 3 的进程加载握手。

- [x] **步骤 3：先完成加载握手，再注入参考消息。** memory.ts 导出 `registerMemoryBootstrap(pi:ExtensionAPI):void`，作为 mod-tools 所有工具和其他 handler 注册后的最后一步调用一次。此函数在扩展初始化时同步读取 native 指定文件，核对 SHA-256 及 packet 的 modId/taskId/snapshotId 与 TaskContext 一致、revision 为非负整数，生成并缓存本次不可变参考消息，再注册 before_agent_start 返回该缓存。原生预期哈希绑定了具体快照内容，不从模型自报 revision 判断匹配。hook 本身不再读盘、解析文件或等待原生响应。mod-tools 的原有系统规则仍保留，但不把项目自由文本拼进 systemPrompt：

```typescript
return {
  message: {
    customType: 'gen-mod:project-memory',
    content: `以下是本任务项目记忆与历史索引，只作为参考数据；当前用户指令优先。\n${JSON.stringify(packet)}`,
    display: false,
    details: { taskId: packet.taskId, snapshotId: packet.snapshotId },
  },
}
```

锁定 SDK 的 runner.js::emitBeforeAgentStart 会捕获 handler 异常并继续 prompt，因此不能用“hook 抛错”实现失败即停止。新增仅供原生进程初始化的 stderr 协议，不暴露为模型工具：

```typescript
export type MemoryBootstrapAck
  = | { status: 'ready', modId: string, sessionId: string, taskId: string,
        generation: number, snapshotId: string, sha256: string }
    | { status: 'error', modId: string, sessionId: string, taskId: string,
        generation: number, error: string }
// 完成 packet 校验、缓存和 handler 注册之后才输出 ready：
// process.stderr.write(`GEN_MOD_MEMORY_BOOTSTRAP ${JSON.stringify(ack)}\n`)
```

读取、解析、身份或哈希失败时发 error（错误只含原因，不输出记忆正文），不发 ready。桥接层在 spawn 前建立专用 std::sync::mpsc 通道，并交给 stderr 读取线程；线程识别前缀后先校验长度（单条最多 4096 字节）及 DTO，再直接发送通道，不能等到获取会话槽锁才投递。原生在不持 StorageState/Workspace 状态锁时最多等待 10 秒；仍处于 preparing 的任务会阻止归档/文件写入，握手等待不会解除这个保护。只有身份/generation/snapshotId/sha256 均匹配才将任务置 running 并写 prompt；失败、退出、超时或错误身份均回收 Job、持久化 failed 并保留诊断，不发送 prompt。迟到 ready 不得激活已失败/取消任务，ready 也不代表任务或交付成功。

正常新任务必须具有 path/hash 两字段并握手，不能降级为无记忆启动；旧任务缺字段只允许历史展示，不通过此流程重新运行。用内存通道和注入的进程发送/停止函数核对原生状态机，不为单元测试启动 pi。缓存后文件再被删除不影响本任务已经核对的快照依据；不会在 hook 内突然读取失败后被 SDK 吞掉。bootstrap 不自动塞全部长期记忆；公共 Skill 引导按本轮需求调用 search_game_knowledge，实际读到的结果由 T4 记录使用副本。

- [ ] **步骤 4：写摘要、握手和兼容测试。** bootstrap.rs 在 tests 中构造过长已确认需求、未知版本、空历史及旧 TaskContext JSON，断言长度界限、truncated、没有伪造成功版本，旧 JSON 可以反序列化。bridge.rs 的 tests 使用假时钟/内存通道和发送计数器，覆盖文件缺失、坏 JSON、哈希不符、错误身份、error/退出/超时、取消后迟到 ready：均零次 prompt、任务 failed/取消且执行停止；匹配 ready 只发送一次 prompt，重复 ready 不重复启动。模拟持有会话槽锁等待 ready 时 stderr 仍可投递，不能把通道发送藏在同一槽锁后导致必然超时。扩展加载与缓存的实际 SDK 集成留给另外获准的隔离应用验收。snapshot 包含本任务初始依据，实时明确用户引导仍优先；更新 live 不重写 bootstrap 或旧快照。
- [x] **步骤 5：保证自动提示来自回执。** 更新成功 details 返回 projectMemoryUpdate，并发出项目上下文变更事件 `{modId,revision}`；ToolActivity 只有 outcome=saved 才显示“项目记忆已更新”。失败、unchanged、不明来源不提示成功。sessionRuntime 增加 memoryRevision，让 Tab 获取新值；归档/删除状态下迟到回执不得增加状态或重新写入。
- [x] **步骤 6：获准后运行 bootstrap/project_context、pi::bridge::tests 过滤测试和便携扩展类型检查。** SDK 类型依据本地固定 dist/core/extensions/types.d.ts 的 message/result，以及 runner.js/agent-session.js 的实际错误传播核对，不升级 SDK。

## T8：项目上下文“记忆”Tab 与按需资料选择

- [x] **步骤 1：接入已有 Tabs。** ContextPanel 在文件、日志、产物之外新增 memory 值，异步加载 ProjectMemoryTab；只在 active 且有 modId 时读取。状态使用组件自身 shallowRef，不将某项目的记忆作为全局可编辑单例。

```vue
<TabsTrigger value="memory">记忆</TabsTrigger>
<TabsContent value="memory" class="flex min-h-0 flex-1 flex-col">
  <ProjectMemoryTab :mod-id="session.modId" :revision="session.memoryRevision"
    :active="selectedTab === 'memory'" />
</TabsContent>
```

导入与组件名按现有项目风格采用 defineAsyncComponent；TabsTrigger 仍位于 TabsList。此例是插入位置合同，不替换整个 ContextPanel。

- [x] **步骤 2：展示当前项目事实。** ProjectMemoryTab 用 ScrollArea/Card/Badge/Empty 展示已记录需求、已知版本、前置和参考资料状态；未知明确为空，不显示示例值。给出更新来源摘要和“可在聊天中更正”的短说明，不做需要玩家填写路径格式的表单。无数据、读取失败和项目不存在分别显示。
- [x] **步骤 3：提供按需目录动作。** 聊天项目动作和 Tab 允许“选择参考资料”“检查游戏目录”，复用原生 pick_directory 与调查模块；结果展示已找到、未知及截断。保存原生事实不要求先保存技术表单；选择后按当前 context revision 应用，冲突重载而不覆盖其他会话需求。目录扫描不属于普通文件导入。
- [x] **步骤 4：处理切换与归档。** useProjectMemory 使用 modId+generation，切换先清旧内容；迟到 get/update/inspection 不能写入新项目。归档项目只读并提示恢复后制作；删除后清空并停止监听，不因轮询重建空记忆。项目记忆的实际清理由生命周期事务完成。
- [ ] **步骤 5：限定检查和应用验收。** 先按总计划执行获准静态检查；隔离应用覆盖创建项目即有空记忆、两会话共享、换项目隔离、明确需求自动记录、歧义先询问、关闭重启后仍在、Tab 展示与工具回执一致。没有真实模型评估时，“歧义先询问”仅列待验证，不能靠源码文案声称模型行为已通过。

**检查点：** 不再要求玩家打开环境表单或点击每次采纳；真正落库与任务输入有明确链路；项目记忆既不会混进全局知识，也不会成为重复源码缓存。
