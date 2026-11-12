> 历史档案：保留原日期、计划与验证限制，不代表当前实现或本轮验收。当前入口见[文档首页](../../../README.md)。

# 交付内容检查分项执行计划

> **面向 AI 代理的执行者：** 使用 executing-plans，继承[总计划](2026-10-19-gen-mod-improvement-plan.md)约束；测试示例仅在获准后执行。

**目标：** 对实际交付候选文件实施可核查的内容规则，阻止明确不应分发的文件，并保存脱敏报告。

**架构：** Rust 维护决定性检查；扩展先请求预检查，最终登记再复核；报告写入独立 delivery_metadata，不改旧 version.json。

**技术栈：** 现有 Rust/serde/sha2/rusqlite 与 Node 扩展。

**规格：** [整体设计第 7.3、9、11 节](../specs/2026-10-19-gen-mod-improvement-design.md#73-交付内容检查)。依赖 P2 快照中的明确参考文件范围。

**执行状态：** 已按完整范围实现并完成限定检查；此文保留原始任务合同。逐项结果、实现调整及未运行的应用/游戏验收见 [执行记录](2026-10-19-execution-progress.md)。

## 文件

- 创建：`src-tauri/src/artifacts/content_check.rs`、`delivery_metadata.rs`、`src/contracts/delivery.ts`、`src-tauri/src/storage/migrations/004_delivery_metadata.sql`、`runtime/extensions/delivery-check.ts`。
- 修改：`artifacts/mod.rs`、`artifacts/registry.rs`、`pi/bridge.rs`、`storage/database.rs`、`storage/repository.rs`、`runtime/extensions/delivery.ts`、`mod-tools.ts`、`src/features/mods/ArtifactPanel.vue`。

## 任务 1：纯检查合同与规则

- [ ] 定义下面合同；Rust Finding 不含匹配文本，CheckInput 的 bytes 只在检查内存中使用：

```typescript
export interface ContentFinding { code: 'env_file' | 'private_key' | 'known_secret' | 'reference_copy' | 'review_origin'; level: 'block' | 'review'; path: string }
export interface ContentCheckReport { rulesVersion: '1'; findings: ContentFinding[] }
```

Rust 纯函数 `check_bytes(relative: &str, bytes: &[u8], known_secrets: &[String], restricted_hashes: &BTreeSet<String>) -> Vec<ContentFinding>`：文件名为 .env 或以 .env 结尾、明确 PEM 私钥头、当前原生凭据快照中的真实值、受限参考文件哈希命中为 block。无可靠依据的 origin 只记录 review，不用扩展名统一禁止 DLL。

- [ ] 读取已允许的交付候选，先限单文件 64 MiB 文本扫描量；二进制按块做哈希，超过上限的文本不返回“未命中”，而返回明确无法完成检查并阻止本次登记。当前规则不递归展开嵌套压缩包，报告明确检查范围，不能声称全面秘密扫描。
- [ ] 先写如下测试，并补充合法 MOD DLL 不被扩展名误拦截：

```rust
#[test]
fn known_secret_is_blocked_without_returning_secret_text() {
    let secret = "fixture-secret-only".to_string();
    let findings = check_bytes("ExampleMod/config.txt", secret.as_bytes(), &[secret.clone()], &Default::default());
    assert!(findings.iter().any(|item| item.code == "known_secret" && item.level == "block"));
    assert!(!serde_json::to_string(&findings).unwrap().contains(&secret));
}
```

测试使用字符串字段的 Finding；另测 .env、私钥头、参考文件同内容与合法文件。规则版本固定在报告中，历史报告不被新规则重写。

## 任务 2：版本元数据锚点与恢复

- [ ] 创建 `delivery_metadata(mod_id TEXT NOT NULL REFERENCES projects(mod_id), version_id TEXT NOT NULL, task_id TEXT NOT NULL, payload TEXT NOT NULL, PRIMARY KEY(mod_id,version_id))`。
- [ ] 仓储接口：`ensure_anchor_on(db, mod_id, version)`、`save_check_on(db, mod_id, version_id, report)`、`mark_deleted_on(db, mod_id, version_id)`、`read_on(db, mod_id, version_id)`；`version` 使用现有 ArtifactVersion，不接受模型自行声称已登记的 ID。
- [ ] 回填所有现有版本，payload 采用总计划 DeliveryMetadata 合同，cases 为空，不添加通过状态。删除版本前确保身份锚点，实际删除成功后置 artifactDeleted；恢复 delete/register 操作时幂等校正锚点。
- [ ] 报告与 project.versions 的登记元数据在同一数据库事务落库；文件 rename 仍使用原有恢复记录。不要在事务里哈希或复制。

## 任务 3：前置检查与原生重查

- [ ] 在 `prepareDelivery` 创建 staging/files 前，通过 `gen-mod:check-delivery` 请求检查候选 `{source,target}`，原生验证当前 task/tool 身份和路径，返回脱敏报告。
- [ ] 前置 block 不创建 staging；review 提供具体来源核对要求，未满足当前 game skill 的明确交付规则不能登记。
- [ ] Process 在内存保留启动时凭据匹配集合，预检查与最终复核使用同一快照；不把凭据写入请求、报告、数据库或交付文件，也不向扩展回传集合。文件与规则扫描在会话槽锁和数据库事务之外完成，提交前重新验证任务代次。
- [ ] 原生 register 在最终文件/ZIP 哈希验证后，对最终文件复核；禁止使用模型提供的 PASS 作为依据。回执增加 checkReport，界面从锚点读取。
- [ ] 最终复核失败保留当前任务日志和候选文件。明确返回 `requiresNewTask: true`，下一任务按现有 fresh-output 规则重新构建；本分项不删除历史 staging 或伪称支持同轮重试。

## 任务 4：展示与验收

- [ ] 版本详情显示规则版本、检查范围、相对路径与原因；review 与 block 不使用含糊的“全部安全”文案。
- [ ] 获准后运行 `cargo test --lib artifacts::content_check::tests:: -- --nocapture`，并为 delivery_metadata 的旧版本回填、重复登记、删除恢复写对应内存库测试。
- [ ] 对扩展、合同和版本详情做限定静态检查；获准后准备运行时资源，检查便携扩展实际包含前置请求。
- [ ] 更新交付 skill 的真实协议说明、执行 CodeGraph sync/status。未运行真实打包时不声称已拦截实际用户包。
