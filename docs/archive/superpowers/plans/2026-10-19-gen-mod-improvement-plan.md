> 历史档案：保留原日期、计划与验证限制，不代表当前实现或本轮验收。当前入口见[文档首页](../../../README.md)。

# GenMod 能力改进与 Skills 优化执行计划总入口

> **面向 AI 代理的执行者：** 使用 executing-plans 按获批子项目逐任务执行。用户已授权 P1–P7 实现与限定验证，不包含提交、推送或委派实现。

**目标：** 将整体设计拆成可选择、可审查的独立交付，完整覆盖七项能力与 skills 优化。

**架构：** 沿用 Vue/Tauri/pi、SQLite 与项目目录；新增业务分模块，Rust 负责协议校验和持久化，skills 提供任务方法。已登记产物与后续验收记录分开保存。

**技术栈：** 当前 Vue 3、TypeScript、Tauri 2、Rust、rusqlite、pi 1.1.0、Windows PowerShell；本计划不增加依赖。

**规格：** [整体设计草案](../specs/2026-10-19-gen-mod-improvement-design.md)。

**状态：** 用户已确认执行 P1–P7，分支 codex/gen-mod-improvements，允许限定 Rust 测试、Skills 评估、前端类型检查和本地资源同步；不安装依赖或启动应用/游戏，不提交。详见 [执行记录](2026-10-19-execution-progress.md)。

## 1. 子代理审核结论与复核

2026-10-19，由两个只读子代理分别完成需求/skills 和技术可行性审核，主代理复核相关源码。

- 需求/skills 审核：未发现确定缺陷，七项能力、目标用户、游戏范围、反馈方式、本地素材及技能优化均有设计覆盖。
- 技术审核：未发现必须推翻总体架构的阻塞。版本身份锚点、staging 重试限制、资源打包、租约白名单和分发同步均需要在实现任务中明确落实。
- 边界澄清：原生不向 AI 暴露确认接口，可以控制正常应用协议路径；当前 `run_command` 的 `cwd` 检查不等于文件系统隔离，子进程仍有当前用户权限。本文与反馈计划不承诺防御绕过协议直接篡改 SQLite/JSONL，也不擅自引入新沙箱。
- 审核时首期尚未选择，该项当时不判为整体设计缺陷；后续用户已明确选择并授权 P1–P7。

审核为静态核对，没有运行模型、测试、构建、应用或游戏，不证明新能力已可用。

## 2. 全局约束

- 普通玩家为主；只允许已有有效游戏能力创建项目，不自动适配未知游戏。
- 游戏开发不强制安装游戏；需要时使用用户合法提供的独立参考程序集。
- 玩家聊天反馈，AI 只整理草稿；正常应用流程中只有玩家 UI 确认可创建正式验收记录。
- 不设计外部付费生成、云端知识库、自动发布或游戏自动操控。
- 不覆盖现有未提交内容；实现前重新核对 Git 差异。本轮已有六个 skills 变更是优化候选，不是新计划已完成的证明。
- 新增工具同时更新扩展注册、原生分派和 `bridge.rs` 的 `--tools` 白名单，工具名和字段以各分项计划为准。
- SQLite 短事务内不做模型调用、文件复制、哈希大文件或等待进程；跨文件操作沿用恢复记录。
- 修改 source 的动作接入租约与 revision；项目环境、反馈、知识和原素材存储在 source 之外。
- 原生登记继续校验命令、必要脚本、fresh output、文件集、ZIP 和源码 revision，AI 自报结果不替代校验。
- `ArtifactVersion` 与包内 `version.json` 保持原有职责。新关联表保留版本身份，删除包不删除验收历史。
- UI 使用项目现有 shadcn-vue 组件；修改前读取 `.agents/skills/shadcn-vue/SKILL.md` 和适用 Vue 技能。
- 不自动安装/替换依赖、不初始化新仓库、不创建新聊天、不提交或推送。纯计划写作不创建 worktree。

## 3. 分项计划与依赖

| 编号 | 分项计划 | 前置 | 独立交付 |
| --- | --- | --- | --- |
| P1 | [Skills 优化](2026-10-19-skills-quality-plan.md) | 无 | 精简入口、单一资料职责、可审查的评估场景 |
| P2 | [项目开发上下文](2026-10-19-project-context-plan.md) | P1 定义方法边界 | 项目环境、任务快照与读取工具 |
| P3 | [交付内容检查](2026-10-19-delivery-check-plan.md) | P2 的参考目录快照 | 原生决定性检查与脱敏报告 |
| P4 | [用户验收闭环](2026-10-19-verification-plan.md) | P2；与 P3 共享版本元数据锚点 | 版本验收条目、聊天草稿、确认和继续修复 |
| P5 | [本地经验复用](2026-10-19-knowledge-plan.md) | P4 的确认记录；也可先检索内置资料 | 本地候选、人工采纳和有来源检索 |
| P6 | [支持游戏调查](2026-10-19-game-inspection-plan.md) | P2 | 玩家选择范围内的只读调查与采纳 |
| P7 | [本地图片素材](2026-10-19-local-assets-plan.md) | P2、P3；交付协议与 P4 对齐 | 本地导入、处理、原图保留、实际资源交付 |

引擎公共能力由 P1/P6 按真实复用场景覆盖，不预先创建空引擎技能。音频、3D、AssetBundle 转换不在 P7 的图片任务中隐式实现；选择这些能力时另写经审核的适配计划。

建议组合保持与规格一致：质量基础为 P1–P3；制作与反馈闭环为 P1–P4；本地内容制作增加 P5–P7。本次已选择 P1–P7。

## 4. 共享数据与迁移顺序

分项合同使用以下固定名称；被选中后不得在其他计划另造同义实体：

| 表/合同 | 拥有者 | 关键约束 |
| --- | --- | --- |
| `project_contexts`、`context_snapshots`、`context_proposals` | P2 | 项目 revision；任务快照唯一；AI 环境建议须由 UI 采纳 |
| `delivery_metadata` | P3 | `(mod_id, version_id)` 主键；保存历史版本身份、报告、快照与验收条目 |
| `feedback_drafts`、`verification_records` | P4 | 草稿 revision；单次确认幂等；正式记录追加更正 |
| `knowledge_notes` | P5 | 候选/可用/归档；来源与版本显式保留 |
| `inspection_reports` | P6 | 已选目录、读取上限、命中与未知信息 |
| `project_assets` | P7 | 项目与 assetId 归属、相对路径、指纹、处理配方 |

默认按 P2→P3→P4→P5→P6→P7 给新 SQL 迁移分配 003–008。只有获批项目才创建对应迁移；若选择顺序不同，执行前统一重排文件编号和初始化检查，禁止缺失 include_str 文件或重复 schema_migrations 版本号。

`delivery_metadata` 的逻辑合同：

```typescript
export interface DeliveryMetadata {
  modId: string
  versionId: string
  taskId: string
  contextSnapshotId?: string
  createdAt: string
  artifactDeleted: boolean
  cases: { id: string, precondition: string, action: string, expected: string, evidenceHint: string }[]
  checkReport?: { rulesVersion: string, findings: { code: string, level: 'block' | 'review', path: string }[] }
}
```

P3 创建锚点与 JSON payload，`contextSnapshotId`、`checkReport` 可缺省、`cases` 必须为数组；P4 复用 cases，不创建第二张“验收版本”表。旧版本从现有真实版本记录回填锚点，cases 为空；不得替旧版本生成通过结果。删除包前确保锚点存在，删除流程成功后置 artifactDeleted，恢复分支同步这一状态。

## 5. 跨分项集成检查单

- [ ] 新模块在 `src-tauri/src/lib.rs` 声明并注册所需 Tauri commands。
- [ ] 迁移在 `storage/database.rs` 按已选编号、同一事务执行并登记；旧 payload 缺省兼容。
- [ ] 模型工具在 `runtime/extensions/mod-tools.ts` 注册，并在 `pi/bridge.rs` 的任务工具列表启用；用户确认命令不进入工具列表。
- [ ] 新原生模型请求在业务模块处理，`bridge.rs` 只保留身份绑定与分派，沿用 generation/taskId/toolCallId 校验。
- [ ] 新文件操作种类在 `storage/repository.rs` 恢复分派中有明确处理器，不依赖未知 kind 的 requires_review 兜底冒充成功恢复。
- [ ] `materialize_asset` 接入 `pi/workspace.rs` 的写操作白名单和外部变更通知；不得只包一层扩展而不更新原生白名单。
- [ ] 修改便携扩展、helper 或 skills 后，经允许运行现有资源准备脚本，同步便携文件和哈希；不手工改哈希冒充同步。
- [ ] 当前 `prepareDelivery` 的同轮 staging 失败：前置拒绝不创建目录；原生复核失败保留日志并要求新任务重新构建，未实施专门重试协议前不承诺同轮重试。
- [ ] 版本列表与详情在无新增元数据时显示中性状态，不改写“最新”“已安装”的语义。

## 6. 检查与授权门槛

下列命令是执行时的检查清单，本次只写入文档，没有运行：

| 检查 | 命令/范围 | 执行条件 |
| --- | --- | --- |
| 差异 | `git diff --check` 与本任务 diff | 可在只读复核中执行 |
| 限定 Lint | `pnpm exec eslint` 后列出本次实际 TS/Vue 文件，不带 `--fix` | 执行前核对脚本与文件范围 |
| 前端类型 | `pnpm exec vue-tsc --noEmit` | 覆盖全前端，先说明收益与范围并获得确认 |
| Rust 行为 | 分项指定的命令，例如 `cargo test --lib project_context::tests:: -- --nocapture` | 用户明确允许该范围测试；可能编译库，但不启动应用 |
| Skills 评估 | P1 的隔离场景与固定输入 | 明确模型、成本、工具及子代理授权后执行 |
| 资源同步 | `node scripts/prepare-runtime.mjs` | 先核对脚本；会写分发资源，必要时可能准备锁定依赖，获准后执行 |
| CodeGraph | 源文件变更后依次 `codegraph sync`、`codegraph status` | 索引维护，不代替功能检查 |
| 应用与游戏 | 玩家或经授权的隔离验收 | 不从编译结果推导通过 |

技术文档使用 find-docs 的适用方法核对。未安装 Context7 CLI 时直接使用官方文档，不为查文档擅自安装 CLI：参考 [Tauri command](https://v2.tauri.app/develop/calling-rust/)、[SQLite 事务](https://www.sqlite.org/lang_transaction.html)。本计划测试示例只规定合同，不代表代码已存在或示例已经执行。

## 7. 执行前检查点

- [x] 用户选择完整范围 P1–P7。
- [ ] 将正常协议确认与 OS 隔离的区别纳入获批规格；若要求强隔离，先单独设计，不在本计划临时加沙箱。
- [ ] 确定所选分项的迁移编号、格式范围及验证权限；未选项目不创建空实现。
- [ ] 核对现有六个 skills 文件和 docs 的未提交差异，冻结本期修改范围。
- [ ] 需要新增 Duckov API 或素材加载时，取得目标版本资料；否则保留相关实现门槛，不编造接口。

上述检查点完成后，执行者才将对应分项任务置为进行中。首期范围未选不影响阅读、审核或修订这些计划草案，但会阻止开始业务实现。
