> 历史档案：保留原日期、计划与验证限制，不代表当前实现或本轮验收。当前入口见[文档首页](../../../README.md)。

# 项目开发上下文分项执行计划

> **面向 AI 代理的执行者：** 使用 executing-plans；先完成[总计划](2026-10-19-gen-mod-improvement-plan.md)执行前检查点。

**目标：** 保存项目环境，在每个任务开始时生成不可变快照并交给 AI 读取。

**架构：** 独立 Rust 模块和 SQLite 表；任务绑定 snapshotId，环境编辑不改运行中任务。只读模型工具通过现有 bridge 取得当前任务快照。

**技术栈：** 现有 rusqlite、serde、Tauri、Vue、pi 扩展；无新增依赖。

**规格：** [整体设计第 7.1、9–11 节](../specs/2026-10-19-gen-mod-improvement-design.md#71-项目开发上下文)。

**执行状态：** 已按完整范围实现并完成限定检查；此文保留原始任务合同。逐项结果、实现调整及未运行的应用/游戏验收见 [执行记录](2026-10-19-execution-progress.md)。

## 文件

- 创建：`src/contracts/project-context.ts`、`src-tauri/src/project_context/{mod.rs,models.rs,repository.rs,tests.rs}`。
- 创建：`src-tauri/src/storage/migrations/003_project_context.sql`、`runtime/extensions/project-context.ts`、`src/features/mods/ProjectContextDialog.vue`。
- 修改：`src-tauri/src/storage/database.rs`、`lib.rs`、`pi/tasks.rs`、`pi/bridge.rs`、`src/contracts/task.ts`、`src/services/native.ts`、`src/features/mods/ProjectPage.vue`、`runtime/extensions/commands.ts`、`mod-tools.ts`。

迁移编号按总计划顺序；文件及 command 名为此分项合同，执行前核对同名文件不存在。

## 任务 1：合同、表与冲突行为

- [ ] 建立如下 TypeScript 合同，Rust 使用对应 serde camelCase 结构：

```typescript
export interface ContextField {
  value: string
  source: 'user' | 'inspection'
  sourceId?: string
  observedAt: string
}
export interface ProjectContext {
  modId: string
  revision: number
  gameVersion?: ContextField
  referenceSets: { id: string, directory: string, files: { path: string, sha256: string }[] }[]
  prerequisites: { name: string, version?: string }[]
  confirmedRequirements: string[]
}
export interface ContextSnapshot {
  snapshotId: string
  modId: string
  taskId: string
  createdAt: string
  context: ProjectContext
}
export interface ContextProposal {
  proposalId: string
  modId: string
  expectedContextRevision: number
  status: 'pending' | 'applied' | 'rejected'
  changes: (
    | { field: 'gameVersion', reason: string, value: string }
    | { field: 'prerequisites', reason: string, value: ProjectContext['prerequisites'] }
    | { field: 'confirmedRequirements', reason: string, value: string[] }
  )[]
}
```

- [ ] 迁移创建 `project_contexts(mod_id TEXT PRIMARY KEY REFERENCES projects(mod_id), revision INTEGER NOT NULL, payload TEXT NOT NULL)`、`context_snapshots(snapshot_id TEXT PRIMARY KEY, mod_id TEXT NOT NULL REFERENCES projects(mod_id), task_id TEXT NOT NULL UNIQUE, payload TEXT NOT NULL)`、`context_proposals(proposal_id TEXT PRIMARY KEY, mod_id TEXT NOT NULL REFERENCES projects(mod_id), status TEXT NOT NULL, payload TEXT NOT NULL)`。snapshot 在任务落库前生成，task_id 此表不引用尚未插入的 tasks 行。
- [ ] 纯仓储接口固定为 `read_on(db, mod_id)`、`save_on(db, value, expected_revision)`、`snapshot_on(db, mod_id, task_id, snapshot_id, created_at)`、`snapshot_for_task_on(db, mod_id, task_id)`，都返回 Result；save 使用条件 UPDATE 的受影响行数判断冲突，首次创建只接受 revision=0。
- [ ] 在 `tests.rs` 定义真实 SQLite 内存夹具并先验证关键行为，获准后运行红绿循环：

```rust
#[test]
fn saving_live_context_does_not_rewrite_snapshot() {
    let db = context_fixture(); // 创建 projects 与本分项两张表，返回 Connection
    let original = empty_context("project-a"); // reference_sets/prerequisites/requirements 均为空
    save_on(&db, &original, 0).unwrap();
    let frozen = snapshot_on(&db, "project-a", "task-a", "snapshot-a", "2026-10-19T00:00:00Z").unwrap();
    let mut changed = read_on(&db, "project-a").unwrap();
    changed.confirmed_requirements.push("只修改显示文字".into());
    save_on(&db, &changed, changed.revision).unwrap();
    assert!(snapshot_for_task_on(&db, "project-a", "task-a").unwrap().context.confirmed_requirements.is_empty());
    assert!(read_on(&db, "project-a").unwrap().revision > frozen.context.revision);
}
```

夹具与 empty_context 放在同一 tests.rs，明确创建表和空集合，不依赖真实用户数据库。补充过期 revision、跨项目读取、同 taskId 不同项目冲突与重复快照幂等用例。

## 任务 2：任务绑定与原生读取

- [ ] 在任务准备获得稳定 taskId 后创建快照；TaskContext 新增可缺省 `contextSnapshotId`，旧任务不含此字段仍可读取。
- [ ] 创建失败时不启动模型。重复准备同任务返回已有快照，不基于最新环境重建。
- [ ] 原生命令：`get_project_context(modId)`、`save_project_context(modId, expectedRevision, context)`；验证项目、有效游戏能力和选中参考目录，无参考目录允许空集合。
- [ ] 保存参考目录时只核对当前需求相关文件，不遍历整盘；记录指纹只说明保存时状态。构建前重核实际使用的参考指纹，发生变化则要求更新环境并启动新任务，不能假定 payload 快照冻结了外部文件。

## 任务 3：工具与界面

- [ ] 扩展注册 `read_project_context` 与 `propose_project_context_update`，不接受模型传入 modId/taskId。读取返回当前任务快照；建议写入 context_proposals，不直接修改 ProjectContext。Rust 以 field 标记的枚举校验字符串、前置数组或需求数组，不允许模型建议外部目录路径。
- [ ] 新请求标题固定 `gen-mod:read-project-context`、`gen-mod:propose-project-context`；bridge 校验当前 generation/task，再调用业务模块。两工具加入任务 `--tools` 白名单。
- [ ] UI 命令 `apply_context_proposal(modId,proposalId,expectedContextRevision)` 在同一短事务内条件更新上下文并标记 applied；`reject_context_proposal` 只改变候选状态。采用后的字段 source=user，保留建议来源供显示，不能伪造工具检测记录。
- [ ] `ProjectContextDialog` 复用 Dialog/Field/Input/Button，允许补充和保存、查看并采纳环境建议；冲突保留输入并要求重载，关闭不影响活动任务。
- [ ] 编辑保存只影响下一任务；聊天展示使用的环境快照摘要，不增加玩家每轮确认完整技术表的步骤。

## 任务 4：核对与交付

- [ ] 获准后运行 `cargo test --lib project_context::tests:: -- --nocapture`（目录 `src-tauri`），只使用内存库。
- [ ] 对实际修改 TS/Vue 文件运行限定 ESLint。全前端类型检查、应用迁移和运行另取对应范围授权。
- [ ] 经授权同步便携扩展与哈希；记录旧项目读写、环境修改后新任务更新、旧任务仍读旧快照的应用验收步骤。
- [ ] `codegraph sync`、`codegraph status`；不自动提交。

通过标准是快照与当前环境可区分、跨项目请求被拒绝、过期编辑不覆盖、旧记录缺省兼容；没有应用验收时明确仅静态/单元范围结论。
