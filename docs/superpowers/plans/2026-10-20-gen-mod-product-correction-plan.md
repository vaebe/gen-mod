# GenMod 产品纠偏执行计划总入口

> **面向 AI 代理的工作者：** 使用 executing-plans 逐任务实现。项目规则不允许技能自行授权子代理；本计划不委派实现、不自动提交。各步骤使用复选框记录实际进度。

**目标：** 将已确认的文件操作、项目记忆、跨项目长期记忆落实到现有产品中，撤下被替代入口，补齐项目归档、恢复和永久删除。

**架构：** 采用用户明确选择的路线 A，改造现有 SQLite、project_context、knowledge 和工作区协调。项目记忆属于项目；长期记忆不依赖来源项目存在；Skills 为运行时只读方法。原生处理身份、来源、并发和持久化，Vue 提供已确认入口，pi 只获得对应任务工具。

**技术栈：** 现有 Vue 3、TypeScript、shadcn-vue、Tauri 2、Rust、rusqlite 0.40.2、pi 1.1.0；不新增依赖或外部服务。

**规格：** [已确认的变更设计](../specs/2026-10-20-gen-mod-product-correction-design.md)。

**状态：** 用户已授权按完整计划开始实现，并另行允许本轮限定 Rust/前端单测、类型检查、限定 Lint 及核对后的本地资源同步。沿用当前 codex 分支，不委派实现、不提交、不启动应用或游戏、不操作真实项目数据。T1–T12 实现已接入，已运行本轮限定检查；剩余复合验收项保持未勾选，实际结果见[本轮验证记录](../../verification/2026-10-20-product-correction.md)，未完成的集成与应用验收不能视为通过。

## 全局约束

- 当前仍处于开发阶段，用户已明确不考虑旧内容；旧库升级、历史数据兼容及旧素材恢复不再作为本次完成条件，也不继续扩展其验证。下文已有兼容合同保留为现有实现记录，本次范围调整不要求删除已实现的兼容代码。当前版本新建项目的初始化、持久化、归档/删除及中断恢复仍须验证。
- 保护当前 `codex/gen-mod-improvements` 工作区已有改动；不重置、不整片还原旧 P1–P7，不因旧文件未跟踪而删除其内容。
- “全部文件”支持多文件原样复制导入、单文件移动/改名、创建目标目录；选择文件后直接复制；保留外部原件，同名不覆盖。本期没有目录导入/移动、拖拽和文件删除。
- 导入不转换、解码、裁剪、缩放、分析内容，不以预览类型或大小限制导入；必要路径校验、文件读写和源码指纹不改变文件内容。
- 两种记忆自动保存，只有真实持久化回执才提示成功；项目记忆有歧义才询问；长期记忆允许用户手动修改、删除，AI 不静默覆盖用户改动或恢复已撤回内容。
- 项目记忆在项目上下文“记忆”Tab；长期记忆在设置页，聊天提示可直达条目。不给玩家增加技术分类表单。
- 运行时 AI 不修改 Skills，不提供 Skills 写工具；文件工具只写 source。保持当前进程权限模型，不把工具约束描述为 OS 文件系统沙箱；当前通用命令仍有用户进程权限，不能声称已实现对任意恶意命令的物理只读隔离。
- 保留交付检查、真实构建回执、版本身份、玩家验收及 Duckov ResourceManifest；不把导入或编译成功当成游戏内效果通过。
- 数据迁移追加 009、010，不改写 001–008；不启动时操作真实用户库；新增测试使用内存库和任务夹具。
- UI 只复用已有组件；实现前阅读 `.agents/skills/shadcn-vue/SKILL.md`、Vue 适用技能和组件官方文档，不运行会下载组件或依赖的 CLI。
- 修改源码后执行 CodeGraph sync、status；索引维护不算测试。测试、类型检查、资源同步等必须按下方验证边界执行，不因计划存在而自动运行。
- 归档保留项目全部数据并可恢复；永久删除须由产品用户明确选择并确认，仅清理本地项目私有数据，绝不扫描或卸载游戏目录中的已安装 MOD；长期记忆独立保留。

## 1. 本次核对出的接入事实

1. `files.rs` 与 `SourceBrowser` 当前只读，已能预览图片，不需要复用 assetId 管线才能导入图片。
2. `start_mod_task_blocking` 目前先持会话槽、再持 StorageState，落 `preparing` 后释放存储锁，再获取源码快照租约。UI 文件操作必须与准备阶段共用预留入口，不能只查 running。
3. 当前只有 `delete_mod_session`，没有删除整个项目的 command。用户已补充选择：页面移除时区分归档和永久删除，并支持恢复归档。
4. `knowledge_notes.mod_id` 目前是非空项目外键；直接复用旧生命周期会阻碍项目删除或丢失全局知识，009 必须解除拥有关系。
5. `prepare-runtime.mjs` 的 cpSync 不会删除目标中的旧文件。撤下 assets 扩展和 process-image helper 时必须清理确定的分发残留，再生成清单。
6. 锁定 pi SDK 的 `BeforeAgentStartEventResult.message` 可加入参考消息；项目自由文本不需要拼成 systemPrompt。
7. 锁定 SDK 会捕获 before_agent_start 的异常后继续 prompt；记忆加载失败必须通过原生等待加载握手来阻止启动，不能只在 hook 中抛错。
8. `connect_pi` 的状态读取、建槽、spawn 当前不是同一个受保护区间；生命周期关闭槽之后仍可能重连。`get_mod` 当前也会写数据库，归档只读必须明确改造它。

## 2. 分项顺序、文件职责与检查点

| 执行批次/任务 ID | 分项计划 | 文件职责 | 可独立检查的交付 |
| --- | --- | --- | --- |
| T1–T2 | [项目文件](2026-10-20-project-files-plan.md) | files.rs 保留读入口，新增 source_files 管复制/移动；Workspaces 管 UI 与任务准备预留；SourceBrowser 复用文件树 | 任意普通文件原样导入、冲突不覆盖、项目忙时拒绝、无伪造任务回执 |
| T3–T5 | [长期记忆](2026-10-20-long-term-memory-plan.md) | knowledge 模块和 009 管独立记忆、来源、修订及历史使用；设置组件和工具负责编辑/自动保存 | 自动保存和用户编辑，删除不恢复，旧状态迁移，解除项目拥有关系 |
| T9–T11 | [项目生命周期](2026-10-20-project-lifecycle-plan.md) | 010 管归档和删除记录；lifecycle 管协调与恢复；项目页及侧边栏负责入口 | 归档/恢复保留 ID 和数据；永久删除清理私有记录，保留长期记忆 |
| T6–T8 | [项目记忆](2026-10-20-project-memory-plan.md) | project_context 管来源与自动更新，bootstrap 管任务输入，ContextPanel 展示 | 创建即初始化，跨会话共享、跨项目隔离，自动更新与提示 |
| T12–T13 | 本计划第 5、6 节 | 退役兼容、Skills、运行时资源和接续文档 | 旧模块确实退出，新旧数据和分发一致 |

任务 ID 是稳定标识，**实际依赖顺序为 T1→T2→T3→T4→T5→T9→T10→T11→T6→T7→T8→T12→T13**：长期记忆 009 先于永久删除，生命周期 010 和 archived 合同先于项目记忆 Tab 的归档判断。项目记忆初始化在迁移及旧 JSON 导入之后执行。四项可分别验证，不并行修改共享的 lib.rs、database.rs、task_tools.rs、bridge.rs。

每个分项通过本节对应检查点后再接后续集成；检查点是执行者自检和用户可审阅结果，不默认停下来请求重复授权。

## 3. 固定合同与共享变更

### 3.1 迁移与数据拥有者

| 迁移/存储 | 变更 | 保留边界 |
| --- | --- | --- |
| 009_memory.sql | 重建 knowledge_notes 解除 mod_id 外键，旧条目全部保守标 userEdited；增加不依赖来源的稳定记忆键、memory_content_links 和 task_memory_uses；同事务 Rust 回填正文指纹 | 不把 candidate/private 自动升级为 active/public；不把旧用户正文交给 AI 覆盖；全局指纹关联随记忆独立保留 |
| 010_project_lifecycle.sql | projects 增加 archived 列；project_removals 记录永久删除过的项目 ID、操作 ID、时间，防止旧 JSON 重导入 | 不修改既有项目 payload 和版本包 |
| project_contexts payload | 兼容扩展 fieldSources；事务内初始化，真实来源自动更新 | 老 payload 缺字段默认空；旧快照不补造来源 |
| file_operations | 增加 source_import、source_move、project_delete；导入先记录 source 外临时路径，复制完登记哈希后才提交；删除 pending/requires_review 均阻断普通写入 | 分发移除工具不意味着丢弃 pending 恢复；不依赖临时文件析构处理崩溃 |

迁移单元测和执行实现共同核对 `PRAGMA foreign_key_check`。不能在事务内切换 foreign_keys 来掩盖缺少拥有关系设计；[SQLite 外键说明](https://www.sqlite.org/foreignkeys.html)及[表结构变更说明](https://www.sqlite.org/lang_altertable.html)是核对依据。

### 3.2 工具、身份及 UI command

| 用途 | 模型工具/原生请求 | 用户界面 command |
| --- | --- | --- |
| 项目记忆 | read_project_context、read_project_memory_sources、update_project_memory；gen-mod:read-project-context/read-memory-sources/update-project-memory | get_project_context；选择目录后沿用 inspect_supported_game/apply_inspection |
| 长期记忆 | search_game_knowledge、save_long_term_memory；gen-mod:search-knowledge/save-long-term-memory | list_long_term_memories、get_long_term_memory、save_long_term_memory_entry、set_long_term_memory_status |
| 文件 | 不增加模型导入工具，AI 读已有 source 文件 | pick_source_import_files、import_source_files、move_source_file、create_source_directory |
| 项目生命周期 | 不提供归档、永久删除的模型工具 | set_mod_archived、list_archived_mods、delete_mod |
| 旧素材恢复 | 不提供图片处理工具 | list_legacy_assets、restore_legacy_asset，复用通用文件复制与工作区预留 |

撤下 propose_project_context_update、propose_knowledge、process_project_asset、materialize_project_asset 及其请求映射，不保留工具别名导致旧语义误执行。玩家正式验收仍使用 P4 确认命令，自动记忆不绕过验收确认。

模型不能传入 modId、sessionId、taskId、generation 作为业务归属；继续由 bridge 的活动任务及原始 tool input 绑定。原生参数必须与工具调用匹配；取消、迟到、已归档或已删除项目拒绝写入。长期记忆手动编辑 command 不接受模型来源。

### 3.3 任务准备与 UI 源码写入

在现有 Workspaces 中增加原生预留，不另外建立一套项目锁注册表。建议函数签名如下，由 T1 实现：

```rust
pub enum NativeOperation { SourceChange, TaskPrepare, ProcessConnect, ProjectLifecycle }
pub struct NativeReservation {
    workspace: std::sync::Arc<Workspace>,
    operation_id: String,
}
impl Workspaces {
    pub fn try_reserve_native(&self, mod_id: &str, operation: NativeOperation)
        -> Result<NativeReservation, String>;
}
```

`try_reserve_native` 不持长时间互斥锁等待：先短锁检查已有预留/源码租约，再设置操作 ID，释放锁返回 RAII 令牌。UI 持令牌核对数据库 preparing/running；任务准备也先取得令牌，再持 StorageState 落 preparing，随后释放令牌。非任务连接与空闲重启用 ProcessConnect，从状态核对持续保护到 spawn 和注册完成；生命周期持 ProjectLifecycle 期间拒绝新的连接。不同项目分别预留。长文件复制不持 SQLite 事务、StorageState、会话槽或 Workspace 状态互斥锁；令牌仅保留排他状态。源码租约入口检查原生预留，不能绕过。

锁顺序固定：任务及连接侧“目标会话槽 → try_reserve_native → StorageState → 短事务”；预留失败立即释放槽，不持槽等待预留。新连接先只读确定会话 ID，或在内存生成新 ID，再依此顺序落库，不能先在 session() 中创建会话。UI 不反向获取会话槽。生命周期先取得预留，再在不持 StorageState 的情况下关闭本项目空闲进程，最后操作存储；这时持槽的新连接只能立即预留失败，不能循环等候。纯存储读取不反向等待项目预留。所有失败路径 Drop 释放预留，测试覆盖。

### 3.4 项目记忆加载握手

T7 将 TaskContext 的 memoryBootstrapPath/memoryBootstrapSha256 作为本任务原生输入；扩展初始化核对并缓存参考消息后，通过专用 stderr 前缀 `GEN_MOD_MEMORY_BOOTSTRAP` 发送 ready/error。bridge 的专用通道投递先于会话槽锁，原生不持 StorageState 等待（最多 10 秒）；身份、generation、快照及哈希匹配后才置 running/发 prompt，失败回收进程并记 failed。before_agent_start 只返回已缓存消息，不读盘。该协议不是工具，也不等待玩家确认，完整 DTO 与失败矩阵见 T7。

## 4. 执行前基线

- [x] 执行 `git status --short`、`git diff --name-status`，分别记录 tracked 与 untracked 的当前范围；保存本次计划文件清单，不生成源码副本到长期记忆。
- [x] 核对分支 `git branch --show-current`。沿用用户已选择的当前目录 codex 分支；不新建 worktree，不修改其他分支。
- [x] 对每个即将修改的现有文件读取 `git diff -- <file>`；未跟踪文件直接核对当前全文，不能用 HEAD 空文件覆盖。
- [x] CodeGraph 先 explore 相关入口；若报告 outdated 按项目规则维护索引，不能靠旧索引实施。
- [x] 实施前核对迁移止于 008。若另有新迁移，统一重排本计划 009/010 引用后再动手，不重用已登记编号。
- [x] 执行与限定验证授权已确认；本轮实际结果单独记录，不把旧 P1–P7 的测试通过当作本次结果。

## 5. T12：撤下旧产品链路，保留最低兼容恢复

**修改：** `src-tauri/src/assets/{mod.rs,models.rs,repository.rs}`、`storage/repository.rs`、`lib.rs`、`pi/{workspace.rs,task_tools.rs,bridge.rs}`、`runtime/extensions/mod-tools.ts`、`src/features/{chat/ChatPage.vue,mods/ModManagerPage.vue}`、`scripts/prepare-runtime.mjs`、`scripts/tests/verify-local-assets.ps1`、通用与 Duckov Skills。

**移除：** `src/features/assets/{ProjectAssetsDialog.vue,AssetImagePreview.vue}`、`src/contracts/asset.ts` 的处理配方合同、`src/features/mods/ProjectContextDialog.vue`、`src/features/knowledge/KnowledgeNoteDialog.vue`、`runtime/extensions/assets.ts`、`runtime/helpers/process-image.ps1`、不再被兼容恢复使用的 `assets/images.rs` 与图片处理测试。移除前逐个核对引用；保留旧素材必要类型，转为 assets 模块内部合同，不全删 assets。

- [ ] **步骤 1：保留可恢复旧数据的函数。** assets 仅保留路径验证、元数据读取、哈希核对和旧 pending 恢复，新增 list_legacy_assets/restore_legacy_asset。恢复到 `source/assets/legacy/<assetId>.<原类型扩展名>`；原字节复制，不转换 JPEG，外层原件保留。已存在相同哈希返回 alreadyPresent，不同内容返回冲突；不自动覆盖或复制所有旧素材。

恢复 command 独立返回 `{assetId,status:'restored'|'alreadyPresent'|'conflict',sourcePath}`，不往通用 SourceMutationItem 添加图片专用状态。“全部文件”只在有未恢复旧资产时显示一次性恢复提示和可选择列表；用户选择后直接走恢复 command，复用 source_files 的路径/复制/预留，不恢复旧图片管理面板。
- [ ] **步骤 2：建立兼容恢复回归测试。** 在 `src-tauri/src/assets/tests.rs` 用 tempfile 及内存表分别构造原图、派生图、缺文件、哈希不符、task pending 情况；期望原有资产仍可列出，缺失不登记成功，已中断 task 不复活为成功处理，恢复不依赖图像解码。给待恢复文件写入普通夹具字节即可，不需要真实游戏。

`scripts/tests/verify-local-assets.ps1` 当前同时测试图像 helper 和 ResourceManifest。删除 helper 调用、contain/crop 配方及像素断言，保留它的包内资源哈希和越界目标拒绝断言；不得删除整个脚本而丢失打包回归。脚本仍只写自己创建的 work/asset-evaluation/<uuid>，获准后单独执行。
- [ ] **步骤 3：核对并移除旧入口和工具。** 对上述精确文件执行 `rg -n 'ProjectAssetsDialog|ProjectContextDialog|KnowledgeNoteDialog|process_project_asset|materialize_project_asset|propose_knowledge|propose_project_context_update' src runtime/extensions skills src-tauri/src`；移除产品入口、command 注册和白名单，迁移 SQL 及兼容类型中的旧名称可保留并说明用途。
- [ ] **步骤 4：清理确定的分发残留。** 在 prepare-runtime 的 node:fs 导入中增加 unlinkSync；复制前只清理 portable 下 `extensions/assets.ts`、`helpers/process-image.ps1` 两个确切路径。代码使用路径包含检查和 unlinkSync，不递归清理 runtime、用户数据或任意扫描结果：

```javascript
for (const entry of ['extensions/assets.ts', 'helpers/process-image.ps1']) {
  const destination = join(portable, entry)
  const pathFromPortable = relative(portable, destination)
  if (pathFromPortable.startsWith('..') || pathFromPortable === '') {
    throw new Error('退役资源路径越界')
  }
  if (existsSync(destination)) { unlinkSync(destination) }
}
```

- [ ] **步骤 5：同步 Skills。** mod-workflow 增加项目记忆自动更新、长期记忆读写及用户改动优先规则；delivery.md 保留唯一工具合同；Duckov ui-assets.md 改用普通相对文件路径和 ResourceManifest，不再提配方或 assetId。两技能 registry 版本改为 2.1.0；保留原始构建脚本身份和已有 BOM 修复，不生成新的 API 或资源加载结论。
- [ ] **步骤 6：复核只读职责。** write/edit 仍只能写 source；新增记忆工具只写数据库；没有导入/修改 Skills 的模型工具；普通命令说明明确不得写 Skills。区分该职责与通用命令的 OS 权限，不通过字符串黑名单宣称隔离完成。

**检查点：** 新任务工具列表没有旧处理工具，实际分发同步撤下退役资源。旧素材恢复及旧会话兼容不再作为本次完成条件。源码删除不是分发删除的替代证据。

## 6. T13：验证、文档与交接

- [ ] **步骤 1：更新接续文档。** 修改 `docs/project-context.md`、`docs/README.md`；新增 `docs/verification/2026-10-20-product-correction.md`，按 T1–T13 记录真实结果。2026-10-19 计划保留历史合同，在总入口补充新方案链接，不重写历史测试结论。
- [ ] **步骤 2：执行获准的限定检查。** 命令与范围见下表；先检查脚本与依赖就绪。必要资源缺失时停止相应检查，不让 prepare-runtime 自动下载/部署依赖。没有获准测试时只核对差异和调用链，并把行为验收列为未运行。
- [ ] **步骤 3：核对验收覆盖。** 在当前版本新建的隔离夹具中完成原样导入、部分失败、任务竞争、用户改动与删除不恢复、归档/恢复、永久删除中断恢复；不再要求旧库升级或旧素材恢复验收。涉及应用启动和本地协议服务的项目另行确认隔离范围。
- [x] **步骤 4：同步索引并审阅差异。** 依次执行 CodeGraph sync/status、`git diff --check`；新文件另查空白与引用。不得用索引或静态检查替代测试。
- [x] **步骤 5：交付。** 列明每份子计划完成项、实际命令及结果、未执行检查和剩余风险。无提交、推送或发布动作，不声明真实模型或游戏验收通过。

| 验证 | 拟执行命令/范围 | 说明 |
| --- | --- | --- |
| 文件与协调 | `cargo test --offline --manifest-path src-tauri/Cargo.toml --lib source_files:: -- --nocapture`，以及分别 `--lib pi::workspace::tests:: -- --nocapture`、`--lib pi::bridge::tests:: -- --nocapture` | tempfile/内存通道/假进程工厂；含连接竞争、导入中断及 bootstrap 握手，不碰真实项目或启动真实 pi |
| 记忆与迁移 | 同一 cargo 前缀，分别 `--lib knowledge::`、`--lib project_context::`、`--lib project_lifecycle::`、`--lib storage::database::tests::` | 每次一个 filter；新测试范围需用户明确认可 |
| 旧文件兼容恢复（已退出剩余验收范围） | 已执行结果保留，不再安排补测 | T12 已有夹具和实现保留，不影响本次完成判定 |
| 前端类型 | `node node_modules/vue-tsc/bin/vue-tsc.js --noEmit` | 无模块限定能力，覆盖全前端的理由是 shared contracts/Settings/ContextPanel 同时变化；执行前沿用有效授权或集中补充确认 |
| 限定 ESLint | 本节下方的明确文件数组与本地 eslint | 无 --fix，不扫描无关目录 |
| 扩展类型 | 本节下方 portable extensions 的 NodeNext 严格检查 | 使用已安装 TypeScript 6，不运行 npx 下载 |
| 资源 | 本地依赖、node.zip、部署标记均就绪后 `node scripts/prepare-runtime.mjs` | 会写分发资源与哈希；不触发下载/安装分支 |
| 资源打包 | `powershell.exe -NoProfile -NonInteractive -ExecutionPolicy Bypass -File scripts/tests/verify-local-assets.ps1` | 先完成 T12 脚本调整，只验收资源清单与包内容，不再测试已退役图像处理 |
| 应用 | 隔离 identifier、当前版本新建数据库、临时 MOD 项目 | 应用启动和删除仅限专门夹具，不能用真实项目验收永久删除；不构造旧版升级数据 |

本计划自检对应关系：设计第 4 节→T1–T2；第 5 节→T3–T5/T12；第 6 节→T6–T11；第 7–8 节→各迁移与 T12；第 9–10 节→T13。子代理审核、真实游戏和外部模型评估不在本轮自动执行范围。

获准后，在仓库根目录使用下列 PowerShell 命令，不把 shell 通配符当作已展开文件列表：

```powershell
$correctionLint = @(
  'vitest.config.ts',
  'src/contracts/source.ts', 'src/contracts/knowledge.ts', 'src/contracts/project-context.ts',
  'src/contracts/task.ts', 'src/contracts/mod.ts', 'src/contracts/session.ts',
  'src/services/native.ts', 'src/services/pi/reducer.ts', 'src/services/pi/reducer.test.ts',
  'src/services/pi/sessionRuntime.ts', 'src/stores/projects.ts', 'src/stores/session.ts',
  'src/features/workbench/SourceBrowser.vue', 'src/features/workbench/SourceFileTree.vue',
  'src/features/workbench/useSourceBrowser.ts', 'src/features/workbench/SourceDirectoryDialog.vue',
  'src/features/workbench/useSourceFileActions.ts', 'src/features/workbench/useSourceFileActions.test.ts',
  'src/features/workbench/ContextPanel.vue', 'src/features/workbench/ProjectMemoryTab.vue',
  'src/features/workbench/useProjectMemory.ts', 'src/features/workbench/ProjectSidebar.vue',
  'src/features/workbench/LegacyAssetsRecovery.vue',
  'src/features/knowledge/LongTermMemorySettings.vue', 'src/features/knowledge/MemoryEntryEditor.vue',
  'src/features/knowledge/useLongTermMemory.ts', 'src/features/settings/SettingsPage.vue',
  'src/features/chat/ChatPage.vue', 'src/features/chat/ToolActivity.vue',
  'src/features/mods/ProjectPage.vue', 'src/features/mods/ModManagerPage.vue',
  'src/features/mods/VersionList.vue', 'src/features/mods/VersionActions.vue',
  'src/features/mods/GameInspectionPanel.vue', 'src/features/mods/RemoveProjectDialog.vue',
  'src/features/mods/ArchivedProjectsDialog.vue'
)
$presentCorrectionLint = @($correctionLint | Where-Object { Test-Path -LiteralPath $_ -PathType Leaf })
& node node_modules/eslint/bin/eslint.js @presentCorrectionLint
```

若实施新增了直接相关文件，先更新该数组及执行记录；不能以删除数组条目掩盖应检查文件。扩展检查使用：

```powershell
$correctionExtensions = @(Get-ChildItem -LiteralPath 'runtime/resources/pi-portable-v1.1.0/extensions' -File -Filter '*.ts' | ForEach-Object FullName)
& node node_modules/typescript/bin/tsc --ignoreConfig --noEmit --target ES2022 --module NodeNext --moduleResolution NodeNext --strict --skipLibCheck --esModuleInterop --types node @correctionExtensions
```

