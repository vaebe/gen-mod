> 历史档案：保留原日期、计划与验证限制，不代表当前实现或本轮验收。当前入口见[文档首页](../../../README.md)。

# 跨项目长期记忆实现计划

> **面向 AI 代理的工作者：** 使用 executing-plans，继承[总计划](2026-10-20-gen-mod-product-correction-plan.md)。本文件保留实施合同，勾选反映已接入实现和已运行检查；复合验收未完全覆盖时保留未勾选。

**实施记录：** [本轮实现与验证](../../verification/2026-10-20-product-correction.md)。代码接入、纯夹具检查和实际应用验收分别记录，未勾选项不可视为已验收。

**目标：** AI 自动保存开发经验并提示；用户在设置页查看、修改、删除；后续相关项目复用，运行时不修改 Skills。

**架构：** 复用 knowledge 模块及 SQLite；解除来源项目的外键拥有关系。来源、自动保存键、用户维护标记及历史使用副本用于去重和追溯，任务按游戏、版本和关键词取有界参考数据。

**技术栈：** 现有 rusqlite、serde、sha2、Vue、pi；不引入文件记忆库、向量服务或依赖。

**规格：** [设计第 5 节](../specs/2026-10-20-gen-mod-product-correction-design.md#5-只读-skills-与可编辑的跨项目长期记忆)。

## 全局约束与文件职责

- 保留旧 noteId、状态、allowReuse 与来源；旧 candidate/private 不自动启用。
- 修改 `src-tauri/src/knowledge/{models.rs,repository.rs,search.rs,mod.rs,tests.rs}`、`src/features/knowledge/types.ts`。
- 新增 `src-tauri/src/knowledge/{evidence.rs,usage.rs}`：来源核查和任务实际使用的独立副本。
- 创建迁移 `src-tauri/src/storage/migrations/009_memory.sql`，修改 `storage/database.rs` 注册及迁移测试。
- 修改 `runtime/extensions/{knowledge.ts,mod-tools.ts}`、`src-tauri/src/pi/{task_tools.rs,bridge.rs}`。
- 新增 `src/features/knowledge/{LongTermMemorySettings.vue,MemoryEntryEditor.vue,useLongTermMemory.ts}`；旧 KnowledgeNoteDialog 在总计划 T12 撤下。
- 修改 `src/features/settings/SettingsPage.vue`、`src/features/chat/ToolActivity.vue`、`src/services/pi/{reducer.ts,sessionRuntime.ts}`、`src/contracts/session.ts`。
- 新增 `src/services/pi/reducer.test.ts` 的记忆回执回归；文件若已存在则增量修改。

## T3：独立于项目的存储、迁移和用户维护

- [x] **步骤 1：固定兼容合同。** KnowledgeNote 保留既有字段并扩展 originModId、originProjectName、userEdited、memoryKey、createdAt/updatedAt；旧记录时间缺省显示“历史记录”，不补造发生时间。

```typescript
export interface MemorySourceRef {
  kind: 'verification' | 'reference' | 'command' | 'user_message' | 'manual'
  id: string
  taskId?: string
  sessionId?: string
  sha256?: string
}
export interface MemorySaveResult {
  outcome: 'saved' | 'unchanged' | 'user_managed'
  noteId: string
  revision: number
  title: string
}
```

versionScope 扩展 `unknown`，其 gameVersion 缺省；没有版本依据的开发经验允许保存，但检索结果标记 needsReview，不把未知版本变成 independent。evidenceKind 扩展 development/manual；sourceRefs 使用上述合同。status 扩展 deleted。gameId=`*` 只供用户在设置中明确选择“通用”，模型只能为当前游戏保存。

- [x] **步骤 2：建立 009 迁移。** 保留 006 SQL 不变，009 在一个已有外键开启的事务内执行下列结构；为新表建新索引，旧索引随旧表移除后不重名：

```sql
CREATE TABLE knowledge_notes_next(
 note_id TEXT PRIMARY KEY,
 origin_mod_id TEXT,
 game_id TEXT NOT NULL,
 revision INTEGER NOT NULL,
 status TEXT NOT NULL,
 allow_reuse INTEGER NOT NULL,
 user_edited INTEGER NOT NULL DEFAULT 0,
 memory_key TEXT NOT NULL UNIQUE,
 payload TEXT NOT NULL,
 created_at TEXT,
 updated_at TEXT
);
INSERT INTO knowledge_notes_next
 (note_id,origin_mod_id,game_id,revision,status,allow_reuse,user_edited,memory_key,payload)
 SELECT note_id,mod_id,game_id,revision,status,
   COALESCE(json_extract(payload,'$.allowReuse'),0),1,'legacy:'||note_id,payload
 FROM knowledge_notes;
DROP TABLE knowledge_notes;
ALTER TABLE knowledge_notes_next RENAME TO knowledge_notes;
CREATE INDEX knowledge_reuse ON knowledge_notes(game_id,status,allow_reuse);
CREATE TABLE memory_content_links(
 game_id TEXT NOT NULL,
 content_sha256 TEXT NOT NULL,
 note_id TEXT NOT NULL REFERENCES knowledge_notes(note_id),
 PRIMARY KEY(game_id,content_sha256,note_id)
);
CREATE TABLE task_memory_uses(
 task_id TEXT NOT NULL REFERENCES tasks(task_id),
 mod_id TEXT NOT NULL,
 note_id TEXT NOT NULL,
 revision INTEGER NOT NULL,
 payload TEXT NOT NULL,
 PRIMARY KEY(task_id,note_id,revision)
);
CREATE INDEX memory_uses_project ON task_memory_uses(mod_id);
```

origin_mod_id 为来源标识而非外键；task_memory_uses 不引用可删除的 note 行，保存实际使用的内容副本。读取将列中的归属、状态、revision、维护标记合并为 DTO，不能继续依赖旧 payload 自报状态；写入同步列与 payload。

旧版允许用户修改正文，却未记录维护标记，因此所有 legacy 条目保守迁移为 userEdited=true，原状态、allowReuse、正文及来源保持原样；不因首次核对来源就转交 AI 更新。旧 `legacy:<id>` 身份保留，不猜测其 topicKey，也不自动合并旧条目。009 的 SQL 执行后、登记迁移版本和提交之前，由 Rust `backfill_content_links_on(tx)` 解码旧正文并调用 `remember_content_on(tx,game_id,note_id,content)` 填充指纹关联；哈希不依赖 SQLite 扩展，任一步失败回滚整个 009。

正文规范化仅统一 CRLF 为 LF 并去除首尾空白，然后 SHA-256；不改变代码缩进、不做语义分析。memory_content_links 只存游戏、指纹和条目身份，保存各次已存正文的关联，不保存撤回正文。它属于独立长期记忆，不随来源项目删除。

- [x] **步骤 3：写迁移与 CAS 测试。** database tests 复用 001–008 真迁移内存夹具，插入 active+reuse、candidate、archived、active+private 四种旧记录，其中一条使用旧 UI 修改后的正文，执行 009 及 Rust 指纹回填。验证数量、noteId、旧状态/正文/来源一致，全部 userEdited=true，正文指纹存在，foreign_key_check 无行。再以旧 noteId/revision 发起自动更新，必须返回 user_managed 且原文不变；新来源和新 topicKey 重存同一旧正文也不得创建新条目。repository 定义并测试：

```rust
pub fn edit_on(db: &mut rusqlite::Connection, note_id: &str, expected: i64,
    title: &str, content: &str) -> Result<super::models::KnowledgeNote, String>;
pub fn set_status_on(db: &mut rusqlite::Connection, note_id: &str,
    expected: i64, status: &str) -> Result<(), String>;
```

纯测试在 tests.rs 内建最小 `Connection::open_in_memory()` 及 001、006、009 表，通过 SQL 参数插入完整旧 KnowledgeNote payload，并执行同一 Rust 指纹回填，再调用上述方法。断言第一次 edit revision+1/userEdited=true，第二次使用旧 revision 返回冲突且正文不变；删除条目后 list/search 不返回，memory_key 及正文指纹关联保留。

- [x] **步骤 4：实现用户编辑、停用与删除。** UI 手动新增为 manual 来源，memory_key 使用原生生成的 `manual:<noteId>`，不接受 UI 自报身份；编辑不伪装旧 evidenceKind 仍已验证，保留来源链但标记用户维护。在同一事务内关联编辑前、编辑后正文指纹；修改适用游戏时保留旧游戏关联并增加新关联，不用用户修改后的标题重建身份。删除前先登记当前正文指纹，再清除活动条目正文及非必要个人信息，只保留 noteId、memory_key、deleted 状态、必要 game_id/revision 和防恢复所需最小指纹关联；旧任务副本按历史规则保留。停用为 archived，可由用户明确再次启用；所有用户状态变更也置 userEdited=true，启用不能把维护权交还 AI。用户修改后的自动保存不覆盖、不自动启用，也不清除 userEdited 标记。
- [x] **步骤 5：注册 UI command。** `list_long_term_memories(query,status,page,pageSize)` 返回分页及 total（pageSize 最大 50）；`get_long_term_memory(noteId)`；`save_long_term_memory_entry(noteId?,expectedRevision?,input)`；`set_long_term_memory_status(noteId,expectedRevision,status)`。手动写入只取允许的标题、内容、适用范围，不接受伪造的 userEdited/sourceRefs；原生自动补来源。
- [x] **步骤 6：获准后运行 T3 的 knowledge 和 database filter。** 删除来源项目最小 fixture 行后记忆仍可读取，证明不是仅改 UI 名字；SQLite 迁移失败必须回滚，不留下 _next 表或迁移版本号。

## T4：真实来源、自动保存、检索和使用依据

- [x] **步骤 1：实现 evidence.rs。** 新增 `validate_sources(app,task,refs)`，按真实当前任务/项目核查：verification 读取本项目已确认记录；reference 对照当前 Skills 资料 ID 和哈希；command 对照 TaskRecord 的真实 receipt/taskId/commandId，不接受模型自报成功；user_message 对照当前会话真实 entry 与 digest。manual 只允许 UI，不进入模型 schema。只保存短来源摘要，不复制整个聊天、日志或密钥。

来源存在只证明可追溯，不证明 AI 自然语言总结一定正确；提示词要求有歧义或无证据时不写成确定 API 事实。命令失败也可以作为排查观察来源，但记忆必须保留失败事实而不是声称修复通过。

- [x] **步骤 2：定义自动工具参数和原生函数。** `save_long_term_memory` 只接受当前游戏的方法，不能选择其他项目/global、status、allowReuse 或 userEdited。

```typescript
interface SaveLongTermMemoryInput {
  noteId?: string
  expectedRevision?: number
  topicKey: string
  title: string
  content: string
  versionScope: 'exact' | 'independent' | 'unknown'
  gameVersion?: string
  evidenceKind: 'reference' | 'user_observation' | 'development'
  sourceRefs: MemorySourceRef[]
}
```

实现 `save_for_task(app,task,input,cancelled)->MemorySaveResult`，按以下顺序在同一 Immediate 短事务内处理身份与写入：

1. `memory_key` 取 JSON 数组 `[gameId,规范化的首次topicKey,versionScope,exact时的gameVersion或null]` 的 SHA-256；topicKey 去首尾空白、统一小写，不能为空。来源 ID、taskId、标题均不参与身份。新的 receipt、消息或验收记录只能补充证据，不能使同一主题自动变成新知识。
2. 有 noteId 时要求 expectedRevision，按原身份查找；不存在不允许“更新式创建”。同时按稳定键和规范化正文指纹查找关联条目，范围是当前 gameId 及用户设置的 `*`。任一路径命中 userEdited、deleted 或 archived，优先返回 user_managed，不写正文、不建替代条目；删除条目不向工具返回已清除的标题或正文。
3. 跨来源、跨 topicKey 的相同正文通过 memory_content_links 关联，不能绕过用户保护。对于未受用户维护的 active 条目，仅在适用版本范围相同时以相同正文返回 unchanged；不同精确版本保留独立身份，不能误合并后导致新版本检索无结果。用户维护与撤回检查仍跨这些正文关联执行。同主题改正文或补充证据必须先取得现有 noteId/revision，使用 CAS 更新，不能以省略 noteId 覆盖。有多个同范围不一致身份时返回冲突，不猜测合并。
4. 只有以上检查均未命中现有知识，才创建 active+allowReuse=true、userEdited=false 的条目；originModId 从 task 绑定。每次保存均在同一事务记正文指纹关联，旧关联保留；同键或同正文并发创建由唯一键及事务保证不重复。

上述是确定性身份和同正文保护，不声称能够识别任意语义改写。Skill 明确禁止通过更换主题或改写已知撤回内容规避用户决定；被工具提示 user_managed 后只说明保留用户版本，不自行换键重试。用户手动新增是明确操作，可以保存自己的内容，但不会解除旧条目的 AI 写保护。

exact 的版本必须与核对后的真实来源版本一致，不能仅信模型填的 gameVersion；当前任务的参考资料来源可采用其项目快照版本，玩家验收采用对应记录版本，未知就保存 unknown。事务提交前再次检查任务仍 preparing/running 中允许的写阶段、当前项目未归档/删除和取消标记；实际模型工具只在 running 阶段写入。手动设置 gameId=`*` 时仅允许 independent，且标记 manual/userEdited。

- [x] **步骤 3：写自动保存行为测试。** fixture 覆盖同主题跨 task、跨 commandId/消息来源仍只一条；用户编辑后，原正文及新正文用新来源、不同 topicKey 再保存均返回 user_managed；删除或停用后，以新 receipt、省略 noteId 和改 topicKey 重存同一正文不能复活。再测用户改标题、适用范围不丢旧保护指纹，同正文并发不重复，伪造消息 digest、其他项目 receipt、manual 来源、迟到取消均拒绝，无成功回执。补充合法 AI 更新必须提供现有 noteId/expectedRevision、旧 revision 冲突及 legacy 条目不自动转交维护权。
- [x] **步骤 4：改造检索与 usage。** 继续关键词排序，gameId 当前游戏或用户手动建立的 `*`；精确版本优先，独立方法其次，unknown 标 needsReview；版本未知不匹配 exact。只返回 active+allowReuse，至多 5 条，每条至多 2000 字符，固定 noteId tie-break。参考文件哈希变化标 needsReview。返回的每个条目在同一短事务写 task_memory_uses，记录当时 revision 和实际返回正文；不因当前 note 后来改变而覆盖副本。
- [x] **步骤 5：来源项目删除后仍可检索。** originModId 查无项目时只标来源已移除；不能为了补证据再打开已经删除的 logs/JSONL 或将项目私有上下文夹带给其他项目。保留条目自身的短知识和来源摘要。
- [x] **步骤 6：接入任务工具。** knowledge.ts 注册 save_long_term_memory，task_tools expected_tool 对应 `gen-mod:save-long-term-memory`，bridge 工具白名单同步；比较原始工具 input 和转发 payload，绑定 generation/cancelled。撤下 propose_knowledge，不以旧名字执行新自动保存语义。成功 details 包含 `memorySave:MemorySaveResult`；失败抛错，绝不包装成 saved。
- [x] **步骤 7：获准后运行 knowledge filter。** 检索测试含旧候选/private、其他游戏、旧版本、通用手动条目、unknown、删除条目与 limit；测试调用任务历史副本读取来核对用户编辑未改变旧任务依据。

## T5：设置页管理与可信保存提示

- [x] **步骤 1：设置页增加长期记忆内容区。** 保留当前模型配置的 draft/baseline/保存逻辑，在 SettingsPage 增加 Tabs：模型服务、长期记忆。新 LongTermMemorySettings 使用原生分页，Input 搜索、Select 游戏/状态，Empty 和错误重试。切 Tab 不丢尚未保存的模型配置或记忆编辑输入。
- [x] **步骤 2：编辑器只突出自然语言内容。** MemoryEntryEditor 用 FieldGroup/Field/Input/Textarea/Button，标题与正文可改；适用游戏、版本和来源作为简要说明，适用范围修改可展开。手动新增可选通用，不要求 route、证据枚举或技术 ID。保存失败保留输入，revision 冲突明确显示重载且不自动覆盖；删除、停用是用户操作，不经 AI 再确认。
- [x] **步骤 3：增加聊天直达。** `/settings?tab=memory&memoryId=<noteId>` 驱动设置 Tab 和选中条目，校验条目仍可见；已删除时显示已移除，不误打开其他条目。工具成功提示使用真实 memorySave；Button 点击通过 router.push，不让 Markdown 的 open_message_link 把内部路由当磁盘链接。
- [x] **步骤 4：保留实时及历史 details。** reducer 当前实时 tool output 有 result.details，但历史 toolResult 只存 message.content。改为同时保留 `{content:message.content,details:message.details}`，在 ToolActivity 识别 memorySave；只在 succeeded 且 outcome=saved 显示“已保存长期记忆”及查看/修改/删除管理入口，unchanged 不刷保存提示，user_managed 显示保留用户维护版本，失败不显示成功。

```typescript
const isSaved = tool.status === 'succeeded'
  && details.memorySave?.outcome === 'saved'
```

details 在 reducer 中进行结构校验，不能根据助手正文“已记住”生成可信提示。其他工具输出保持原格式可查看，不能因记忆改造丢失命令回执展示。

- [x] **步骤 5：先写 reducer 回归并获准后运行。** `node node_modules/vitest/vitest.mjs run src/services/pi/reducer.test.ts`；固定输入为含 memorySave 的实时 tool_execution_end 和历史 toolResult，断言相同 noteId/revision；isError=true 无 saved 提示，普通 read/run_command 输出仍保留。不调用模型、不写实际用户库。
- [x] **步骤 6：前端类型和单文件 Lint 按总计划执行。** 应用验收另覆盖设置页编辑、筛选、删除、错误保留输入及聊天直达；只通过 reducer 单测不能声明这些 UI 已验收。

**检查点：** 新项目能读到适用长期记忆，来源项目删除不删除全局条目；用户能改正文和删除；原来的“候选待采纳”不再是新记忆保存必经步骤；既有正式玩家验收仍要求玩家确认。
