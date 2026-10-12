# 多会话并发、运行中引导与 SQLite 实施计划

> 状态：用户已授权实施（含 rusqlite/SQLite），任务 1–7 源码已实施并通过限定静态检查；所有运行 Gate 尚未执行。由主代理执行，未使用子代理、未提交。

**目标：** 多个会话可以同时调用 AI，运行期间自由切换会话，并向当前任务发送补充要求；左侧持续展示各会话的运行状态，应用元数据使用 SQLite 持久保存。

**架构：** 原生按 `(modId, sessionId)` 管理独立 pi 进程，前端按同一键维护会话状态，当前路由只决定展示对象。同项目继续共用 `source/`，通过原生工作区协调器管理工具操作，不对整轮 AI 运行加锁。SQLite 保存应用元数据，pi JSONL、源码和交付文件保留现有文件格式。

**技术栈：** Vue 3、Pinia、Tauri 2、pi 1.1.0 RPC；计划新增 Rust `rusqlite`（bundled SQLite），不引入前端 SQL 插件。

**规格依据：** 本文第二至第六节，以及 [当前项目上下文](./project-context.md)。本文保留实施规格；下方勾选表示源码实施与静态核对完成，不表示运行 Gate 通过。

## 一、现状与原因

实施前代码核对基准：本地提交 `77d5bd2`，核对日期 2026-10-09。

- `src/stores/session.ts` 只有一个 client 和一组 chat、task、busy、contextUsage；`open()` 忙碌时拒绝切换。
- `src-tauri/src/pi/bridge.rs` 的 `Inner` 只有一个 `process`；`spawn()` 先停止原有进程，`connect_pi` 和 `start_mod_task` 拒绝已有活动任务。
- `ChatPage.vue` 的路由离开和更新守卫阻止忙碌时导航；侧栏新增、会话管理和版本页面也引用全局 `session.busy`。
- 每个项目只有一份 `mods/<modId>/source`；任务日志、staging 和历史按任务或会话隔离。不能仅解除 UI 禁用，否则进程会被替换，源码也可能相互覆盖。
- 草稿目前按项目保存，运行时间来自 task.json，上下文用量只保留在前端内存。
- 内置 pi 的 `steer` 支持文本和图片，返回 `queued` 或 `handled`。消息在当前 assistant turn 的工具调用完成后、下一次模型调用前接入，不会立即中断正在执行的命令。`agent_settled` 才表示本次会话级运行不再有自动工作。

以上是源码与内置协议核对，未启动应用、未执行并发或模型验收。

## 二、已确认范围与交互

用户已确认：

1. 多个会话可以同时执行、并发调用 AI，任务运行时可以切换会话。
2. 同项目多个会话修改同一份数据，不建立独立源码分支，不按整轮任务排队。
3. “引导”指 AI 运行中发送补充要求，作用于当前任务。
4. 会话列表显示类似 Codex 的圆环运行标记。
5. 可以采用 SQLite；最初只写计划；后续已授权本计划及依赖实施，仍不直接试验或修改现有应用数据。

目标交互：

- A 正在运行时，可以打开 B、创建新会话、浏览全部项目、设置和版本；A 的事件继续处理与持久保存。
- 回到 A 显示已有内存内容和最新运行状态，不重新启动进程、不重复发送提示词。
- 运行中输入框仍可输入、添加附件；有内容时显示发送引导按钮，同时保留独立停止按钮。Ctrl + Enter 按当前会话状态发送普通需求或引导。
- 准备中暂不能引导；自动/手动压缩期间保留输入，暂禁用提交；等待源码操作时仍可向活动 Agent 引导。手动压缩仍只允许目标会话空闲。
- 正在运行的目标会话不能删除，其他空闲会话可管理；重命名不停止任务。禁止操作的条件按目标会话或目标资源判断，不能用“任一会话忙碌”禁用全部界面。
- 圆环放在会话行右侧，不恢复左侧会话图标。普通灰色循环圆环表示准备、运行、压缩；等待共享工作区操作时配悬浮说明。停止、结束或断连后不继续旋转。
- 保留现有正文、用时、当前活动、最新过程文字与掠光；不增加聊天正文前的重复 loading，不展示思考文本。

不包含：首次使用向导、云同步、独立工作区自动合并、新模型接入、运行中修改模型/强度、自动安装产物、版本数量限制。

## 三、会话运行架构

### 3.1 身份、状态和事件

会话键同时包含 modId 和 sessionId。每个运行槽独立保存进程、Windows Job、stdin、事件通道、generation、当前 TaskRecord、工具参数和日志句柄。

所有桥事件和命令使用完整身份：

```ts
interface RuntimeIdentity {
  modId: string
  sessionId: string
  generation: number
}
type RunPhase = 'idle' | 'preparing' | 'running' | 'compacting' | 'stopping'
  | 'finished' | 'cancelled' | 'interrupted' | 'failed'
```

- `generation` 在单个会话内递增；旧槽移除后也不能重用旧 generation。身份、代次和 requestId 一并检查，防止迟到事件更新新进程。
- connection、RunPhase、等待工作区操作分别保存，不能把断连、压缩和失败混为同一状态。
- 注册表锁只用于查找/替换运行槽；进程等待、源码快照、文件复制、数据库写入不占用整个注册表锁。
- 单会话仍只有一个活动任务。新一轮继续按现有方式绑定新的 taskId 并恢复同一历史；steering 沿用当前任务，不触发该启动流程。
- 停止、重启、断开只作用于指定会话；应用退出时回收所有 Job，确认命令后代结束后再释放工作区占用。
- 查询历史优先 SDK helper 或会话缓存，不为查看空闲历史强制创建 pi 进程。发送普通需求和手动压缩时按需连接；空闲进程可由明确断开或退出回收，首期不增加自动淘汰配置。

### 3.2 前端生命周期

- 将 store 改为会话注册表，selectedKey 只决定当前展示；每个会话持有独立 client、reducer 队列、请求映射、上下文、计时、错误和 uiRequest。
- 事件按身份更新对应会话，后台会话也接收 terminal、artifact、changes、queue 等事件。
- 已有会话只切换引用，首次查看历史才读取文件；路由异步读取带会话键与加载代次校验。
- draft 按会话保存；发送捕获目标身份及输入快照，完成回调不能清空用户后来切换到的会话输入。
- 未发送附件按会话保留在内存中，切换不丢失；本期不增加附件草稿跨重启保存。异步文件读取绑定发起会话。
- 后台事件不重绘所有历史列表；只更新对应会话状态与侧栏摘要。当前页面保持原有自动贴底和用户上滚停止跟随行为。
- 右侧日志、文件差异、上下文和扩展请求都绑定当前会话。后台存在需用户响应的请求时显示“等待输入”提示，切入后展示原请求。

## 四、同项目共享源码协调

### 4.1 为什么需要工具级协调

模型请求可以并发，但多个进程写同一文件、共用构建输出或在交付时更改源码会导致覆盖和错误登记。计划采用**同项目工具操作协调 + 文件版本检查**；不锁住整轮会话，不复制独立源码目录，不自动合并两个模型的修改。

协调器在 Rust 中按 modId 管理，跨 pi 进程有效。不能用 extension 进程内的 Map 充当跨会话锁。

| 操作                             | 同项目规则                                                                   |
| -------------------------------- | ---------------------------------------------------------------------------- |
| 模型请求、公开过程文字、steering | 可并发，不持工作区锁                                                         |
| read/grep/find/ls、任务初始快照  | 共享读取租约；写操作期间等待，避免观察半写入状态                             |
| write/edit                       | 独占租约，拿锁后检查文件基线，完成写入后立即释放                             |
| run_command                      | 独占租约覆盖整个命令及其后代生命周期；任意命令可能改文件，首期不猜测其只读性 |
| prepare_delivery                 | 独占租约覆盖复制、打包、原生校验与登记，保证取样和校验一致                   |

这意味着同项目模型可以同时运行，但长构建期间其他会话的文件工具可能等待；界面应如实显示等待原因，仍允许切换和引导。不同项目不共享工作区锁。

### 4.2 文件基线、命令和变更归属

- 为 read/write/edit 接入包装工具，保留 pi 原工具执行能力和当前路径校验。读取时在租约内记录完整文件哈希；已有文件写入必须具备该会话的读取基线，新建文件记录“不存在”基线。
- 写入时重新比较哈希；变化则拒绝覆盖，工具返回“文件已由其他会话修改，请重新读取后修改”。edit 的文本匹配校验继续保留。路径仍执行现有链接、越界和 Windows 大小写规则校验。
- 写入租约内记录操作前后哈希、taskId、toolCallId 和源码 revision；成功后通知同项目其他会话“源码已变化”，让下一步重新读取相关文件。
- run_command 记录观察到的 sourceRevision；排队后 revision 已变化时返回需重新检查的工具结果，不直接执行基于旧源码的命令。获得租约后记录实际执行前后 manifest。
- 任意脚本的业务含义不能自动合并；锁与哈希只能防止应用工具的并发写入和明显过期基线，不能保证用户外部编辑器或任意脚本不会产生语义冲突。
- 任务开始到结束的目录差异可能包含别的会话修改，不得直接标为“本会话修改”。右侧区分“本任务工具操作记录”与“共享源码变化”；命令造成的文件变化归执行该命令的任务记录。
- 构建命令回执附带源码 revision。交付所需构建记录必须匹配交付开始时源码 revision，且任务身份、必要原始脚本、fresh output、哈希、ZIP 等原有验证继续执行。源码已变化时要求重新构建，不能复用其他会话的成功记录。
- sourceRevision 表示可构建输入的变化，任务专属 `.delivery/<taskId>`、staging 和日志写入不使它递增；其他生成目录的排除规则来自通用 skill 元数据，不写游戏分支。旧命令回执没有 revision 时不能用于新交付，已登记旧版本仍可浏览、安装和导出。

### 4.3 租约与异常

- 使用现有 extension_ui_request/response 桥承载内部工作区 acquire/release 请求，内部请求不弹用户输入框；带 identity、taskId、toolCallId、operation 和 leaseId。
- bridge 收到 acquire 后异步等待，必须先释放会话/注册表锁；否则 release 和其他事件无法处理。
- 操作包装器用 finally 释放租约。prepare_delivery 内部 runCommand 复用已有租约，不能嵌套再次申请而死锁；原生登记同理。
- 取消排队操作时撤销等待项。持租约的命令被停止时先回收后代再释放，不能只因超时就放行下一次写入。
- 锁状态仅属于当前运行时，不从 SQLite 恢复成活跃锁。进程退出后清理其等待项和租约，并记录 interrupted/cancelled。
- 租约等待请求超时必须取消原等待项；如果授予与超时竞态发生，原生撤销该未开始的租约，不能留下无人使用的占用。排队采用先到先服务，不能因不断进入的读取而饿死写入。

## 五、运行中引导

### 5.1 提交链路

新增受控 IPC `steer_mod_task(identity, taskId, inputId, message, images)`；前端不获得任意 prompt 权限。

1. 原生确认会话属于项目、代次和任务匹配，当前有可接收引导的活动 Agent，且未停止/压缩。
2. 复用普通发送的附件大小、格式、路径和只读校验；文本附件仍拼入 message，图片使用 pi images。
3. 先保存 inputId、任务身份和 submitted 状态，再向该进程发送 RPC：

```json
{ "id": "<inputId>", "type": "steer", "message": "将范围改为 140m", "images": [] }
```

4. `queued` 表示已接收等待处理，`handled` 表示扩展消费；不能将二者写成“已经执行完毕”。根据 queue_update 与正式用户消息事件更新展示。
5. 正式消息进入 pi 历史后替换本地引导占位，避免双份消息。不要继续只用一个 pendingUserId；同一运行允许多个待处理输入。

pi 不承诺正式用户消息携带应用 inputId。引导采用 one-at-a-time 模式，按实际 queue_update、提交序列和正式消息内容关联，并保存匹配的历史 entryId；扩展转换/消费导致无法可靠关联时标记未确认，不能仅凭“下一条用户消息”强制认领。重复正文按提交顺序分别处理；不向模型正文注入应用 ID。

### 5.2 时序与恢复

- 引导不新建 taskId、不重启 pi、不重置本轮计时；执行摘要可显示“补充要求已提交”。
- conversationRounds 不能按每条用户消息无条件新建独立运行轮次；同一 taskId 内的正式引导作为本轮补充，恢复也使用持久输入关联。
- 若提交时任务刚结束，拒绝该次引导并保留输入，提示改为发送新需求；不隐式启动新任务。
- 正在等待源码工具租约时引导会在 pi 可接收的边界进入，不宣称立即取消该工具；需要停止时使用独立停止按钮。
- submitted 但无响应、收到 queued 后崩溃但未确认进入历史等情况标记“状态未确认/未处理”，重启不自动重发。用户可检查历史后重新发送，避免重复修改。
- pi 内存队列不等于持久任务队列；SQLite 保存应用收到的引导及其状态，不声称重启能继续原来的进程。
- 首期只做 steer，不增加 follow_up、定时任务、队列编辑或自动重试。

## 六、SQLite 数据方案

### 6.1 数据权威与边界

数据库保存到应用 local data 根目录 `gen-mod.sqlite3`，Rust 提供受控访问，Vue 不执行 SQL，API Key 继续来自 env。

| 数据                                                            | 实施后权威来源                                                     |
| --------------------------------------------------------------- | ------------------------------------------------------------------ |
| 项目信息、版本条目、安装记录、部署位置                          | SQLite projects.payload；先保留现有 DTO 形状，避免无关的全量表拆分 |
| 会话标题、skill 快照、历史文件关联、只读状态、草稿、强度        | SQLite sessions                                                    |
| 任务状态、时间、命令回执、登记版本关联                          | SQLite tasks.payload 与索引字段                                    |
| 运行摘要日志、引导状态、上下文快照、最后活动                    | SQLite 对应表                                                      |
| 原始消息、工具结果、压缩历史、已发送附件                        | pi JSONL；数据库只存应用关联，不能重复存完整聊天作为第二权威       |
| 源码、任务工具全文/命令日志、staging、版本文件、ZIP、不可变清单 | 文件系统，现有目录继续使用                                         |
| skills registry.json、模型配置、env                             | 原有文件，不属于此次迁移                                           |

上下文快照包含 tokens（可空）、percent（可空）、contextWindow、estimated、sampledAt 和 generation；历史快照展示为上次统计，不能伪装成当前准确值。压缩后写入未知状态，保留此前快照但不继续作为当前占用显示。

SQLite 建议启用 foreign_keys、短事务、busy timeout；采用 WAL 时检查实际启用结果并保持数据库位于本机应用数据目录。WAL 仍只有一个写入者，因此文件/进程操作不能长时间占用数据库事务。采用 rusqlite 的 bundled 方式，执行时锁定经过官方发布说明核对的版本及 SQLite 修复版本，不依赖用户安装数据库。

### 6.2 最小表结构

迁移中建立以下表，字段采用 SQL snake_case、IPC 保持 camelCase：

```sql
CREATE TABLE schema_migrations(version INTEGER PRIMARY KEY, applied_at TEXT NOT NULL);
CREATE TABLE projects(
  mod_id TEXT PRIMARY KEY, payload TEXT NOT NULL, revision INTEGER NOT NULL DEFAULT 0
);
CREATE TABLE sessions(
  mod_id TEXT NOT NULL REFERENCES projects(mod_id), session_id TEXT NOT NULL,
  payload TEXT NOT NULL, draft TEXT NOT NULL DEFAULT '', thinking_level TEXT NOT NULL DEFAULT 'medium',
  generation INTEGER NOT NULL DEFAULT 0, last_activity_at TEXT,
  PRIMARY KEY(mod_id, session_id)
);
CREATE TABLE tasks(
  task_id TEXT PRIMARY KEY, mod_id TEXT NOT NULL, session_id TEXT NOT NULL,
  status TEXT NOT NULL, created_at TEXT NOT NULL, finished_at TEXT, payload TEXT NOT NULL,
  FOREIGN KEY(mod_id, session_id) REFERENCES sessions(mod_id, session_id)
);
CREATE INDEX tasks_session_time ON tasks(mod_id, session_id, created_at);
CREATE TABLE runtime_logs(
  log_id INTEGER PRIMARY KEY, mod_id TEXT NOT NULL, session_id TEXT NOT NULL,
  task_id TEXT, timestamp TEXT NOT NULL, payload TEXT NOT NULL
);
CREATE INDEX runtime_logs_session ON runtime_logs(mod_id, session_id, log_id);
CREATE TABLE steering_inputs(
  input_id TEXT PRIMARY KEY, mod_id TEXT NOT NULL, session_id TEXT NOT NULL, task_id TEXT NOT NULL,
  generation INTEGER NOT NULL, submitted_at TEXT NOT NULL, status TEXT NOT NULL, payload TEXT NOT NULL
);
CREATE TABLE session_stats(
  mod_id TEXT NOT NULL, session_id TEXT NOT NULL, sampled_at TEXT NOT NULL, payload TEXT NOT NULL,
  PRIMARY KEY(mod_id, session_id),
  FOREIGN KEY(mod_id, session_id) REFERENCES sessions(mod_id, session_id)
);
```

payload 使用 serde DTO；projects.payload 不重复保存 sessions 和会话草稿，读取项目时组合 DTO。运行日志与引导关联采用显式清理规则：删除会话仍保留原有任务/日志/交付，不通过级联删除抹掉记录；保留归档会话行供 tasks 外键引用，列表排除已删除标记。

项目字段更新使用局部仓储方法与 revision 检查，禁止将旧整份项目覆盖数据库。并发版本登记、自动标题、部署位置更新必须各自读改写于短事务内。

### 6.3 迁移与文件一致性

- 应用开始接受运行命令前，导入旧 mod.json、task.json、session.runtime.jsonl。校验 ID、归属、路径与数据格式后按项目事务导入，迁移记录防止重复。
- 导入保留 sessionId、sessionFile、taskId、版本/部署关联、时间、skill 哈希；不创建新 pi 历史、不移动源码或 ZIP，不更改 skill hash 校验。
- 原有项目草稿导入最后一个可用会话；其他会话空草稿。无会话的项目保留 initialDraft，首个新会话消费一次。之后每个会话独立保存，不相互覆盖。
- 导入失败的项目不启用写入/执行，显示明确迁移原因；成功项目可以正常使用。只查询有效项目目录，不把隐藏临时目录当项目。
- 迁移完成后，mod.json/task.json/runtime.jsonl 不再作为活动元数据双写目标；旧文件保留为迁移前资料。不可变 version.json/install.json、命令和工具全文日志仍保留。
- 所有现有元数据读写调用统一改走 repository，包括 bridge、sessions、mods、skills 的会话刷新、changes、artifacts、install、files。不能只改侧栏查询而留旧写入路径。
- 文件操作和 SQLite 不能做一个跨介质原子事务：版本登记先生成完整临时目录再重命名，后提交数据库；失败清理仅本次目录，启动核对孤立目录，不自动删除用户原有文件。删除版本保留现有 tombstone 流程并记录操作状态。
- 安装文件复制失败沿用 uncertain；数据库写失败不宣称安装成功。业务元数据更新与恢复记录分别持久，不能因“数据库事务回滚”声称部署文件已经回退。
- 重启把上次未终结的 preparing/running/stopping 标记 interrupted；保存历史和源码，等待用户继续，不自动恢复进程或重放引导。

## 七、实施任务与检查点

各任务按顺序完成；涉及同一文件时不并行修改。执行前读取本计划及 project-context，核对工作区差异，先使用 CodeGraph 定位当前入口。

### 任务 1：SQLite 仓储与旧数据迁移

**新增：** `src-tauri/src/storage/database.rs`、`repository.rs`、`migration.rs`、`src-tauri/src/storage/migrations/001_initial.sql`。

**修改：** `Cargo.toml`/`Cargo.lock`、`lib.rs`、`storage/mod.rs`/`mods.rs`/`models.rs`，以及第六节列出的元数据调用方。

- [x] 创建数据库初始化、版本迁移、事务仓储与会话软删除接口；保留前端既有项目/版本 DTO。
- [x] 将所有活动元数据读写替换为仓储调用，删除本次替换产生的旧写入路径；不迁移 skills/config。
- [x] 实现按项目幂等导入与错误隔离，应用接收任务前完成迁移。
- [x] 补齐 artifact/install 与数据库交界的恢复记录，保留直接覆盖和无限版本手动删除规则。
- [x] 静态核对不存在继续写活动 mod.json/task.json 的入口。

**Gate：** 使用临时副本验收旧 sessionId/历史/版本/安装记录不丢失，重复迁移不重复，失败项目不被半迁移后执行。此运行验收须用户明确授权，禁止直接试验用户唯一数据目录。

### 任务 2：原生多会话运行注册表

**新增：** `src-tauri/src/pi/manager.rs`。

**修改：** `pi/bridge.rs`、`runtime.rs`、`tasks.rs`、`sessions.rs`、`lib.rs`。

- [x] 按会话键拆运行槽、代次和进程锁，IPC 明确目标身份；所有 stdout/stderr/task/artifact 事件附带身份。
- [x] start/connect/abort/disconnect/restart/delete 只操作目标槽；单会话重复开始由原生拒绝。
- [x] 启动和停止不持注册表全局锁，退出遍历所有槽并回收 Windows Job。
- [x] 一次任务异常不终止其他槽，数据库记录目标终态。

**Gate：** 两个不同项目并发时分别恢复历史、运行、停止；A 停止/退出不影响 B，旧 generation 事件被丢弃。此阶段同项目并发暂不开放，任务 3 通过后开放。

### 任务 3：共享源码工具协调与交付一致性

**新增：** `src-tauri/src/pi/workspace.rs`、`runtime/extensions/workspace.ts`。

**修改：** `bridge.rs`、`tasks.rs`、`src-tauri/src/changes.rs`、`artifacts/registry.rs`、`runtime/extensions/mod-tools.ts`/`commands.ts`/`delivery.ts`、`scripts/prepare-runtime.mjs`（需要新增扩展复制时）。

- [x] 建立共享读/独占写租约与内部桥协议，排队不阻塞 bridge 消息处理。
- [x] 包装文件工具，记录读取哈希、写入基线和任务操作记录，保持现有路径限制。
- [x] 命令完整生命周期持租约；prepare_delivery 复用租约，不能嵌套死锁。
- [x] 构建回执绑定 revision，登记校验 revision 与现有 receipt/hash/fresh output；并发版本事务更新不互相覆盖。
- [x] 记录共享源码变化与本任务操作的区别，停止/崩溃后清理等待项并回收持锁后代。
- [x] 同项目并发只在本任务全部链路接入后开放。

**Gate：** A/B 读同文件后 A 修改，B 写旧基线被拒绝；构建期间另一写入等待；构建后被其他会话改源码则原构建不能直接交付；交付中的取消不会泄漏锁或错误登记。

### 任务 4：前端会话注册表与自由切换

**新增：** `src/services/pi/sessionRuntime.ts`，封装单会话状态与事件处理。

**修改：** `stores/session.ts`、`services/pi/client.ts`/`types.ts`、`contracts/session.ts`/`task.ts`、`ChatPage.vue`、`useDraft.ts`、`ProjectSidebar.vue`、`SessionManageDialog.vue`、`ContextPanel.vue`、`ModManagerPage.vue`、`services/native.ts`。

- [x] store 管注册表和 selectedKey；复用单会话 reducer，不把后台事件归入当前页面。
- [x] 路由守卫只处理旧草稿保存，不因任务运行阻止导航；页面卸载不注销后台事件。
- [x] 会话草稿、附件、读取代次和提交回调绑定身份；从缓存切换时立即显示。
- [x] 日志、任务、变更、版本通知和扩展请求按会话/项目归属刷新，消除全局 busy 禁用。
- [x] 运行会话不能删除，空闲会话可以；既有不可变版本浏览、导出、安装不被无关任务禁用，删除与实际版本文件操作互斥。

**Gate：** A 生成时切 B、新建 C、进入设置再返回 A，A 不重启不丢事件；各自输入/附件不串、历史不重复，后台完成可见。

### 任务 5：steering 后端、持久记录与回复归并

**修改：** `pi/bridge.rs`/`tasks.rs`、`storage/repository.rs`、`lib.rs`、`services/pi/client.ts`/`reducer.ts`、`sessionRuntime.ts`、`conversationRounds.ts`、`contracts/session.ts`。

- [x] 实现 steer_mod_task 校验、submitted 持久记录与 RPC 投递，保持原 taskId/进程。
- [x] 处理 queued/handled/rejected/unknown、queue_update 和正式消息确认；替换多条本地占位而非一个 pendingUserId。
- [x] 同一 taskId 的补充消息不被当成重新开始的一轮，恢复后时长与任务归属一致。
- [x] 结束竞态拒绝并保留输入，停止/崩溃未确认项不自动重发。

**Gate：** 连续两条补充文本及图片不重复、顺序与 pi 队列一致；taskId 和进程保持；响应超时或退出不自动重发，不错误显示已执行。

### 任务 6：输入框引导与侧栏圆环

**新增：** `src/features/workbench/SessionRunIndicator.vue`。

**修改：** `ChatComposer.vue`、`ChatPage.vue`、`ProjectSidebar.vue`、`SessionManageDialog.vue`、`RoundActivity.vue`、`ContextUsageIndicator.vue`。

- [x] 运行中同时支持引导发送与独立停止，Enter/快捷键、附件和当前输入状态按第二节执行。
- [x] 会话行右侧绘制圆环，hover 菜单与状态标记不相互挤占；为状态提供 aria-label/悬浮说明。
- [x] 压缩/准备/停止时精确控制提交，不增加重复 loading；reduced motion 下使用静态标记。
- [x] 展示上次上下文统计的采样时间/估算属性，不把过期统计当当前值。

**Gate：** 多个后台会话的圆环同步变化，进入菜单仍可切换；运行/引导/停止操作可明确辨认，原有布局和思考隐藏规则保留。

### 任务 7：重启恢复与范围内验收

**修改：** 本任务只修改前六项直接相关的恢复入口，以及 `docs/project-context.md`、`docs/verification/implementation-progress.md`。

- [x] 启动恢复任务终态和未确认引导，保留同一会话及历史文件路径。
- [x] 对照第二至第六节逐项记录静态检查和运行验收，不把计划场景标为已通过。
- [x] 更新上下文中的单进程、项目草稿、无数据库、全局忙碌禁用等旧描述；只在实现完成后改为已实现。

**Gate：** 重启后历史/产物可恢复、运行圆环不永久保留、不自动重放输入；无 Key 不阻塞无模型阶段实施，真实并发/引导最终由用户补 Key 后验收。

## 八、验证范围与执行约束

本轮已执行下列限定类型/编译和资源准备检查，具体结果见实施记录。默认不运行测试、不启动应用；运行 Gate 由用户使用临时数据副本和补充 Key 后验收。

1. Vue 改动采用 `work/tsconfig.multi-session.json` 限定相关组件、store、contracts 和直接依赖，再运行：

   ```pwsh
   node node_modules/vue-tsc/bin/vue-tsc.js --noEmit -p work/tsconfig.multi-session.json
   ```

2. runtime 扩展采用限定 TypeScript 配置检查；新增工作区扩展需同步 portable 资源：

   ```pwsh
   node node_modules/typescript/bin/tsc --noEmit -p work/tsconfig.runtime.json
   node scripts/prepare-runtime.mjs
   ```

3. Rust 修改跨 pi/storage/artifacts/install，相关编译范围为应用 lib：

   ```pwsh
   cargo check --manifest-path src-tauri/Cargo.toml --lib --locked --offline
   ```

   新依赖首次解析/下载属于实施步骤，不能在锁文件未更新时声称 offline 检查必定可运行；只添加计划批准的 rusqlite 依赖，不替换其他依赖。

4. 检查当前授权文件差异、`git diff --check`；源文件变更结束前依次 `codegraph sync`、`codegraph status`。索引维护不代替验收。
5. 获授权后的针对性运行检查使用临时数据副本和可控 RPC 进程，覆盖多槽隔离、租约取消、迁移重入、引导竞态与交付 revision；真实模型/图片和 UI 最终验收单独记录。
6. 不执行全仓格式化/lint/test，不自动提交或推送，不修改游戏部署目录；已获本轮实施授权，无需再次确认。

## 九、资料依据

- 内置 pi 1.1.0：`runtime/resources/pi-portable-v1.1.0/node_modules/@earendil-works/pi-coding-agent/docs/rpc-commands.md`、`docs/json.md`、`dist/modes/rpc/rpc-types.d.ts`。实施以锁定版本协议为准。
- [SQLite WAL](https://www.sqlite.org/wal.html)：读写并发、单写入者与本机文件系统边界。
- [SQLite 外键](https://www.sqlite.org/foreignkeys.html)：连接启用与约束。
- [rusqlite 官方 API](https://docs.rs/rusqlite/latest/rusqlite/)：Rust 数据访问与事务 API；实施时再核对选定版本。

## 十、本轮进度与验收记录（2026-10-09）

任务 1 → 7 已依次接入。基线与计划相比，原有单进程、全局 busy、项目级草稿及文件元数据入口均已替换；保留既有 skill 哈希保护、历史投影、安装直接覆盖与手动无限版本规则。实现没有源码分支或整轮任务队列。

Vue 限定类型检查（work/tsconfig.multi-session.json）、runtime TypeScript 检查（work/tsconfig.runtime.json）、cargo check --lib --locked --offline 均退出 0，Rust 无警告。已授权依赖下载完成；rusqlite 0.40.2 bundled SQLite 3.53.2，没有升级既有依赖。完整命令和资源准备、差异及 CodeGraph 最终输出记录于 docs/verification/implementation-progress.md。

全部任务的运行 Gate 未执行。用户验收使用临时数据副本，覆盖：

1. 旧历史/产物/安装记录/skill 校验迁移，重复启动幂等，坏项目隔离，异常安装及 tombstone 恢复。
2. A/B 同时调用 AI，A 运行时切 B、新建 C、设置往返；事件、圆环、草稿、附件、任务、扩展请求和后台产物归属。
3. 同项目读写 FIFO、旧 read 后覆盖拒绝、排队取消、长命令及后代进程停止；过期构建回执及交付 revision 拒绝。
4. 连续文本及图片引导、队列先后、同 taskId/进程/计时、退出/超时不重发，恢复后正式历史不重复。
5. 重启终态、历史统计提示、reduced motion、空闲版本操作与无关后台任务同时存在。

没有运行测试、启动应用、调用模型、访问现有应用数据库/游戏部署目录、构建新版安装包、提交或推送。API Key 由用户补充，不影响上述已完成实施。

### 2026-10-10 后续修复

用户实际启动发现 SQLite 项目读取提前反序列化，导致缺少 sessions。已调整 read_on 为先合并 sessions 表，再反序列化；Rust lib 编译通过。任务 1 的实际启动 Gate 仍待用户重新执行，未将此次编译检查记为运行通过，详见实施记录。
