# GenMod 未发布阶段兼容逻辑清理实施计划

日期：2026-11-11。状态：T1–T8 代码与文档变更已实施，运行验证未执行。

代码基线：`bce7101539e855fd5c38720a4acd70dd42dd7c58`。

依据：[兼容逻辑对抗审核](../../verification/2026-11-11-compatibility-audit.md)及用户随后确认的保留项。审核编号用于追踪范围，不将“可清理”视为已经获得实施授权。

## 1. 目标与实施约束

目标：删除只服务旧应用数据、旧协议和退役素材功能的代码；使用单份当前数据库结构初始化新库；让当前生产者始终提供的字段与 Rust、TypeScript 契约一致。

架构保持现状：Vue 前端 → Tauri IPC → Rust 仓储及 SQLite；Node 运行时继续使用现有命令、工作区租约和交付登记协议。不新增依赖，不调整模块职责，不建立迁移框架。

技术栈：Vue 3、TypeScript、Rust、Serde、rusqlite、SQLite，以及现有 Node 运行时。

执行约束：

- 本文只安排工作，不执行源代码修改、数据库操作、测试、应用启动、提交或推送。
- 后续获得实施授权后，先核对工作区和当前源码；保护其他改动，按本计划的依赖顺序执行。
- 不删除本机 SQLite、项目目录、素材、安装记录；不自动重置旧开发数据。
- 移除字段缺失容忍，不等于禁止空字符串、空集合、合法 `null`；不机械删除所有 `default`、`Option` 或 `?`。
- 删除旧入口时同步删除其专属消费者；共享函数按当前调用链保留。
- 不顺带清理其他冗余、重构构建脚本或统一 Skills 存储布局。
- 不自动运行测试。本文中的测试方案需用户明确授权执行；全仓类型检查或编译也按项目规范另行确认范围。
- 不自动启动子代理、创建额外任务跟踪文件或提交代码。

## 2. 已锁定的保留项

下列行为不再重新讨论，也不作为本次清理目标：

1. S06：项目存在但记忆记录缺失时，`read_on` 返回空记忆；保留新项目初始化、空记忆构造及 revision 为 0 的首次保存。
2. F08：`excludePaths` 可以为空，省略时按既定规则处理；`UserRegistry.imported`、`disabled` 的空数组、缺字段补空和文件不存在时初始化均保留。
3. R02：会话消息缺少显式任务 ID 时，保留现有时间匹配。
4. F04：`finishedAt` 保持可空以及当前缺字段读取行为。
5. 当前中断恢复、删除恢复、历史消息身份关联、Skills 快照、离线能力选择、模型配置和 Windows 环境适配均保留。
6. R05、R06：保留增量构建退役资源清理、`pnpm --legacy` 及发布流程；它们不是旧玩家数据迁移。
7. V05：不处理游戏名称的冗余末级回退，也不删除当前仍使用的 `gameSkillName`。

## 3. 审核项到任务的完整映射

| 审核项 | 决策 | 实施任务 |
| --- | --- | --- |
| S01、S02 | 删除旧 JSON 导入及迁移错误展示链 | T1，表在 T4 移除 |
| S03、S04 | 合并当前 schema，删除升级过程、旧记忆搬迁和批量指纹补齐 | T4 |
| S05 | 删除历史 candidate 状态支持 | T3 |
| S06 | 仅删除启动 backfill，正常空记忆保留 | T1 |
| S07 | 仅删除启动 backfill，当前锚点登记及恢复保留 | T1 |
| S08、S09 | 删除旧素材恢复及旧操作日志处理 | T2，表在 T4 移除 |
| S10 | 删除旧提案表及直接残留 | T4 |
| V01、V02 | 摘要、来源会话统一为当前必填契约 | T5 |
| V03、V04 | 删除旧目录反查、备份与退役回退结果 | T5 |
| V05 | 非兼容冗余，本轮不动 | 保留 |
| R01 | 删除工具结束事件补登记 | T7 |
| R02 | 用户确认保留 | 保留 |
| R03、R04 | 当前功能 | 保留 |
| R05、R06 | 当前构建机制 | 保留 |
| F01、F02、F03 | 只收紧当前必有字段 | T6 |
| F04 | 用户确认保留 | 保留 |
| F05、F06、F07 | 收紧必有字段，保留合法空集合和来源缺值 | T6，时间列配合 T4 |
| F08 | 用户确认保留及当前配置契约 | 保留 |

## 4. 文件职责与修改范围

以下列出主要修改位置；同一字段的构造器、消费者和现有夹具属于直接关联范围，实施时按符号补齐，不扩展为全仓重构。

| 文件或目录 | 本次职责 |
| --- | --- |
| `src-tauri/src/storage/migration.rs`、`storage/mod.rs` | 删除旧 JSON 导入模块及声明 |
| `src-tauri/src/storage/database.rs` | 删除历史启动调用，改用当前 schema 初始化 |
| `src-tauri/src/storage/schema.sql`（新增） | 唯一当前建库结构 |
| `src-tauri/src/storage/migrations/001_initial.sql` 至 `012_submission_messages.sql` | 替换完所有调用与夹具后删除历史脚本 |
| `src-tauri/src/storage/repository.rs`、`lib.rs`、`src/stores/projects.ts` | 删除迁移错误 IPC 链及旧素材恢复分发 |
| `src-tauri/src/project_context/repository.rs`、`artifacts/delivery_metadata.rs` | 删除启动扫描，保留当前共享写入 |
| `src-tauri/src/assets/` | 删除旧素材专属模块和专属测试 |
| `src-tauri/src/source_files/mod.rs` | 删除旧恢复入口，保留正常导入、移动、日志恢复 |
| `src/features/workbench/SourceBrowser.vue`、`LegacyAssetsRecovery.vue` | 删除旧素材入口、状态和专属组件 |
| `src-tauri/src/project_lifecycle/repository.rs` | 删除退役表清理引用，保留项目删除顺序 |
| `src-tauri/src/knowledge/`、`src/features/knowledge/types.ts` | 删除 candidate 与迁移 helper，收紧当前字段 |
| `src/features/knowledge/LongTermMemorySettings.vue`、`MemoryEntryEditor.vue` | 同步状态和时间展示 |
| `src-tauri/src/storage/models.rs`、`src/contracts/` | 同步项目、会话、交付、安装和任务契约 |
| `src/features/chat/DeliveryCard.vue`、`src/features/mods/`、`src/features/workbench/ContextPanel.vue` | 删除已确认的交付展示回退 |
| `src-tauri/src/pi/tasks.rs`、`bridge.rs`、`workspace.rs` | 收紧任务/命令契约，删除旧交付握手 |
| `src-tauri/src/project_context/`、`src-tauri/src/game_inspection/mod.rs` | 必填快照和调查来源字段，保留身份校验 |
| `src-tauri/src/artifacts/` | 同步命令回执校验、交付和安装构造器 |
| `runtime/extensions/commands.ts` | 同步 TaskContext 类型，现有回执生产语义保持 |

## 5. 顺序与交付单位

顺序：T1 → T2 → T3 → T4 → T5 → T6 → T7 → T8。

T1–T3 先移除旧业务消费者，暂时保留原建库脚本。T4 再替换 schema 及全部引用，避免删除表后前端查询失败。T5、T6 每组类型与全部构造/读取调用必须一起修改，不能只提交模型的一半。T7 保留新登记通路。T8 集中核对全部保留项和清理项。

这是执行顺序，不是自动提交指令。若后续要求分批提交，每个单位均应包含其直接相关夹具、文档和验证说明；不把无法编译的中间状态作为完成结果。

## 6. 详细任务

### T1：移除旧导入和历史启动扫描

- [x] 删除 `storage/migration.rs`，同步删除模块声明和 `database::initialize` 中的 `migration::import`。
- [x] 删除 `list_migration_errors` 命令、IPC 注册及前端调用；调整项目列表的 `Promise.all` 解构和错误展示，保留正常列表失败提示。
- [x] 删除 `project_context::repository::backfill` 和 `delivery_metadata::backfill` 及启动调用。
- [x] 保留 `initialize_on`、`read_on`、`ensure_anchor_on`；逐一确认创建项目、登记交付、删除和恢复仍调用需要的共享函数。
- [x] 保留 `recover_pending`、`interrupt_previous`、pending submission 拒绝处理及 `reconcile_files`，保持它们之间的现有顺序。
- [x] 本步暂不删除旧 SQL 表，统一由 T4 处理。

验收：前端不再请求迁移错误；启动不再读取旧项目 JSON 或遍历所有项目/版本做历史补齐；新项目初始化和当前恢复入口仍在。

### T2：删除旧素材恢复功能

- [x] 删除 `assets` 模块及其模块声明、专属模型、仓储和测试。
- [x] 删除 `list_legacy_assets`、`restore_legacy_asset` 的 IPC 注册与前端调用。
- [x] 删除 `LegacyAssetsRecovery.vue`；清理 `SourceBrowser.vue` 的对应导入、状态、刷新请求、按钮和弹窗。
- [x] 删除 `source_files::restore_legacy`、`legacy_target_status` 及只被它们使用的逻辑。
- [x] 核对 `import_reserved` 的 `expected_hash` 调用者；确认仍只有旧恢复使用后，删除该参数及其专属比较，更新正常导入调用。不能删除正常导入的路径校验、当前指纹计算和落盘保护。
- [x] 从 `reconcile_files` 删除 `asset_import`、`asset_process` 分支及 `assets::reconcile` 调用；保留当前 source、安装、交付、删除和会话删除恢复分支。
- [x] 不删除本机 originals、derived 或 `source/assets/legacy` 目录；取消应用功能不等于获得数据删除授权。

验收：旧素材入口及恢复协议无生产代码引用；普通文件/目录导入、移动和中断恢复的实现仍完整。`project_assets` 在 T4 从新 schema 和生命周期清理中一并移除。

### T3：移除历史长期记忆 candidate 状态

- [x] 从 Rust 状态筛选、前端联合类型和设置界面移除 `candidate` 支持、标签及操作入口。
- [x] 删除自动保存保护中仅针对 candidate 的条件，保留 `user_edited`、archived、deleted 等当前保护。
- [x] 同步相关夹具；不把变量名 `candidates` 等普通候选集合误当成历史状态。
- [x] 核对手工创建和 AI 创建仍写 active，状态更新仅接受当前支持值。

验收：不存在可达的历史候选状态展示或转换；用户编辑保护、停用和删除后的防重复保存逻辑保持。

### T4：使用当前 schema，移除历史升级链

#### T4.1 定义最终结构

- [x] 新增 `storage/schema.sql`，直接创建最终结构，不包含旧表搬迁、`ALTER TABLE` 升级或 `legacy:` 键生成。
- [x] 保留以下 18 张业务表及现有有效索引、主外键和约束：

  `projects`、`sessions`、`tasks`、`runtime_logs`、`steering_inputs`、`session_stats`、`file_operations`、`project_contexts`、`context_snapshots`、`delivery_metadata`、`feedback_drafts`、`verification_records`、`knowledge_notes`、`inspection_reports`、`memory_content_links`、`task_memory_uses`、`project_removals`、`submission_requests`。

  实施核对时检查逐表枚举与最终建表数一致。

- [x] 将 sessions 的 provider/model 列、projects.archived、submission_requests 的消息时间与指纹直接写入建表语句；保持现有列顺序，核对无列名 INSERT 的夹具。
- [x] knowledge_notes 使用 009 迁移后的最终列，`created_at`、`updated_at` 改为 `TEXT NOT NULL`，与 T6 当前必填契约一致。保留 origin_mod_id 可空及当前内容关联结构。
- [x] 不创建 `legacy_imports`、`project_assets`、`context_proposals`、`schema_migrations`、`knowledge_notes_next`。
- [x] 对照 001–012 的最终结果检查任务会话索引、日志索引、记忆复用索引、项目使用索引、唯一任务快照和反馈外键，避免只复制 CREATE TABLE 而遗漏约束。

#### T4.2 明确初始化行为

采用单份 schema 加 `PRAGMA user_version = 1` 标识当前结构，不做版本循环升级。将连接级初始化提取为 `initialize_schema(&mut Connection)`，只供现有数据库模块和其测试使用，不新增通用迁移层。

| 数据库状态 | 处理 |
| --- | --- |
| 无用户表且 user_version 为 0 | 一个事务执行当前 schema 并设置 user_version=1 |
| user_version 为 1 且无旧 schema_migrations 标记 | 按当前库继续启动；正常 SQL 错误按现有错误路径报告 |
| 存在旧 schema_migrations，或非空库版本为 0 | 明确报“不支持旧开发数据库”，不导入、不升级、不删库 |
| 其他 user_version | 报结构版本不支持，不猜测转换规则 |

- [x] 在修改 journal_mode 或执行恢复写入之前识别不支持的数据库，避免拒绝旧库的路径先执行业务修改。
- [x] 空库检查排除 SQLite 内部 `sqlite_%` 对象；未知非空结构不得当作新库覆盖。
- [x] 结构与版本标记在同一事务内完成；失败不留下成功版本标记。
- [x] 保留 SQLite 版本检查、WAL 校验、外键启用、busy timeout 和路径保护。
- [x] 不捕获所有建库错误后自动删库重建；不提供隐式旧数据回退。

这会使现有开发数据库无法直接启动新代码，是预期影响。若后续需要实际运行，先由用户明确授权选择独立测试数据目录或备份后的开发数据重置；本任务不执行这两种操作。

#### T4.3 删除升级实现及同步夹具

- [x] 删除旧版本循环、`migrate_memory_on`、`backfill_content_links_on`；保留当前保存/编辑/删除用到的 `remember_content_on`。
- [x] 从生命周期删除 SQL 中移除三张退役表；保持外键相关删除顺序。
- [x] `src/contracts/project-context.ts` 的 `ContextProposal` 当前仅检索到定义：实施时再次核对引用，确认仍无消费者后删除；保留当前使用的 `ContextChange`。
- [x] 替换 database、knowledge、project_context、project_lifecycle、verification、delivery_metadata 和 pi/submissions 中引用旧 SQL 的测试夹具。
- [x] 夹具优先直接执行当前 schema；已有最小 schema 的测试应避免重复建表，并保留其行为断言。删除仅断言旧迁移的用例，不以空测试代替。
- [x] 全部 `include_str!` 引用替换完后，删除 001–012 脚本。

验收：生产环境只有一条当前建库路径；无旧表查询残留；现有开发库不会被自动改造或删除；新库具备全部当前业务结构。

### T5：统一交付及安装契约

- [x] V01：`ArtifactVersion.summary` 删除缺省，TypeScript 改为必填；保留登记入口非空校验。卡片/列表/详情直接展示摘要，删除“摘要缺失”专属回退，不删除独立的文件数、时间信息。
- [x] V02：session_id 删除缺省，前端 sessionId 改为必填；`VersionConversationLink` 直接使用来源会话，删除查询任务反查。保留“原会话已删除/不存在”的当前生命周期处理。
- [x] V03：`previousTarget` 和上下文面板只读取 lastDeploymentDirectory，不反查安装记录；新项目无目录仍合法，继续进入现有选择目录流程。
- [x] V04：删除 backup_directory / backupDirectory、RollbackStatus 的 Succeeded/Failed 及专属展示；同步安装构造器、类型和夹具。
- [x] 保留 NotNeeded、NotAttempted、安装失败、目标状态不确定及未回退说明；不新增备份或回退功能。

验收：当前交付结果不依赖任务反查或旧字段补齐；无来源会话和无目录这两种情形不混淆；失败安装仍能如实展示。

### T6：收紧当前必有字段

每一小组都按“生产者 → Rust 模型 → SQL/JSON 读取 → TS 契约 → 消费者 → 夹具”完成，不批量替换全部 optional/default。

| 项目 | 调整 | 必须保留 |
| --- | --- | --- |
| F01 | 项目 revision、archived、description 和会话 name 删除缺字段 default；TS 同步必填 | 空名称、空描述；新会话占位；SQL 权威列覆写 payload |
| F02 | context_snapshot_id 从 Option<String> 改 String，TS/runtime 同步 | bootstrap path/hash 可缺，preparing 阶段及失败记录 |
| F03 | program/args 删除 default；source_revision、started_at、duration_ms 改必填 | pid、exit_code、startup_error 可空；output_tail 的 default 及空值省略 |
| F05 | field_sources 删除 default；ReferenceSet.source_report_id 改 String | 空 fieldSources、空 referenceSets；未设置参考集合的新项目 |
| F06 | user_edited、memory_key 删除 default；created_at、updated_at 改 String | 手工记忆和删除后的来源可空；SourceRef 各种证据的合法可选字段 |
| F07 | SourceOperation.directory 删除 default | 正常文件/目录区分及操作恢复 |

具体步骤：

- [x] 在 bridge 创建 TaskContext 前生成 snapshot ID，直接赋给必填字段；`snapshot_on` 使用同一个 ID。快照与 preparing 任务继续同事务写入，不改变提交/失败顺序。
- [x] 修改 bootstrap 身份比对、bridge ACK 比对；仅去掉 Option 包装，不移除 mod/session/task/generation/hash 等身份约束。
- [x] 删除 `project_context::check_task_references`、`knowledge::evidence` 中仅因快照 ID 缺失而跳过校验的旧分支；保留查询失败和身份不匹配的错误。同步全部 TaskContext 夹具。
- [x] CommandReceipt 的 workspace/交付验证改用必填 source_revision 比较；不因为 Option 消失而删除源版本匹配检查。更新 TS 当前消费者需要的必填元数据。
- [x] 调查报告应用直接写 source_report_id 字符串；证据收集由 filter_map 改为 map，保留去重和报告归属检查。
- [x] 长期记忆的手工/自动创建、编辑和仓储投影同步时间类型及 SQL NOT NULL；编辑器使用当前 updatedAt，删除“历史记录”回退。保留 SQL 列覆盖 payload 的权威关系。
- [x] 对每个改动字段检查是否存在 `skip_serializing_if`；不能产生“当前写出省略，当前读入却必填”的自相矛盾。

验收：当前生产者写出的对象可以被当前读入；真实合法空值仍接受；旧缺字段对象不再通过专属默认值伪装成完整当前数据。不要宣称所有输入都新增了严格非空校验，除非原契约本就要求。

### T7：删除旧交付协议触发分支

- [x] 删除 bridge 对 prepare_delivery 的 tool_execution_end 补登记分支及其专属状态处理。
- [x] 保留 `gen-mod:register-delivery` 请求、登记回执、异步处理器及共享 `register_prepared_delivery`。
- [x] 保留工具结束事件的普通消息/状态处理，不删除整个事件分支。
- [x] 核对 runtime 的 prepare_delivery 仍等待原生登记成功才返回成功；失败继续沿现有路径传播。

验收：正常工具调用只有新握手登记入口；普通结束事件不再二次登记；登记失败不会显示成功交付。

### T8：集中复核与文档收尾

- [x] 按第 3 节覆盖全部 29 个审核编号，分别标记已清理、明确保留、未完成及原因。
- [x] 对照第 2 节逐项确认未误改，重点检查 S06、R02、F04、F08。
- [x] 用差异确认不涉及构建机制、游戏适配、Skills 路径策略或真实开发数据删除。
- [x] 更新审核文档的实施状态，保留原审核结论与实际执行结果的区别。
- [x] 如修改了源文件，按项目规则执行 `codegraph sync`、`codegraph status`；索引操作不作为功能验证。
- [x] 汇总实际检查命令及结果、未执行的测试/运行验证、旧开发库不能直接启动的影响。不自动提交或推送。

## 7. 检查与验证方案

### 7.1 实施期间可执行的静态核对

在仓库根目录执行；按每个任务限定文件或模块，不运行自动修复：

```powershell
git status --short
git diff --stat
git diff --check
rg -n 'list_migration_errors|list_legacy_assets|restore_legacy_asset|asset_import|asset_process|legacy_imports|project_assets|context_proposals' src src-tauri/src runtime
rg -n 'migrate_memory_on|backfill_content_links_on|include_str!.*migrations' src-tauri/src
rg -n 'context_snapshot_id|source_report_id|source_revision' src-tauri/src/pi src-tauri/src/project_context src-tauri/src/knowledge src-tauri/src/artifacts
```

前两组退役符号检索在生产代码中应无匹配；如为明确的拒绝旧库测试或说明文本，逐条解释。必填字段检索预期仍有正常使用，不能以零匹配作为成功条件。检查新增文件内容，`git diff` 不覆盖未跟踪文件。

### 7.2 建议的行为验证（执行前须明确授权）

不新增测试依赖，不为简单删除写镜像测试。修改现有夹具是保持代码一致性；以下新增断言集中于初始化、事务和当前合法缺值这些实际风险。

| 场景 | 预期 |
| --- | --- |
| 全新内存库初始化 | 当前业务表/索引/约束齐全，无退役表，版本标记正确 |
| 同一当前库再次初始化 | 不重复建表，不清除记录 |
| 旧库或未知非空库 | 返回明确错误，原有 schema、业务行和 user_version 不变 |
| 建库事务故意失败 | 无成功版本标记，不留下部分建表结果 |
| 当前完整 JSON 往返 | 收紧后的项目、任务、命令、记忆可正常读回 |
| 删除一个已收紧必填字段 | 读取失败；与合法空集合、空字符串分别断言 |
| 命令无输出/启动失败 | outputTail 省略仍可读，pid 等合法缺值仍可读 |
| preparing 任务无 bootstrap | 保持可持久化；正式运行仍要求路径、哈希及快照身份一致 |
| 项目记忆缺行及首次保存 | 返回空记忆，revision=0 保存仍成功 |
| Skills 可选配置、任务未结束 | F08 和 F04 当前契约不变 |
| 当前登记交付及结束事件 | 新握手成功一次，结束事件无补登记，失败不产生成功版本 |
| 文件导入/移动及删除恢复 | 当前日志分支继续工作，无旧表查询错误 |

推荐优先运行的现有 Rust 测试过滤器（需先核对实际测试名和覆盖）：

```powershell
cargo test --manifest-path src-tauri/Cargo.toml --lib storage::database::tests
cargo test --manifest-path src-tauri/Cargo.toml --lib project_context
cargo test --manifest-path src-tauri/Cargo.toml --lib knowledge
cargo test --manifest-path src-tauri/Cargo.toml --lib project_lifecycle
```

这些命令只选择部分测试运行，但会编译整个 Rust library。交付、source_files、pi 的关联用例按实际模块再选择，不能把“过滤后运行 0 个测试”记为验证通过。

如需编译/类型验证，可申请执行 `cargo check --manifest-path src-tauri/Cargo.toml --lib` 与 `pnpm exec vue-tsc --noEmit`；两者覆盖整个对应工程，理由是共享 Rust/TS 契约跨模块调整无法仅靠单文件检查发现全部构造器遗漏。运行前按项目规则核对配置和脚本，不默认执行 build。

当前 Vitest 配置仅覆盖既有 services/pi 和 useSourceFileActions 测试，不假定存在 Vue 组件测试环境。界面交互应使用获授权的实际应用验证，或继续明确标记静态未验证，不新增依赖补齐测试环境。

## 8. 完成标准、风险与回退

完成条件：全部清理项和直接消费者一致；所有保留项未改变；schema 与字段契约匹配；现有夹具不再引用已删除文件；实际执行的检查结果清楚，未执行的验证明确列出。

主要风险及控制：

- schema 合并遗漏列或索引：对照最终 SQL 逐表核对，使用新库验证，不直接跳过旧脚本后尝试启动。
- 删掉共享恢复：保留新建、登记、删除和异常恢复调用链，按函数调用者区分专属与共享。
- 字段收紧破坏当前序列化：同步检查每个生产者与省略规则，特别是 outputTail 和 preparing 阶段。
- 前后端契约半更新：模型、消费者、构造器和夹具作为同一修改单位。
- 开发数据不可读：明确拒绝旧库，保留数据，不自动处理。实际数据处置另行授权。

代码回退仅针对本次明确改动，先检查工作区，不使用 reset/clean 丢弃其他工作。新 schema 数据不承诺可由旧代码读取；需要回到旧代码运行时，应使用与其匹配的数据副本或独立目录，不能把回退代码当作数据降级方案。

## 9. 本次计划编写的验证边界

本次仅核对当前源码、审核文档、SQL 文件与相关引用并编写计划，没有实施上述任务。未运行测试、Lint、类型检查、构建或应用；未操作本机业务数据库，未提交或推送。


## 10. 实施记录（2026-11-11）

用户已授权按本文开始变更。T1–T7 的代码清理和 T8 的静态收尾已完成；第 7.2 节行为验证未执行，复选框表示修改及静态核对完成，不表示测试验收通过。

- T1：移除 JSON 导入、迁移错误 IPC/展示，以及两个启动 backfill；保留当前初始化和恢复调用。
- T2：移除旧素材模块、界面、IPC、源码恢复入口及旧日志分支；同步删除专属测试，没有删除本机数据。
- T3：移除 candidate 状态支持，保留用户编辑、停用、删除保护。
- T4：新增单份 `storage/schema.sql`，包含 18 张当前业务表；移除 12 份升级脚本及旧记忆搬迁，使用 user_version=1 标识。旧库/未知非空库拒绝启动，不迁移或重置。已改写现有夹具，并添加当前建库幂等、拒绝旧库不改写的测试代码，但未执行。
- T5：统一交付摘要、来源会话必填契约，删除旧目录反查、备份字段及退役回退状态。
- T6：收紧计划列出的字段，同步 Rust、TS、runtime 和相关夹具。额外发现 `artifacts/content_check::restricted_for_task` 存在同一快照缺失跳过分支，随 F02 一并删除；这是必填快照的直接消费者。DeliveryMetadata 的快照字段仍可空，因为当前锚点构造会先写 None，再补登记元数据。
- T7：仅删除 tool_execution_end 的旧补登记，保留新请求与共享登记函数。
- T8：已核对全部 29 项映射，S06 的正常空记忆、R02、F04、F08 及构建机制保持。审核文档保留历史基线记录，并追加实施状态。

实际验证：

- `git -c core.safecrlf=false diff --check`：通过。初次发现的两处文件末尾多余空行已修正。
- 对本次修改的 `src` 下 `.ts` / `.vue` 文件执行 `./node_modules/.bin/eslint.cmd <修改文件列表>`：初次发现 SourceBrowser 的 reveal 解构变量未使用，删除该遗留后单文件复查通过；其余修改文件初次检查无错误。
- 定向 `rg`：旧 IPC、旧素材恢复、迁移 helper、旧 SQL 文件引用无残留；退役表名仅保留在新建库测试的“不得创建”断言中。
- `codegraph sync`、`codegraph status`：成功，索引显示 up to date。
- 未运行测试或应用，未执行构建。用户随后明确授权全工程 Rust 编译与前端类型检查，两项均已完成，结果见下。
- 未提交、未推送，未操作真实业务数据库。


### 编译与类型检查补记

用户明确授权后执行：

- `cargo check --manifest-path src-tauri/Cargo.toml --lib`：退出码 0。仅提示增量缓存无法硬链接、改用复制，不影响编译检查结果。
- `pnpm exec vue-tsc --noEmit`：首次发现 SourceBrowser 中普通文件选择仍依赖的 `nativeCall` 导入被误删，连带导致回调参数缺少类型。恢复共享导入后重新执行，退出码 0。
- `./node_modules/.bin/eslint.cmd src/features/workbench/SourceBrowser.vue`：针对修正复查通过，退出码 0。
- `codegraph sync`、`codegraph status`：成功，索引最新；`git -c core.safecrlf=false diff --check` 无错误。

仍未运行测试、应用或生产构建。`cargo check --lib` 不编译 `cfg(test)` 用例，已有和新增测试代码尚未通过测试编译/执行验证。未提交或推送。
