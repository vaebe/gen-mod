> 历史档案：保留原日期、计划与验证限制，不代表当前实现或本轮验收。当前入口见[文档首页](../../../README.md)。

# 用户验收闭环分项执行计划

> **面向 AI 代理的执行者：** 使用 executing-plans，遵循[总计划](2026-10-19-gen-mod-improvement-plan.md)。本分项已包含在用户批准的 P1–P7 范围。

**目标：** 将交付步骤、玩家聊天证据、AI 草稿与玩家确认关联到明确版本，并支持继续修复。

**架构：** P3 的 delivery_metadata 保留版本身份与验收条目；独立反馈模块保存草稿和追加式确认记录。AI 工具只能提出草稿，玩家确认通过正常应用 UI 的 Tauri command。

**技术栈：** 现有 SQLite、Tauri、Vue/shadcn-vue、pi JSONL helper 与扩展。

**规格：** [整体设计第 6、7.2、9–11 节](../specs/2026-10-19-gen-mod-improvement-design.md#72-用户验收闭环)。依赖 P2 与 P3 的身份锚点；不依赖 P3 已完成全部内容规则。

**执行状态：** 已按完整范围实现并完成限定检查；此文保留原始任务合同。逐项结果、实现调整及未运行的应用/游戏验收见 [执行记录](2026-10-19-execution-progress.md)。

## 权限保证范围

原生只在正常 UI/IPC 确认路径创建正式记录，模型工具不提供确认或拒绝命令；模型自填 confirmedAt 无效。此合同不提供 OS 文件隔离：现有命令子进程仍有当前用户权限，不能承诺防御恶意命令直接改 SQLite/JSONL。若用户要求后者，暂停该安全目标并另做隔离设计，不在本分项暗加沙箱。

## 文件

- 创建：`src/contracts/verification.ts`、`src-tauri/src/verification/{mod.rs,models.rs,repository.rs,evidence.rs,tests.rs}`、`src-tauri/src/storage/migrations/005_verification.sql`、`runtime/extensions/verification.ts`。
- 创建：`src/features/verification/VerificationCard.vue`、`VerificationHistory.vue`、`useVerification.ts`。
- 修改：`src-tauri/src/lib.rs`、`storage/database.rs`、`pi/bridge.rs`、`runtime/extensions/mod-tools.ts`、`delivery.ts`、`runtime/helpers/read-session.mjs`、`src/services/pi/sessionRuntime.ts`、`src/services/native.ts`、`src/features/chat/ChatPage.vue`、`MessageTimeline.vue`、`src/features/mods/ArtifactPanel.vue`、`VersionList.vue`。

## 任务 1：固定合同和草稿状态

- [ ] 定义合同，Rust 使用同名含义及 camelCase 序列化：

```typescript
export type VerificationResult = 'passed' | 'partial' | 'failed' | 'unknown'
export interface EvidenceRef { sessionId: string, entryId: string, attachmentIndex?: number, sha256: string }
export interface FeedbackPayload {
  versionId: string
  gameVersion?: string
  observations: string
  result: VerificationResult
  caseResults: { caseId: string, result: VerificationResult, observed: string }[]
  sourceRefs: EvidenceRef[]
}
export interface FeedbackDraft {
  draftId: string
  modId: string
  revision: number
  status: 'pending' | 'confirmed' | 'rejected'
  payload: FeedbackPayload
}
export interface VerificationRecord {
  recordId: string
  modId: string
  versionId: string
  sourceDraftId: string
  confirmedAt: string
  payload: FeedbackPayload
  supersedes?: string
}
```

- [ ] 迁移创建 feedback_drafts：draft_id 主键、mod_id、version_id、revision、status、payload；`(mod_id,version_id)` 引用 delivery_metadata。verification_records 保存 record_id 主键、source_draft_id UNIQUE、mod/version、confirmed_at、payload、supersedes。草稿与记录有显式项目关联，不依赖现有 project.versions 永远保留身份。
- [ ] Rust 仓储接口固定：`propose_on`、`edit_on`、`confirm_on`、`reject_on`、`list_on`；edit/confirm/reject 均接收 expectedRevision。confirm 重复同一草稿与相同内容返回既有记录，内容不同则拒绝，不再插入第二条。
- [ ] 先写过期草稿与跨项目确认测试，获准后跑红绿循环：

```rust
#[test]
fn stale_confirmation_cannot_confirm_newer_draft() {
    let db = verification_fixture(); // 内存库含 projects、delivery_metadata 和本分项两表
    let draft = propose_on(&db, "project-a", "version-a", fixture_feedback()).unwrap();
    edit_on(&db, "project-a", &draft.draft_id, draft.revision, changed_feedback()).unwrap();
    assert!(confirm_on(&db, "project-a", &draft.draft_id, draft.revision).is_err());
    assert!(list_on(&db, "project-a", "version-a").unwrap().records.is_empty());
}
```

fixture_feedback/changed_feedback 在同一测试模块建立合法相同版本、不同观察的 payload；确认字段不属于输入 payload。补充幂等、旧版本删除后历史可读、记录追加更正、新版本不继承通过状态。

## 任务 2：原消息证据与版本选择

- [ ] 在现有只读 helper 上新增可选 evidence 投影模式，保留原三参数调用。返回已选会话的 user 消息 entryId、文本摘要和附件索引/指纹；不创建 Agent、不调用模型、不改 JSONL。
- [ ] 新模型工具 `read_feedback_sources` 只读取当前项目的已选会话；`propose_verification` 接收 FeedbackPayload，原生通过当前进程绑定 modId/taskId，校验 version 锚点、caseId、user 角色、原消息内容和附件指纹。
- [ ] SourceRef 的摘要由原生生成，不采信 AI 提交的“原话”。消息缺失、非用户消息、伪造附件索引均拒绝创建草稿；图片未提供时不补造图片证据。
- [ ] 版本不明时工具返回该项目可选版本的简短身份，聊天要求玩家选择；不得直接用 latestVersionId。玩家明确验证旧版本时允许关联旧锚点。
- [ ] 玩家确认后保留最小证据摘要与指纹；原消息后来删除时显示来源不可打开，但不制造仍能访问原附件的链接。

## 任务 3：交付条目与模型工具集成

- [ ] `prepare_delivery` 增加可选 `verificationCases`，字段采用总计划 cases 合同。caseId 在本版本内唯一，预期描述为空则拒绝该条；无条目的旧运行时仍能登记，显示无结构化步骤。
- [ ] 原生登记将条目写 P3 锚点，与版本登记同事务；AI 交付后的草稿不能修改版本原始验收条目。
- [ ] 新模型请求标题固定 `gen-mod:read-feedback-sources`、`gen-mod:propose-verification`；同时更新扩展注册与任务 --tools 列表。用户确认命令绝不注册为模型工具。
- [ ] 原生保存/编辑草稿后发送 `kind: task, type: feedback_draft_updated`；sessionRuntime 新增 verificationRevision，独立刷新对应项目记录，不挪用已安装状态。

## 任务 4：玩家卡片与继续修复

- [ ] Tauri commands：`list_verification(modId, versionId?)`、`edit_feedback_draft(modId,draftId,expectedRevision,payload)`、`confirm_feedback_draft(modId,draftId,expectedRevision)`、`reject_feedback_draft(modId,draftId,expectedRevision)`。
- [ ] 聊天按 sourceRefs 关联显示 VerificationCard，字段为版本、观察、结果与未验证项。确认/编辑/拒绝均保留输入，冲突要求重新查看。不能仅因 AI 文本说“已确认”就切换卡片状态。
- [ ] 版本详情显示条目与确认历史；列表只显示当前版本的摘要，unknown/无记录为中性标签；“已安装”继续来源于 installedTargets。
- [ ] “继续修复”将明确版本、确认记录和需求填入输入草稿，由玩家发送才调用模型。只读旧会话显示“在新会话继续修复”，创建可写会话后预填，不能覆盖玩家已有未发送输入。

## 任务 5：验证与交付

- [ ] 获准后运行 `cargo test --lib verification::tests:: -- --nocapture`。测试所有确认路径、来源校验和版本归属，不修改真实 JSONL。
- [ ] 静态核对扩展工具白名单、用户 IPC 和 UI 事件归属；限定 ESLint，获准后运行类型检查与资源同步。
- [ ] 应用验收逐项覆盖：玩家拒绝不生正式记录、重复确认仅一条、过期卡片被拒、删除包后记录仍可查看、新版本待验证、重启不自动修复、不发起隐藏模型调用。
- [ ] 真实游戏结果由玩家提供，静态/内存库检查不替代；完成 CodeGraph sync/status，不自动提交。
