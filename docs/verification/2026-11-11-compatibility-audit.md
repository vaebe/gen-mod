# GenMod 兼容逻辑对抗审核

日期：2026-11-11。代码基线：`bce7101539e855fd5c38720a4acd70dd42dd7c58`。

用户已明确：应用尚未发布，不需要兼容旧应用数据。本审核用于核实上一轮兼容清单，不能作为删除开发机数据、调整业务行为或删除代码的执行授权。

## 已确认保留的行为

2026-11-11，用户在审核后明确以下行为保持原样，排除出本轮兼容清理范围：

- S06 的 `read_on`：项目记忆不存在时返回空记忆，属于正常处理。此决定不涉及启动时扫描旧项目的 `backfill`。
- F08 的 `excludePaths`：允许为空及省略后的空列表处理，属于既定设计。
- F08 的 `UserRegistry`：保留 `imported`、`disabled` 空列表、缺字段时补空列表，以及首次运行无文件时的初始化。
- R02：保留缺少显式任务 ID 时的时间匹配逻辑，不在本轮调整任务关联。
- F04：保留结束时间的可空语义及当前缺字段读取行为。

上述项目不再作为待用户确认事项；以下源码证据及静态验证限制仍然适用。

## 判断标准

“历史兼容”指当前正常写入流程不再产生某种数据或协议形态，代码仍专门读取、转换或接纳该旧形态。必须核对写入者、读取者和入口，不能只根据 `legacy`、`default`、`Option`、`?` 或“旧”的注释判断。

- **历史兼容**：旧数据或旧协议分支；可以列入后续清理范围，但必须保留共享的当前功能。
- **混合用途**：历史兼容和当前初始化、正常缺值或恢复机制交织，必须拆分后判断。
- **当前功能**：同一应用版本内的新数据也会触发，不因未发布就删除。
- **旧设计残留**：不再有当前业务生产者，但并不执行兼容转换。
- **待确认**：存在当前调用或契约歧义，尚不能证明只服务于旧数据。

字段缺失、字段值为空、字段值为 `null` 是三件不同的事。取消缺字段容忍，不等于禁止正常空值；删除 `Option` 上的 `serde(default)` 也不能直接宣称已完成严格校验。

## 审核范围与方法

覆盖上一轮全部候选项：存储迁移、素材恢复、长期记忆、交付元数据、运行时协议、历史消息投影、字段缺省、前端回退和构建脚本。第三方依赖内部实现、实际用户数据库与游戏运行不在本轮范围。

3 个子代理分别独立审核存储与素材、运行时与协议、字段与前端。每项要求尝试提出“当前版本新数据仍会触发”的反例；主代理复核源码和共享调用，集中形成下面的结论。本文为静态审核，不代表测试或运行时复现。

## 逐项结论

### 存储与旧功能

| 编号 | 候选逻辑与源码 | 结论 | 证据及清理边界 |
| --- | --- | --- | --- |
| S01 | 旧 JSON 项目导入（历史路径 `src-tauri/src/storage/migration.rs`，已退役；见本文原清理记录），`project` / `import` | 历史兼容 | 读取 `mod.json`、`task.json`、旧运行 JSONL，补已删除会话和旧草稿；当前项目通过 SQLite 仓储创建。清理范围是导入链，不能删除仓储正常写入。 |
| S02 | [迁移失败记录](../../src-tauri/src/storage/repository.rs)，`list_migration_errors`；[项目列表](../../src/stores/projects.ts) | 历史兼容 | `legacy_imports`、查询项目失败时的迁移错误解释、IPC 注册和列表提示依赖 S01。必须一并清理查询、表及生命周期清理引用，不能只删表。 |
| S03 | [数据库版本升级](../../src-tauri/src/storage/database.rs)，`initialize`；SQL 目录（历史路径 `src-tauri/src/storage/migrations`，已退役；见本文原清理记录） | 混合用途 | 全新空库也依次执行 001–012；其中包含当前表、索引、列的定义。可改为当前结构的初始 schema，但不能直接删除脚本或初始化调用。 |
| S04 | 长期记忆迁移（历史路径 `src-tauri/src/storage/migrations/009_memory.sql`，已退役；见本文原清理记录）；[迁移 helper](../../src-tauri/src/knowledge/repository.rs)，`migrate_memory_on` / `backfill_content_links_on` | 混合用途 | 旧记录搬迁、`legacy:` 主题键和批量正文指纹补齐是历史兼容；最终记忆表、内容关联表、任务使用表是当前结构。`remember_content_on` 仍服务于当前保存和编辑，必须保留。 |
| S05 | [历史候选界面](../../src/features/knowledge/LongTermMemorySettings.vue)；[状态筛选](../../src-tauri/src/knowledge/mod.rs)；[自动保存保护](../../src-tauri/src/knowledge/repository.rs) | 历史兼容 | 当前手工和自动创建记忆写 `active`，当前状态修改不产生 `candidate`。历史候选筛选、标签、类型及针对该状态的保护可成组清理；不能删除当前停用、删除、用户编辑保护。 |
| S06 | [项目记忆补齐](../../src-tauri/src/project_context/repository.rs)，`backfill` / `read_on` | 历史补齐与已确认正常回退 | 启动 backfill 属历史补齐；当前 create 已在事务内初始化。用户确认 read_on 缺行返回空记忆是正常处理，保持原样。initialize_on、空记忆构造及 revision=0 首次保存分支仍用于新项目，必须保留。 |
| S07 | [交付元数据补齐](../../src-tauri/src/artifacts/delivery_metadata.rs)，`backfill` / `ensure_anchor_on` | 混合用途 | 启动遍历既有版本是历史补齐；`ensure_anchor_on` 也由当前版本登记、删除及恢复路径调用。可以清理启动 backfill，不能直接删除锚点函数及当前数据结构。 |
| S08 | 旧素材模块（历史路径 `src-tauri/src/assets/mod.rs`，已退役；见本文原清理记录）；恢复界面（历史路径 `src/features/workbench/LegacyAssetsRecovery.vue`，已退役；见本文原清理记录）；[文件浏览器](../../src/features/workspace/SourceBrowser.vue)；[源码恢复入口](../../src-tauri/src/source_files/mod.rs)，`restore_legacy` | 历史兼容 | 当前文件导入走 source；旧素材链读取 originals/derived 和 `project_assets`，原样恢复到 `source/assets/legacy`。可连同旧表、IPC、界面、恢复测试清理；共享文件导入、路径与指纹校验不能跟着删除。 |
| S09 | [旧素材操作恢复](../../src-tauri/src/storage/repository.rs)，`reconcile_files` 的 `asset_import` / `asset_process` 分支 | 历史兼容 | 仅服务旧素材操作日志。可以连同 `assets::reconcile` 清理；当前 `source_import`、`source_move`、安装、交付和删除恢复必须保留。 |
| S10 | 项目记忆提案表（历史路径 `src-tauri/src/storage/migrations/003_project_context.sql`，已退役；见本文原清理记录），`context_proposals` | 旧设计残留 | 仍建表并参与项目删除清理，当前未发现业务创建、读取提案的路径。它不等同于迁移算法；清理时同步 schema、生命周期引用和相关夹具。 |

### 交付与界面

| 编号 | 候选逻辑与源码 | 结论 | 证据及清理边界 |
| --- | --- | --- | --- |
| V01 | [版本摘要缺省](../../src-tauri/src/artifacts/models.rs)，`ArtifactVersion.summary`；[交付卡片](../../src/features/chat/DeliveryCard.vue)；[版本列表](../../src/features/delivery/VersionList.vue)；[详情](../../src/features/delivery/ArtifactPanel.vue) | 历史兼容 | 当前工具及原生登记均要求非空摘要，旧记录缺失时补空串，界面隐藏摘要或显示文件数与时间。可统一当前必填契约，不能用“移除 default”代替非空值校验。 |
| V02 | [来源会话缺省与反查](../../src/features/delivery/VersionConversationLink.vue)，`navigate`；[存储模型](../../src-tauri/src/artifacts/models.rs) | 历史兼容 | 当前登记写入 `session_id`；缺少时查询任务反查来源属于旧字段兼容。来源会话后来被删除的检查仍是当前正常生命周期，必须保留。 |
| V03 | [安装目录回退](../../src/features/delivery/ModManagerPage.vue)，`previousTarget`；[上下文面板](../../src/features/workbench/ContextPanel.vue) | 历史兼容分支 | 当前预览会保存 `lastDeploymentDirectory`；从安装记录反查目录服务于缺少该字段的项目。可去掉回退，保留从未选择目录的新项目的空值。 |
| V04 | [安装备份及回退模型](../../src-tauri/src/install/models.rs)，`InstallationRecord` / `RollbackStatus`；[历史展示](../../src/features/delivery/ModManagerPage.vue) | 历史兼容 | 当前安装不生成备份，写 `notNeeded` 或 `notAttempted`；旧 `backupDirectory`、`succeeded` / `failed` 回退结果只剩读取展示。清理不能删除失败记录、目标不确定状态和未回退说明。 |
| V05 | [游戏名称回退](../../src/features/projects/ProjectPage.vue)，`gameName` | 当前容错与非兼容冗余 | `catalog name → gameId` 用于目录未加载或名称查不到的当前场景。最后一层 `gameSkillName` 对合法非空 gameId 不可达，可以作为冗余分支简化，但没有证据证明其专为旧数据设计；字段仍由当前创建流程写入，并在聊天空状态展示。 |

### 运行时与构建

| 编号 | 候选逻辑与源码 | 结论 | 证据及清理边界 |
| --- | --- | --- | --- |
| R01 | [工具结束后的交付登记](../../src-tauri/src/pi/bridge.rs)，`prepare_delivery` 的 `tool_execution_end` 分支 | 历史兼容分支 | 当前工具先通过 `gen-mod:register-delivery` 取得登记回执，再返回结果。可清理结束事件中的补登记触发；共享 `register_prepared_delivery` 仍由新握手路径调用，不能删除。 |
| R02 | [按时间匹配任务](../../src/features/chat/conversationRounds.ts)，`conversationRounds` | 混合用途，已确认保持原样 | 显式任务 ID 优先，缺失时才根据已确认消息时间匹配。当前历史 helper 只补 entryId，submissions 只给匹配的 user 补 taskId；SDK 历史已保存而原生尚未确认消息身份时退出，也可能缺少关联。用户确认保留当前关联方式，不纳入清理。压缩后可能出现无 user 的投影，但本轮未完整核实首保留策略，不把这一点作为确定反例。 |
| R03 | [历史身份补齐](../../src-tauri/src/pi/submissions.rs)，`annotate_history`；[引导记录恢复](../../src-tauri/src/storage/repository.rs)，`reconcile_steering` | 当前功能 | 当前 SDK 消息历史与应用请求身份分开保存；重开历史及确认后尚未绑定 entryId 的恢复仍需关联。与应用升级无关。 |
| R04 | [Step Plan 映射补齐](../../src-tauri/src/config.rs)，`restore_thinking_map` | 当前服务适配 | 新创建的配置也允许缺少思考档位映射；指定服务、协议、模型组合会补齐支持范围。不能因注释“恢复原协议”而按历史数据兼容删除。 |
| R05 | [退役资源清理](../../scripts/prepare-runtime.mjs)，`extensions/assets.ts` / `helpers/process-image.ps1` | 构建产物升级清理 | 处理增量构建目录遗留文件，不读取玩家旧数据。可以在确认采用干净构建或通用目录同步后移除特例；已有构建目录包含退役资源时，直接删除清理分支会保留这些资源。 |
| R06 | [pnpm 部署参数](../../scripts/prepare-runtime.mjs)，`--legacy`；[发布流程](../../.github/workflows/release-windows.yml) | 当前构建机制 | 当前运行时部署直接使用该模式，发布流程也处理它对工作区依赖的影响。名称含 legacy 不代表兼容旧应用，变更属于构建方案调整。 |

### 字段缺省与正常可选值

| 编号 | 候选逻辑与源码 | 结论 | 证据及清理边界 |
| --- | --- | --- | --- |
| F01 | [项目与会话模型](../../src-tauri/src/storage/models.rs)，`name` / `revision` / `archived` / `description` | 缺字段兼容与当前空值分离 | 当前创建与完整序列化都写这些字段，可去掉缺字段 default；新会话名称、选填描述为空仍合法，不能删除侧边栏“新会话”占位。revision 和 archived 仍以数据库权威列为准，不能因去掉 default 改成相信 payload。 |
| F02 | [任务上下文](../../src-tauri/src/pi/tasks.rs)，快照及 bootstrap 可选字段 | 混合用途 | 当前正式任务在第一次持久化前已填入 contextSnapshotId，可考虑收紧该字段；bootstrap path/hash 则在 preparing 任务落库后才写入，当前版本失败或退出时可缺失。不能把三个字段一起改必填；运行任务的 bootstrap 校验仍需保留。 |
| F03 | [命令回执](../../src-tauri/src/pi/tasks.rs)，`CommandReceipt` | 混合用途 | 当前 [commands.ts](../../runtime/extensions/commands.ts) 返回回执始终写 program、args、sourceRevision、startedAt、durationMs，这些缺字段容忍可收紧。pid、exitCode、startupError 则有当前成功/失败路径下的合法缺值。outputTail 为空时当前 Rust 序列化会省略，读取依赖 default，不能只删 default。 |
| F04 | [任务结束时间](../../src-tauri/src/pi/tasks.rs)，`TaskRecord.finished_at` | 当前可选状态，已确认保持原样 | 正在准备、运行的任务没有结束时间。用户确认保留可空语义及当前缺字段读取行为，不纳入清理；单凭 default 不能证明其设计动机仅为历史兼容。 |
| F05 | [项目记忆模型](../../src-tauri/src/project_context/models.rs)，`sourceReportId` / `fieldSources` | 旧缺字段支持与当前空集合分离 | 当前 empty 构造会写空 fieldSources，可移除该字段的缺省但保留空 map。当前 ReferenceSet 唯一生产构造来自 [调查报告应用](../../src-tauri/src/game_inspection/mod.rs)，总写 sourceReportId；可另行收紧其 Option 类型，不能只删 default。没有参考集合的新项目仍合法。 |
| F06 | [长期记忆模型](../../src-tauri/src/knowledge/models.rs)，来源、编辑标记、主题键与时间 | 混合用途 | 当前手工记忆无来源项目，删除条目也清空来源，来源可空必须保留。当前创建者总写 userEdited、memoryKey 和时间，可收紧对应缺字段/旧时间支持；[编辑器](../../src/features/knowledge/MemoryEntryEditor.vue) 的“历史记录”时间回退也属于候选。NOTE_SELECT 从 SQL 列覆写多数扩展字段，应同时核对列与 payload；字段本身参与去重和用户保护，不能删除。 |
| F07 | [文件操作日志](../../src-tauri/src/source_files/mod.rs)，`SourceOperation.directory` | 历史形态容忍 | 当前文件导入和移动都明确写入 directory；缺字段补 false 可收紧。保留当前目录/文件区分和中断恢复。 |
| F08 | [Skills 模型](../../src-tauri/src/skills/models.rs)，`SkillDefinition` / `UserRegistry` | 当前配置契约，已确认保持原样 | 当前内置 mod-workflow 和界面默认导入 JSON 省略 requiredCommandScripts、inspectionFile，不能按旧兼容移除。用户确认 excludePaths 允许为空的设计，以及 UserRegistry 的空列表、缺字段补空和首次初始化均保持原样，不收紧读取规则。 |

## 不应作为历史兼容删除的功能

- [Skills 快照与只读会话](../../src-tauri/src/skills/registry.rs)：同一应用版本中用户修改能力或更新 Skills，也会使既有会话上下文变化。
- [下载能力与内置能力选择](../../src-tauri/src/skills/registry.rs)：服务于当前离线使用和已下载版本，不是旧应用数据迁移。
- [文件操作恢复](../../src-tauri/src/storage/repository.rs)、[项目删除恢复](../../src-tauri/src/project_lifecycle/recovery.rs)：当前版本崩溃、写入中断也会触发。
- [模型配置缺省](../../src-tauri/src/config.rs)：首次启动、可选 API Key、模型能力映射都可能缺值；保存表单空密钥保留现有密钥是当前编辑语义。
- [Windows 命令环境探测](../../runtime/extensions/commands.ts)：PATH 大小写、PowerShell 可执行文件探测及子进程环境补充属于环境适配。
- [消息内容解析](../../src/features/chat/reducer.ts)、[历史读取](../../runtime/helpers/read-session.mjs)：字符串与内容数组是当前协议支持的形态，不是仅旧应用数据。
- [游戏 Skills](../../skills)：不同游戏、前置和 MOD 版本的适配属于生成 MOD 的业务范围，与 GenMod 自身尚未发布是两个问题。
- [初始草稿转入首个会话](../../src-tauri/src/storage/repository.rs)，`save_on`：代码明确保留尚无会话的新项目草稿，不应与 JSON 导入时迁移旧草稿混为一谈。
- [Skills 版本存储路径](../../src-tauri/src/skills/official.rs)，`definition`：当前实现保留可用的单级版本目录，其余版本走 hash 目录。它同时保留既有路径并适配当前版本字符串；统一路径布局是存储方案调整，不能直接删除某分支使其支持的当前版本无法安装。

## 对上一轮清单的纠正

1. “数据库迁移可删除”必须限定为升级过程和旧记录搬迁；当前最终结构与初始化不能删除。S03、S04 已明确拆分。
2. 用户已确认项目记忆缺行转空是正常处理，保持原样；它与历史 backfill、新项目 revision=0 首次保存分支是不同对象，见 S06。
3. gameSkillName 最后一层显示回退属于可简化的冗余，不能据此认定整个字段是兼容字段，见 V05。
4. 任务快照和 bootstrap 的写入阶段不同，命令无输出时还主动省略 outputTail；不能整组取消可选性或 default，见 F02、F03。
5. 时间关联、历史身份补齐及引导恢复可能服务于当前版本内的正常重开或异常退出，不等于旧应用升级，见 R02、R03。
6. Skills 配置的字段缺省存在当前内置数据与默认导入输入的直接反例；用户已确认 excludePaths 和 UserRegistry 当前处理保持原样，见 F08。

## 清理边界

后续若执行清理，应按“旧入口及专属数据 → 共享函数调用核对 → 类型与界面 → 文档与测试夹具”成组处理。数据库升级历史若合并，先准备能完整创建当前所需表、索引和列的初始 schema，再移除旧升级过程。

删除兼容分支并不授权删除本机 SQLite、项目源码、安装记录或素材。也不意味着需要保留旧数据迁移；两者应分别确定执行范围。

## 验证记录

3 个子代理均已完成独立源码核查和成稿复核。存储审核提出的缺行回退分类、运行时审核提出的构建目录前提和结束时间分类已采纳；字段审核确认成稿的合法空值与历史缺字段边界。

覆盖记录：S01–S10、V01–V05、R01–R06、F01–F08 共 29 项，另列当前功能排除项。43 个不同的本地源码链接均已检查存在，条目编号无重复，文档无行尾空白；`git diff --check` 未报告错误，工作区仅新增本文。

本轮只写审核文档，没有修改业务代码、数据库或运行时资源；未运行测试、Lint、类型检查、构建或应用，未提交本文。静态分类不等于清理后的功能验收。


## 实施状态补记（2026-11-11）

用户随后授权执行[兼容清理实施计划](../superpowers/plans/2026-11-11-compatibility-cleanup.md)。当前工作区已完成计划中的代码清理、当前 schema 合并、契约收紧及相关夹具调整，尚未提交。上文源码链接与证据描述对应审核基线，部分文件已在本次实施中删除；不应把基线链接失效误认为当前仍存在旧实现。

S01–S10 按混合用途边界实施，其中 S06 正常空记忆读取、S07 当前锚点保留；V01–V04、R01、F01–F03、F05–F07 已清理或收紧。V05、R02–R06、F04、F08 及明确排除的当前功能保持原样。

实际检查为差异/引用核对、限定修改文件的 ESLint 和 CodeGraph 索引同步。ESLint 发现的一处未使用解构变量已修正并复查通过。未执行测试、类型检查、Rust 编译、构建或应用运行，不能据此宣称功能验收通过。具体变更与验证记录见实施计划第 10 节。


### 后续授权检查

用户授权后，`cargo check --manifest-path src-tauri/Cargo.toml --lib` 通过；`pnpm exec vue-tsc --noEmit` 发现 SourceBrowser 的共享 nativeCall 导入被误删，恢复后复查通过。Rust 仅报告增量缓存硬链接失败并回退复制的环境警告。未运行测试、应用或生产构建，测试代码未通过测试编译验证。此次检查补充上段的历史验证记录，不代表功能验收。


### 桌面运行验证（阶段性，2026-11-11）

用户授权实际启动应用后，使用临时 Tauri 配置将 identifier 设为 `com.ygdpz.gen-mod.validation-20261111`，窗口名为 `GenMod Compatibility Validation`。配置位于忽略目录 `.scratch/compatibility-runtime/tauri.validation.json`，未改变正式应用配置。

实际执行 `pnpm dev:app --config D:/codetest/gen-mod/.scratch/compatibility-runtime/tauri.validation.json`。受限进程首次无法连通本地 Vite；停止该次启动后，在获批准的桌面执行环境中成功构建并运行 `target/debug/gen-mod.exe`。

已观察到：

- 应用成功显示空项目首页，无启动错误弹窗。
- 打开“新建 MOD”表单，游戏列表从加载状态变为“逃离鸭科夫”，项目描述显示选填。
- 只读查询隔离数据库：user_version=1、18 张当前业务表、journal_mode=wal、0 个项目；未创建退役表。
- 原应用使用另一数据目录，仍为 user_version=0、22 张表、1 个项目；本次启动未对其执行迁移或清理。

界面自动化在填写项目名称前遇到 `failed to activate captured window`，重新获取窗口并激活仍失败。已停止输入并请求用户将验证窗口切到前台。项目创建、源码导入/移动、记忆编辑、重开恢复、模型交付及安装流程目前均未完成实际验证，不能将启动成功表述为全流程通过。

开发控制台另报告 `Invalid layout total size: 75%. Layout normalization will be applied.`，尚未定位，不能认定由本次兼容清理引入。验证实例暂留在新建项目表单，等待继续。
