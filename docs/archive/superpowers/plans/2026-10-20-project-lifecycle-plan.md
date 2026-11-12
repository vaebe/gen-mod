> 历史档案：保留原日期、计划与验证限制，不代表当前实现或本轮验收。当前入口见[文档首页](../../../README.md)。

# 项目归档、恢复与永久删除实现计划

> **面向 AI 代理的工作者：** 使用 executing-plans，继承[总计划](2026-10-20-gen-mod-product-correction-plan.md)。前置：T1 原生预留、T3 长期记忆解除项目外键。

**实施记录：** [本轮实现与验证](../../verification/2026-10-20-product-correction.md)。代码接入、纯夹具检查和实际应用验收分别记录，未勾选项不可视为已验收。

**目标：** 页面和侧边栏移除项目时，让用户选择可恢复归档或永久删除项目全部私有数据；后者包含项目记忆与文件。

**架构：** SQLite archived 状态控制列表和写入，目录原位保留用于归档恢复；永久删除使用受控同级临时目录、短事务和恢复记录。长期记忆只保留独立知识及来源摘要，不随来源项目删除。

**技术栈：** 现有 Rust/SQLite/文件恢复记录、Vue 和 shadcn-vue，不安装依赖。

**规格：** [设计第 6、8 节及用户的归档/删除补充](../specs/2026-10-20-gen-mod-product-correction-design.md)。

## 全局约束与文件职责

- 创建 `src-tauri/src/project_lifecycle/{mod.rs,repository.rs,recovery.rs,tests.rs}`；不把长删除流程塞进 pi/bridge。
- 创建 `src-tauri/src/storage/migrations/010_project_lifecycle.sql`；修改 storage/{database.rs,repository.rs,migration.rs,models.rs,mods.rs}、pi/{bridge.rs,manager.rs}、lib.rs、src/contracts/mod.ts。
- 修改 `src/services/native.ts`、`src/stores/{projects.ts,session.ts}`、`src/features/{mods/ProjectPage.vue,workbench/ProjectSidebar.vue}`。
- 新增 `src/features/mods/{RemoveProjectDialog.vue,ArchivedProjectsDialog.vue}`。不以“折叠侧边栏分组”冒充归档，不以“删除会话”冒充删项目。
- `VersionList.vue`、`VersionActions.vue` 透传归档只读状态，禁用安装、导出与删除动作；保留详情和目录查看，原生仍独立校验。
- 永久删除仅处理 `mods/<modId>` 及该 modId 私有数据库记录；游戏安装目录和全局长期记忆不归该项目删除操作所有。

## T9：归档、恢复和所有写入口的状态校验

- [x] **步骤 1：添加 010 迁移和 DTO。** 初始化现有项目 archived=0；ModProject.archived serde 默认 false，repository read_on 从列合并，不允许旧整份 payload 保存将归档状态改回。

```sql
ALTER TABLE projects ADD COLUMN archived INTEGER NOT NULL DEFAULT 0;
CREATE TABLE project_removals(
 mod_id TEXT PRIMARY KEY,
 operation_id TEXT NOT NULL UNIQUE,
 removed_at TEXT NOT NULL
);
```

project_removals 不引用 projects，只保留删除过的 ID/操作/时间，防止残留 mod.json 重新导入。它不保存项目记忆、源码或私人聊天。

- [x] **步骤 2：固定原生合同并拆除读取副作用。** `set_mod_archived(modId,expectedRevision,archived)->ModProject`、`list_archived_mods()->ModProject[]`、`delete_mod(modId,expectedRevision)->DeleteProjectResult`。archived=true 通过 CAS 更新 projects.archived/revision；恢复为 false，不换 modId、不重建记忆或版本。list_mods 默认只返回 archived=0；归档列表独立查询。当前 storage/mods.rs::get_mod 会调用 repository::update，不能沿用为只读入口：改为 read_on 后在返回 DTO 上计算游戏标识与会话只读状态，不保存、不增加 revision、不初始化目录。任务启动仍独立验证 Skills 快照，不能靠一次界面读取持久化只读标记才安全。归档历史和处于恢复待处理状态的现存项目均可读取实际已存数据，删除后返回不存在。

```typescript
export interface DeleteProjectResult {
  modId: string
  removed: boolean
  cleanupPending: boolean
  message?: string
}
```

- [ ] **步骤 3：先写归档/恢复仓储测试。** fixture 使用 001–010 真实迁移，插入两个项目、来源长期记忆与上下文。调用 `set_archived_on(db,id,expected,true)`，断言活动列表不含它，归档列表包含，记忆/文件身份不变；反复 get_mod 后 projects/sessions 的 payload、revision 和行数不变，且不创建目录。再恢复，modId/版本/上下文 revision 内容不重建；另一个项目不受影响，旧 revision 失败。归档后的 rename_mod/save_mod_draft/会话偏好写入必须被原生拒绝。
- [x] **步骤 4：实现统一状态入口。** 定义 `has_unresolved_removal_on(db,mod_id)`，判定存在 kind=project_delete 且 status 为 pending 或 requires_review；`ensure_writable_on(db,mod_id)` 要求项目存在、未归档且前述判定为 false。禁止只检查 pending。恢复失败保留这两种阻断状态之一；interrupted 只表示原目录身份和数据库已核对一致、无需继续恢复，不能把恢复失败记成 interrupted。删除墓碑存在时普通写入也必须拒绝，不重新初始化项目。

项目范围的公开写入口及其实际提交点均核对状态：storage/mods.rs 的 rename_mod/save_mod_draft；task start/connect/create session/steering；会话改名、删除及 draft/model/thinking 偏好；source files；项目记忆自动保存；以该项目为来源保存长期记忆；目录调查应用；版本删除/安装/导出。没有 StorageState 的记忆仓储在自身写事务内检查，持锁入口在锁内检查；不能只在进入 command 时检查后释放锁再写。生命周期内部归档/恢复/删除使用自己的 CAS 事务，不能调用会把归档恢复操作本身拒绝的普通写入口。读取归档历史可用，不启动 AI。

测试以同一 fixture 分别设置 pending、requires_review、已安全恢复的 interrupted，断言前两种状态阻止上述普通写入及新任务，第三种才恢复可写。读取历史、列出归档不改变操作状态；UI 不提供绕过恢复冲突的“强制继续”。
- [x] **步骤 5：协调进程与安装操作。** 非任务连接 `connect_pi` 及模型切换等会重启空闲进程的路径必须使用 NativeOperation::ProcessConnect，覆盖状态核对、会话/模型写入、spawn 与注册完成的整个过程，不能在入口检查后释放预留再启动。固定顺序为“目标槽 → try_reserve_native(ProcessConnect) → 短 StorageState/状态校验”，启动进程时释放 StorageState、保留令牌；失败 Drop 释放。已有 TaskPrepare 流程由 preparing 状态覆盖后续任务进程启动；所有 spawn 调用点须明确属于哪一种保护，不存在无保护的空闲重连。

connect_pi 的会话选择先纯读现有 ID，或仅在内存生成新 UUID，然后取得目标槽与预留，最后在短 StorageState 内调用会创建 session 的逻辑；不能为了取得槽 ID 在预留前先写入 sessions。模型切换沿用已知槽及相同锁序。所有创建目录的代码只能位于保护区间内。

归档/删除先取 NativeOperation::ProjectLifecycle，拒绝 preparing/running 和 compacting，不自动取消任务。新增 Registry::for_project(mod_id) 只取目标项目槽的 Arc；取得预留后才扫描、关闭空闲 pi 及后代 Job，持续持令牌到状态提交/目录 rename 完成。此期间 ProcessConnect 立即失败，不能先排队到归档后启动。关闭时不持 StorageState，避免反向等槽。后续 StorageState 使用 try_lock，其他版本/安装/导出操作占用时提示忙，不长等待后悄悄归档/删除。

用 Barrier 在夹具中固定“连接已通过状态检查但尚未 spawn”和“生命周期已取得预留但尚未扫描槽”两个竞争位置，分别验证只能一方成功；失败连接不能生成 logs/sessions，归档后无新进程，删除后不重建原目录。测试使用假的进程工厂计数及 tempdir，不启动真实 pi；另测预留异常释放、恢复后可重新连接及另一项目不受影响。
- [x] **步骤 6：所有文件入口在自身已有 StorageState 锁内重核状态。** installer 的 preview 和 install、registry delete_version、files export_version 在读取项目和文件前检查；不能只在 frontend disabled。它们不需要重新取得 native 预留；生命周期在短 StorageState 临界区完成状态变化和目录 rename，保证旧安装预览不能在归档/删除后执行。已安装到游戏的文件不卸载。
- [ ] **步骤 7：获准后运行 project_lifecycle filter 及受影响 workspace/bridge 夹具。** 包含归档后原生写入拒绝、恢复可继续、get_mod 无副作用、requires_review 阻写、连接与生命周期竞争、关闭空闲进程失败不归档、不同项目不互锁。fixture 不打开游戏或实际安装目录。

## T10：永久删除全部项目私有数据与中断恢复

- [x] **步骤 1：定义删除的精确拥有范围。** 同一短事务按外键子表先后删除：task_memory_uses、verification_records、feedback_drafts、delivery_metadata、context_snapshots、context_proposals、project_contexts、inspection_reports、project_assets、runtime_logs、steering_inputs、session_stats、tasks、sessions、legacy_imports，最后 projects。保留 knowledge_notes 及独立的 memory_content_links，origin_mod_id 只是历史来源；保留本操作 project_delete 的 file_operations，其余本项目 file_operations 在确认无待恢复外部操作后删除。verification 的 supersedes 仅允许同项目，若发现异常跨项目外键则回滚并报告，不删除其他项目弥补。

定义 `delete_private_rows_on(db:&mut Connection,mod_id:&str,operation_id:&str,removed_at:&str)->Result<(),String>`；函数只做上述事务与 project_removals 写入，不进行文件 I/O。测试用参数化 SQL seed 所有子表，并以 foreign_key_check 校验顺序与零遗留。

- [x] **步骤 2：先写不触碰实际项目的删除测试。** tests.rs 使用 tempdir 建模拟 mods/<uuid>/source、logs、artifacts、assets，另建 external-installed/ 和第二项目，并在内存库种长期记忆。执行测试适配器 `remove_fixture_project(db,mods_root,mod_id)`：断言只移除目标项目私有记录及目录；长期记忆、另一项目和 external-installed 原字节仍存在。该适配器调用 production 的路径算法、事务和恢复函数，不另写一套删除算法。
- [x] **步骤 3：原生目录边界核对。** 不接受 UI 提供任意目录；由 modId 得出原始 `mods/<modId>`，临时名固定 `.delete-project-<operationId>`，验证 UUID、解析后的 mods root 和两个目标均在该 root 下。拒绝祖先重解析点。临时目录存在不覆盖，不根据字符串 contains 判断归属。所有递归清理只针对本操作生成的临时目录。
- [x] **步骤 4：实现四阶段删除。**

```text
预留并核对空闲、用户确认对应项目 revision
  → 持短 StorageState，写 project_delete/pending 记录
  → rename 项目目录到同级受控临时目录
  → 短 SQLite 事务删私有记录、写 project_removals；提交
  → 释放 StorageState，清理临时目录，file_operation 置 done
```

这里用户确认来自 RemoveProjectDialog 的明确操作；不额外给模型 delete 工具。执行前检查该项目是否有 pending/requires_review 的安装等外部文件操作：先恢复或报阻塞，不把未处理的外部变更记录随项目删除。已经完成的安装仅保留游戏文件，本地安装历史随项目数据删除。

- [x] **步骤 5：确定失败和恢复矩阵。**

| 重启/失败位置 | 恢复行为 |
| --- | --- |
| pending 已写、目录未改名，project_removals 不存在 | 核对原目录身份与元数据一致后标 interrupted；不自动重试删除；不能核对则 requires_review 并持续阻写 |
| 临时目录存在、项目记录仍在，未写 project_removals | 目标原路径空闲且身份匹配时原位恢复临时目录；冲突不覆盖，requires_review 并禁止继续写 |
| project_removals 已提交、临时目录仍在 | 项目已逻辑删除，只清理此临时目录，不把它恢复为项目 |
| 元数据提交失败 | 回滚事务并尝试恢复原目录；成功核对后才解除阻断，恢复失败明确报告、置 requires_review 并持续阻写 |
| 清理临时目录失败或发现重解析点 | removed=true、cleanupPending=true，明确“项目已移除，文件清理未完成”；不声称全部文件已删除 |

清理使用 symlink_metadata 检查受控目录树，不跟随链接；发现特殊重解析内容即停止并报告待处理，绝不沿链接清理外部目录。仅有 tempdir 的故障夹具测试可以练习这条路径。

- [x] **步骤 6：修正启动顺序与 JSON 重导入。** migrations→app.manage(Database)→project_lifecycle::recover_pending→旧 JSON import→上下文回填/交付锚点/interrupt_previous→其余 reconcile_files。import 扫描 UUID 目录前查 project_removals；在墓碑中则跳过，不让删除失败残留的 mod.json 复活。通用 reconcile_files 对 project_delete 先分派生命周期恢复，不先 read_project。重复恢复应幂等。

逻辑删除成功后关闭并移除该项目的 Pi 槽和 Workspaces 缓存，清掉进程及观察指纹的内存状态；归档只关闭空闲进程、保留项目磁盘数据，不清理记忆。注册表仅短锁删除目标键，不能持注册表锁等待其他会话。
- [ ] **步骤 7：测试每个阶段中断。** 以明确 phase 参数构造记录与磁盘状态，不靠强杀真实应用；验证数据库事务失败回滚、目录冲突不覆盖、requires_review 重启后仍阻止全部普通写入与连接、逻辑删除后不能被 late save/bootstrap/legacy import 复活。只有恢复已完成且身份核对一致才解除阻断。foreign_key_check 失败必须作为失败报告。
- [x] **步骤 8：获准后运行限定测试。** `cargo test --offline --manifest-path src-tauri/Cargo.toml --lib project_lifecycle:: -- --nocapture`。在真实应用中的永久删除验收只允许任务新建的隔离 fixture；不让执行计划成为删除当前真实项目的授权。

## T11：页面、侧边栏与归档管理

- [x] **步骤 1：添加两种移除选项。** RemoveProjectDialog 复用 Dialog、Field、ToggleGroup、Button，默认“从侧边栏移除（归档，可恢复）”，另一项“永久删除项目数据”。明确后者删除本地源码、交付包、聊天、日志、项目记忆与本地安装记录；已安装 MOD 和跨项目长期记忆不删除。最终按钮文本分别为“归档项目”和“永久删除”，不得共用含糊的“确定”。
- [x] **步骤 2：接入项目页和 ProjectSidebar 菜单。** 两处打开同一组件并传稳定 modId/revision；ProjectPage 当前卡片是 RouterLink 外包，增加操作按钮时改成 Card 外壳、内容区域链接，避免 RouterLink 中嵌套交互按钮。项目仍运行时显示忙并不提交，原生再次检查。
- [x] **步骤 3：实现归档入口和恢复。** 项目列表页提供“已归档项目”，ArchivedProjectsDialog 只列归档项目，允许恢复和永久删除。恢复按钮使用当前 revision，原 ID/数据恢复显示；不创建新项目或自动发任务。归档项目仍可只读查看，但所有制作/写操作需恢复后进行。
- [x] **步骤 4：同步 store、路由与会话。** projects store 增加 removeLocal(modId) 并在真实 native 成功后更新；归档与删除当前项目时，先确认后端关闭空闲连接成功，再清理当前 session runtime 和路由到 /mods。其他项目会话不受影响。迟到的 project update 不能把 archived/deleted 项目重新 upsert 到活动列表。

```typescript
function upsert(project: ModProject): void {
  projects.value = project.archived
    ? projects.value.filter(item => item.modId !== project.modId)
    : [project, ...projects.value.filter(item => item.modId !== project.modId)]
}
```

永久删除成功后保留一个本地失效 ID 集合直到列表完整重读完成，或用 generation 丢弃先前请求；不能仅靠一个 upsert 分支防止删除项目的晚响应。

- [x] **步骤 5：失败和取消保留上下文。** 关闭选择框不修改项目；归档失败留在当前项目；永久删除 removed=true 但 cleanupPending=true 时移出列表并显示未清理信息，恢复列表不显示它为可恢复归档。错误消息对应具体阶段，不重复提交删除。
- [ ] **步骤 6：获准应用验收。** 合成项目依次归档、重启、恢复、核对记忆/源码/版本原 ID，再执行永久删除及重启；长期记忆仍在，项目记忆与本地文件移除，另一个项目及外部安装夹具不变。原生测试未运行或 UI 未启动时分别记录，不互相替代。

**检查点：** “从侧边栏移除”和“永久删除数据”语义清楚；归档可恢复，删除不会被旧 JSON 或迟到工具恢复；用户的真实项目和游戏目录未被验收脚本操作。
