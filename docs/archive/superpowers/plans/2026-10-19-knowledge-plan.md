> 历史档案：保留原日期、计划与验证限制，不代表当前实现或本轮验收。当前入口见[文档首页](../../../README.md)。

# 本地经验复用分项执行计划

> **面向 AI 代理的执行者：** 使用 executing-plans，继承[总计划](2026-10-19-gen-mod-improvement-plan.md)授权门槛。

**目标：** 检索有来源的内置资料与玩家主动采纳的本地经验，减少重复调查。

**架构：** 内置参考资料仍由 skills 维护；SQLite 只保存本地独立经验与索引，候选经用户选择可用后才跨项目检索。

**技术栈：** 现有 SQLite、Rust、Vue、pi；无向量库、嵌入服务或云端上传。

**规格：** [整体设计第 7.4、9–11 节](../specs/2026-10-19-gen-mod-improvement-design.md#74-可复用游戏知识)。依赖 P4 的验收来源；内置资料索引可独立于候选采纳实施。

**执行状态：** 已按完整范围实现并完成限定检查；此文保留原始任务合同。逐项结果、实现调整及未运行的应用/游戏验收见 [执行记录](2026-10-19-execution-progress.md)。

## 文件与合同

- 创建：`src/features/knowledge/types.ts`、`src-tauri/src/knowledge/{mod.rs,models.rs,repository.rs,search.rs,tests.rs}`、`src-tauri/src/storage/migrations/006_knowledge.sql`、`runtime/extensions/knowledge.ts`、`src/features/knowledge/KnowledgeNoteDialog.vue`。
- 修改：`lib.rs`、`storage/database.rs`、`pi/bridge.rs`、`runtime/extensions/mod-tools.ts`、`src/services/native.ts`、`src/features/mods/ProjectPage.vue`。

```typescript
export interface KnowledgeNote {
  noteId: string, revision: number, gameId: string
  versionScope: 'exact' | 'independent'
  gameVersion?: string
  route: string, title: string, content: string
  evidenceKind: 'reference' | 'user_observation'
  sourceRefs: { kind: 'verification' | 'reference', id: string }[]
  status: 'candidate' | 'active' | 'archived'
  allowReuse: boolean
}
```

## 任务 1：可追溯笔记及采纳

- [ ] 建表 knowledge_notes(note_id 主键、game_id、revision、status、payload)，对 game_id/status 建索引。仅 candidate 可由 `propose_knowledge` 模型工具创建；activate/archive/edit 使用 UI command 与 expectedRevision。
- [ ] 用户确认的效果只创建 user_observation；reference 类型必须对应已维护资料索引项，模型不能仅引用一条“能用了”改成通用 API 结论。
- [ ] `versionScope=exact` 必须填写游戏版本；当前版本未知时不自动匹配 exact。independent 只用于明确不依赖游戏版本的方法，UI 显示其来源与范围。
- [ ] 用户采纳时明确选择是否允许后续项目复用。candidate、archived 或 allowReuse=false 均不进入模型跨项目结果。

## 任务 2：本地检索与边界

- [ ] 定义 `search_notes(db, game_id, game_version, terms, limit)`；最多返回 5 条，每条内容最多 2000 个字符，优先同游戏同版本，再是明确 independent；不设计未经验证的语义相似度。
- [ ] 参数化 SQL 先筛游戏/状态，再用简单标题/路线/正文关键词排序；固定排序 tie-break 为 noteId，重跑结果可解释。返回来源与差异，不把正文拼成 systemPrompt。
- [ ] 内置资料的索引只存资源相对路径、内容哈希与描述；读取时以当前 skill 实际文件为准，不复制整份 API 手册进数据库。
- [ ] 模型工具 `search_game_knowledge`、`propose_knowledge` 同时加入注册与 --tools；原生验证当前 gameId，禁止工具切换到其他项目的私有资料。active 条目也只能作为资料，不是新增授权。

## 任务 3：有意义的检索测试

- [ ] 在 search.rs 的 tests 构造五条固定笔记并验证隔离，获准后运行：

```rust
#[test]
fn search_excludes_unaccepted_and_wrong_version_notes() {
    let db = knowledge_fixture(); // 内存库及五条笔记：同版本active、旧版active、candidate、archived、其他游戏active
    let results = search_notes(&db, "duckov", Some("build-a"), &["悬浮".into()], 5).unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].note_id, "accepted-build-a");
}
```

fixture 五条均含“悬浮”，同版 active 允许复用，其余按各自原因排除。补测 versionScope independent、limit 截断、无版本时不返回 exact、资料文件哈希变化后标记需核对。

## 任务 4：管理入口与交付

- [ ] 项目入口提供“保存经验候选”和查看相关资料，复用 Dialog/Field/Button，不建立独立社区中心；采纳 UI 展示适用版本、证据类型和原记录链接。
- [ ] 再次保存相同来源候选提示已有条目，不以全部聊天/日志自动累积知识；不把源码可直接查得的事实重复写成长久笔记。
- [ ] 获准后运行 `cargo test --lib knowledge:: -- --nocapture`，限定静态检查并同步扩展资源；记录没有模型实测时的验证缺口。
- [ ] CodeGraph sync/status；不自动贡献上游、不提交。
