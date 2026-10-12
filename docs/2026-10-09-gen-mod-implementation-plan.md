# AI MOD 桌面工具执行计划

> 历史执行计划：保留首期任务和当时检查记录。2026-10-10 的多会话并发、运行中引导、SQLite 与后续交互见 [当前项目上下文](./project-context.md) 和 [多会话计划](./2026-10-09-multi-session-steering-plan.md)。首期单任务、JSON 活动存储及备份回退安装已被后续实现替代；旧勾选状态不证明当前源码运行验收通过。

> **面向执行者：** 使用 executing-plans 逐任务推进并更新复选框；仅在用户明确授权后采用子代理执行。先读取本计划与设计文档，再修改实现。

**目标：** 在现有项目中完成从需求输入、skills 驱动开发、ZIP 交付到版本管理与按钮安装的 Windows 桌面闭环。

**架构：** Vue 工作台通过 Tauri Channel 消费 pi RPC 事件，Rust 管理运行时、稳定目录、配置与安装。pi extension 提供通用命令和交付能力，游戏知识、依赖与必要编译由 skills 规定。成功产物按版本隔离，安装记录关联具体版本和目标目录。

**技术栈：** Vue 3、TypeScript、Vite 8、Tauri 2、pnpm、shadcn-vue、Tailwind CSS 4、Pinia、Vue Router、markstream-vue、Node + pi RPC。

**规格：** [设计文档](./2026-10-09-gen-mod-design.md)。本计划落实该设计，不另行授权实现、依赖安装、测试、提交或发布。

**2026-10-09 执行状态：** 用户已授权实施。任务 1–9 的源码实现及任务 10 的 Windows 分发已完成；前端/extension 类型检查、原生库检查、前端正式构建和 Windows MSI/NSIS 构建通过。打包后核对 17,387 个资源文件，大小与 SHA-256 均匹配当前清单。已勾项不代表真实运行验收通过。创建/重开夹具、视觉参考、Duckov 编译、测试与检查点 A–E 仍未执行，详见 [实施进度](verification/implementation-progress.md) 和 [Windows 验收](verification/windows-release.md)。Key 仍由用户在 E 手动补充。

## 1. 全局约束与基线

- 项目根目录：`D:\codetest\gen-mod`。所有下文文件路径相对该目录。
- 沿用当前 Vue/Vite/Tauri 配置及 pnpm 锁文件，不迁移 React 或 Bun，不重建项目。
- 首期单活动生成任务；运行中追加需求保存为草稿，空闲后发送。
- provider：`stepfun-plan`；Base URL：`https://api.stepfun.com/step_plan/v1`；模型：`step-5-preview`；API：`openai-responses`。
- Key 通过 `.env` 的 `STEPFUN_API_KEY` 配置，Rust 读取并传给 pi；不得进入前端、交付包或日志。
- API Key 由用户在最终验收时手动补充；实现阶段不要求 Key，任务 1–9 和 Windows 打包准备不因缺 Key 停止。真实 Responses 往返仅列入最终检查点 E。
- 创建/开发 MOD 不要求安装游戏；缺编译依赖时保留源码。游戏内测试与效果验证由用户负责。
- 桌面端只有通用命令、文件交付和安装能力；不得添加 `gameId === 'duckov'` 等游戏执行分支。
- 默认通过通用命令打包 ZIP；需要编译的 MOD 由 skill 规定命令并在任务中执行，不设置手动构建流程。
- 成功版本使用 `artifacts/<versionId>/`，既有版本不覆盖；安装必须由用户点击并选择版本与目标目录。
- 不包含远程控制、并行 Agent、游戏扫描、创意工坊、远程 skill 市场或自动更新。
- 本计划编写时读取到的 `package.json` 只有 `dev/build/preview/tauri` 脚本；`build` 实际执行 `vue-tsc --noEmit && vite build`。不存在测试脚本，不能将未配置命令写成现成能力。
- 当前工作区没有 `.git`；不创建 worktree，不初始化 Git，也不将提交作为任务结束条件。用户授权 Git 操作后才执行。
- CodeGraph 已启用，源码探索先执行 `codegraph explore`，每轮源文件变更后依次执行 `codegraph sync`、`codegraph status`。

**执行与检查边界：** 本次只新增计划。开始实现前确认用户授权覆盖实施及新增依赖。默认只做相关文件的读取和差异检查；下文自动化测试、类型检查、应用启动、真实模型调用和安装包验收是明确的候选检查，获得相应授权后执行，未执行则记录为未验证。不能用静态检查代替运行验收。

## 2. 模块与文件职责

先建立这些边界，文件在对应任务中创建，不提前生成空文件。

| 文件或目录                                                                       | 职责                                   | 首次任务 |
| -------------------------------------------------------------------------------- | -------------------------------------- | -------- |
| `src/contracts/mod.ts`、`skill.ts`、`session.ts`                                 | 应用数据契约，不保存密钥               | 1        |
| `src-tauri/src/storage/{models,paths,mods}.rs`                                   | 原生 DTO、应用路径、MOD 元数据读写     | 1        |
| `src/services/native.ts`                                                         | Tauri 调用及错误转换入口               | 1        |
| `src/router/index.ts`、`src/styles.css`、`src/components/ui/`                    | 路由、主题、shadcn 基础组件            | 2        |
| `src/features/workbench/`                                                        | 三栏布局、输入区、文件/日志上下文      | 2        |
| `src/features/mods/`                                                             | 新建/打开 MOD、版本和安装管理          | 2、9     |
| `src/services/pi/{types,client,reducer}.ts`                                      | RPC 契约、请求匹配、事件归并           | 3        |
| `src-tauri/src/pi/{runtime,bridge}.rs`                                           | 运行时路径、JSONL 与进程生命周期       | 3        |
| `src/contracts/task.ts`、`src-tauri/src/pi/tasks.rs`                             | 应用任务上下文、真实命令记录和交付关联 | 3、7     |
| `src-tauri/src/config.rs`、`.env.example`                                        | env 定位、读取与脱敏状态               | 4        |
| `src/features/settings/`                                                         | env 状态与诊断，不编辑 Key             | 4        |
| `src/features/chat/`、`src/stores/session.ts`                                    | 消息、工具、扩展 UI、会话状态          | 5        |
| `src-tauri/src/pi/sessions.rs`                                                   | 会话路径关联、历史读取与恢复           | 5        |
| `src-tauri/src/changes.rs`、`src/features/workbench/{ChangedFiles,FileDiff}.vue` | 源码任务基准、文件变更摘要与基础差异   | 5        |
| `src-tauri/src/skills/{models,registry}.rs`                                      | 清单、导入、依赖解析与快照             | 6        |
| `src/features/skills/`                                                           | 分类、导入、启停与加载状态             | 6        |
| `skills/common/mod-workflow/`                                                    | 通用开发流程与交付规范                 | 7        |
| `runtime/extensions/{mod-tools,commands,delivery}.ts`                            | 通用命令及交付结果，无游戏分支         | 7        |
| `src-tauri/src/artifacts/{models,registry}.rs`                                   | 交付检查、版本登记和导出               | 7        |
| `skills/games/duckov/duckov-mod/`                                                | Duckov 资料、模板与必要编译脚本        | 8        |
| `src-tauri/src/install/{models,installer}.rs`                                    | 安装预览、复制、备份与回退             | 9        |
| `scripts/prepare-runtime.mjs`、`runtime/runtime-lock.json`                       | 精确运行时版本、资源清单与准备         | 3、10    |
| `docs/verification/`                                                             | 各任务检查记录及 Windows 验收记录      | 对应任务 |

Vue 页面使用 `<script setup lang="ts">`。路由页和 `App.vue` 只组合组件；跨页面的活动会话由一个 store 维护，页面卸载不自行结束后台任务。基础组件放 `components/ui`，业务组件留在 feature 内。

## 3. 数据契约与调用边界

任务 1 落实以下 MOD 字段，任务 3 落实任务字段，任务 7 接入命令与交付记录；TypeScript 与 Rust 保持相同字段，原生 DTO 序列化为 camelCase。ID 为应用生成的稳定字符串，时间使用 ISO 8601 UTC。

```typescript
export interface SkillSnapshot {
  name: string
  version: string
  contentHash: string
}

export interface ArtifactVersion {
  versionId: string
  taskId: string
  createdAt: string
  skills: SkillSnapshot[]
  files: { path: string, size: number, sha256: string }[]
  zipSha256: string
}

export interface InstallManifest {
  targetDirectoryHint: string
  entries: { source: string, target: string }[]
}

export interface InstallationRecord {
  installationId: string
  versionId: string
  targetDirectory: string
  attemptedAt: string
  status: 'succeeded' | 'failed'
  rollback: 'notNeeded' | 'succeeded' | 'failed'
  backupDirectory?: string
  error?: string
}

export interface ModProject {
  modId: string
  name: string
  gameId: string
  gameSkillName: string
  draftRequirement: string
  updatedAt: string
  developmentStatus: 'draft' | 'generated' | 'running' | 'failed' | 'ready'
  latestVersionId?: string
  versions: ArtifactVersion[]
  installations: InstallationRecord[]
  installedTargets: { targetDirectory: string, versionId?: string, state: 'known' | 'uncertain' }[]
  sessions: { sessionId: string, sessionFile: string, skills: SkillSnapshot[], readOnly: boolean }[]
}

export interface TaskContext {
  taskId: string
  modId: string
  sessionId: string
  generation: number
  sourceDirectory: string
  stagingId: string
  stagingDirectory: string
  createdAt: string
}

export interface CommandReceipt {
  taskId: string
  commandId: string
  toolCallId: string
  exitCode: number | null
  status: 'succeeded' | 'failed' | 'cancelled'
  logPath: string
}

export interface TaskRecord {
  context: TaskContext
  status: 'preparing' | 'running' | 'finished' | 'failed' | 'cancelled'
  commands: CommandReceipt[]
  registeredVersionId?: string
}
```

`draftRequirement` 保存在 `mod.json`，创建时接收需求全文，`get_mod` 返回该字段；`save_mod_draft(mod_id, requirement)` 保存后续输入。运行期间的追加需求也保存为草稿，不发送到活动任务；仅在发送已接受且输入仍与提交文本一致时清空草稿，失败或缺 Key 保留。保存写入按 MOD 串行，切换项目先完成保存，保存失败提示并保留当前输入。

`files[].path` 与 `InstallManifest.entries[].source` 相对该版本的 `files/`；目标相对用户所选目录。清单相对路径必须校验，重复目标拒绝。文件内容哈希用于核对 ZIP 与版本文件一致，不将模型声称成功当证据。

安装失败保留失败记录；只有成功安装更新 `installedTargets`。回退成功保留之前的关联，无之前安装则仍未安装；回退失败将目标标为 uncertain。界面依据所选版本与该目标的当前关联显示状态，不能从任意历史成功记录推断当前版本已安装。

调用分为四条边界：

1. Vue → 原生命令：MOD 创建/读取与草稿保存、任务启动、运行时连接、skill 管理、文件变更读取、产物登记、安装和导出。
2. Vue → piClient → Rust → pi stdin：只发送锁定版本支持的 RPC 命令，Rust 不转发前端提供的任意可执行文件路径。
3. pi extension → 通用命令：执行游戏 skill 给出的命令，返回退出码、日志与本次交付路径；输出通过 RPC 工具事件进入 UI。
4. 交付结果 → 原生登记：Rust 从当前进程的成功 `prepare_delivery` 工具结束事件发起登记；前端消费登记结果，不从助手正文或任意工具链接发起登记。Rust 重新核对工作区、任务、文件、清单与 ZIP，成功后生成版本 ID。只有登记成功的版本可安装或导出。

**应用任务到交付的责任链：**

1. Vue 调用 `start_mod_task(mod_id, session_id, requirement)`；Rust 检查空闲、会话归属与 Key 状态。缺 Key 只保存草稿，不创建执行任务。允许发送时，Rust 生成 taskId/stagingId，建立独立临时交付目录，在 `logs/<taskId>/task.json` 保存 TaskRecord；RPC 请求 ID 和连接代次只用于通信，不充当 taskId。
2. 每次生成任务启动绑定该 TaskContext 的 pi 进程，恢复所选会话历史而不重放旧 prompt。Rust 通过子进程环境传入 `GEN_MOD_TASK_CONTEXT` JSON，应用 extension 在启动时读取；taskId/stagingId 不来自模型参数。空闲只读历史连接不带执行上下文，不能调用交付工具。任务 3 完成进程绑定，任务 7 接入 extension 读取和工具行为。
3. `run_command` 为每次实际执行生成 commandId，在工具结果中返回 CommandReceipt。Rust 仅接收当前代次、当前任务和实际 toolCallId 匹配的工具结束事件，将记录写入 TaskRecord；模型文本中的退出码不进入该记录。取消/进程退出后不再接收该任务的新交付申请。
4. `prepare_delivery` 返回 `{ taskId, stagingId, commandIds, manifestPath, zipPath }`，路径只能属于绑定的 stagingDirectory。Rust 将 commandIds 与已记录的实际命令结果核对，校验文件和 ZIP；申请携带旧任务 ID、未记录命令 ID 或失败的必要命令时拒绝登记。
5. 登记同一 stagingId 最多形成一个版本；重复结束事件返回已有登记结果。任务结束后保留命令与交付记录，重启不自动继续未完成任务，也不登记未完成临时目录。

## 4. 推进顺序与检查点

严格顺序：`1 → 2 → 3 → 4 → 5 → 6 → 7 → 8 → 9 → 10`。共用契约、`lib.rs` 与配置文件不并行修改。

| 检查点 | 完成任务 | 可交付结果                                                     | 尚不代表                    |
| ------ | -------- | -------------------------------------------------------------- | --------------------------- |
| A      | 1–2      | 无 Key 创建/重开 MOD，工作台布局完整                           | AI 已接入                   |
| B      | 3–5      | RPC 生命周期、env、结构化会话与基础文件差异；离线协议检查      | 真实模型已通过              |
| C      | 6–7      | 可控 skills、通用命令、版本隔离 ZIP 登记                       | Duckov API 与游戏效果已验证 |
| D      | 8–9      | Duckov 开发路径、版本管理、通用按钮安装                        | 游戏加载与效果正确          |
| E      | 10       | Windows 内置运行时验收，以及用户补 Key 后的真实 Responses 往返 | 已对外发布或游戏效果已验证  |

每任务结束记录：改动文件、实际检查命令与结果、未验证项、影响下一任务的阻塞。API Key 不属于实现检查点的阻塞：A–D 通过本地协议夹具、文件与交付夹具检查，真实模型验收统一属于 E，用户届时手动补 Key。其他直接相关实现阻塞先解决或请用户决定，不以“完成”跳过；最终未提供 Key 时只将 E 的真实模型验收标记为未执行。

## 5. 任务清单

### 任务 1：稳定工作区与元数据

**文件：** 创建 `src/contracts/mod.ts`、`skill.ts`、`session.ts`、`src/services/native.ts`、`src-tauri/src/storage/mod.rs`、`models.rs`、`paths.rs`、`mods.rs`；修改 `src-tauri/src/lib.rs`、`Cargo.toml`。路径中的 `storage/mod.rs` 为 Rust 模块入口，`models.rs` 为 DTO。

- [x] 实现第 3 节数据契约；需要 ID/哈希库时列明新增依赖并按授权安装，不修改无关依赖。
- [x] 通过 Tauri 路径 API 建立本地 `mods/<modId>/source`、`artifacts`、`logs`，元数据用临时文件写入后替换，名称修改不改变 modId。
- [x] 实现 `create_mod(name, game_skill_name, requirement)`、`list_mods()`、`get_mod(mod_id)`、`rename_mod(mod_id, name)`、`save_mod_draft(mod_id, requirement)`；工作区由原生根据 ID 定位，不接收前端指定的待写绝对根路径。
- [x] 创建时将 requirement 保存到 `mod.json.draftRequirement`，读取时返回全文；后续草稿按 MOD 串行保存。不做游戏检测、不调用模型；失败返回可见错误，读取损坏 JSON 不伪装成空列表。
- [ ] 对照 `mod.json` 示例检查创建、重开、重命名结果与契约一致。

**交付：** 原生服务可创建稳定 MOD 项目并跨重启读取，尚不登记产物。

**候选自动化：** `src-tauri/src/storage/mods_tests.rs`，在隔离临时目录检查两个同名项目 ID 不同、重命名目录不变、无 Key 创建后重开仍返回原需求、后续草稿保存后重开一致、损坏 JSON 返回错误。授权后运行 `cargo test --manifest-path src-tauri/Cargo.toml storage::`，不写真实应用数据。

### 任务 2：工作台、路由与项目入口

**文件：** 修改 `package.json`、`pnpm-lock.yaml`、`vite.config.ts`、`tsconfig.json`、`tsconfig.node.json`、`src/main.ts`、`src/App.vue`；创建 `components.json`、`src/styles.css`、`src/router/index.ts`、`src/components/ui/`、`src/features/workbench/{WorkbenchLayout,ProjectSidebar,ContextPanel}.vue`、`src/features/mods/{ProjectPage,NewModDialog}.vue`。

- [x] 在获得依赖安装授权后引入 Pinia、Vue Router、Tailwind 4/Vite 插件及官方 shadcn-vue 所需依赖；保持 pnpm 单锁文件。
- [x] 按官方 Vite 安装指南合并 CSS、`@` 别名和 `components.json`，保留 Tauri 1420 端口、HMR 与 Rust 目录忽略配置。
- [x] 用 Sidebar、Resizable、ScrollArea、Sheet 组成导航/主区/右侧上下文，Dialog 创建 MOD，调用任务 1 的服务。无数据页面显示真实空态。
- [x] 路由采用 hash history，定义 `/mods`、`/mods/:modId/chat`、`/skills`、`/settings`；主会话底部输入区与消息滚动容器分开。
- [ ] 按实施时取得的 Codex 桌面参考确认比例、字号与工具过程外观；宽屏可收起侧栏，窄窗口用 Sheet。参考未取得时记录视觉验收未完成，不声称逐像素复刻。
- [x] 删除被新入口替代的 greet 演示 UI；业务未完成区域保留明确空态，不填造假聊天或安装结果。

**交付：** A 检查点，可通过 UI 创建和打开稳定项目。授权启动后在 800×600 与 1280×800 检查可读性、折叠与输入区；这是界面验收，不是模型验收。

### 任务 3：锁定 pi 运行时并建立 RPC 桥

**文件：** 创建 `runtime/runtime-lock.json`、`scripts/prepare-runtime.mjs`、`src-tauri/src/pi/{mod,runtime,bridge,tasks}.rs`、`src/contracts/task.ts`、`src/services/pi/{types,client,reducer}.ts`；修改 `src-tauri/src/lib.rs`、`Cargo.toml`、`package.json`。

- [x] 从 pi 官方发行信息获取支持当前设计的 CLI 精确版本与对应 Node 要求，核对安装版本 `--help`、RPC 类型和 `agent_settled/disposition`。将包名、版本、Node 版本/架构、下载来源与校验值写入 runtime-lock；不把 Orbit 固定源码版本当 pi 包版本。
- [x] `prepare-runtime.mjs` 按锁文件准备完整 Node/pi 资源，首期用 Node 工具链保留包目录及必要 worker/extension 依赖，不引入 Bun 打包依赖。开发与发布用同一资源清单。
- [x] 实现 `connect_pi`、`send_pi_command`、`disconnect_pi`，启动前建立 Channel；Envelope 使用 `{ generation, record }`，进程退出和诊断使用独立类型。
- [x] 实现 `start_mod_task` 与第 3 节 TaskContext/TaskRecord：生成稳定 taskId/stagingId、校验 MOD/会话归属、保存 task.json、绑定子进程环境。每次任务连接恢复同一会话；只读连接与缺 Key 状态不创建执行上下文。普通 prompt 必须经过任务启动入口，不能绕过任务记录直接发送。
- [x] 按 LF 分帧并保留尾部半包；stdout 连续读取，stderr 写诊断。命令写入采用单一顺序写通道；校验 RPC 类型和 ID。
- [x] client 实现唯一 ID、pending Map、超时与断线统一结算；命令超时不取消长任务，不自动重发。处理 started/handled/queued、先于响应的结束事件、代次过滤；handled 只结束提交，不覆盖扩展独立工作状态。
- [x] Windows 将 pi 与后代进程纳入同一 Job Object，关闭/重启回收完整进程树；abort 先走协议，超时才显示重启入口。不依赖全局 Node/pi 或 Git Bash。

**交付：** 无模型 Key 也能启动本地 RPC、读取状态和展示诊断。

**候选自动化：** `src/services/pi/client.test.ts`、`src-tauri/src/pi/bridge_tests.rs` 与 `tasks_tests.rs`。覆盖半包、CRLF、Unicode 分隔字符、乱序响应、handled 无 settled、settled 先于响应、旧代次、断线 pending 拒绝；同会话两次任务必须得到不同 taskId/stagingId；缺 Key 保存草稿但不创建执行上下文。假进程仅用于协议检查，不代替真实 pi 验收。

```typescript
// client.test.ts 的关键场景；createFakeTransport 在同文件实现订阅与 emit。
it('handled 结束提交而不等待 settled', async () => {
  const transport = createFakeTransport()
  const client = createPiClient(transport)
  const submitted = client.prompt('需求')
  transport.emit({ type: 'response', id: transport.lastRequestId, command: 'prompt', success: true, data: { disposition: 'handled' } })
  await submitted
  expect(client.submissionState.value).toBe('finished')
})
```

### 任务 4：env、模型配置与诊断设置

**文件：** 创建 `.env.example`、`src-tauri/src/config.rs`、`src/features/settings/SettingsPage.vue`；修改 `.gitignore`、`src-tauri/src/pi/runtime.rs`、`src/services/native.ts`。

- [x] 写入已确定的三个 env 字段，真实 `.env` 忽略，示例 Key 为空。
- [x] 开发读取项目根 `.env`，从构建期确定的项目根定位而非 MOD cwd；发布读取 `appConfigDir/.env`，设置页可打开目录。
- [x] Rust 显式解析并验证 URL/model/key，生成独立 pi provider 配置；Key 只通过子进程环境传递，provider 引用变量，不把真实 Key 写入配置 JSON。
- [x] 前端只返回 baseUrl、model、keyConfigured、envPath、运行时版本和脱敏错误；日志不打印进程完整环境，不自动改协议或端点。
- [x] 无 Key 调用 `save_mod_draft` 保存需求，禁用模型发送并指向配置入口；不阻塞 UI、RPC、skills、产物与安装实现。修改 env 后仅空闲时重建连接，不中断活动任务。

**交付：** 设置页展示真实配置状态，本任务通过空 Key 和脱敏配置夹具检查，不要求提供真实 Key。文本流、文件工具调用和认证/连接错误的真实 Responses 验收归任务 10。

### 任务 5：结构化消息与会话恢复

**文件：** 创建 `src/features/chat/{ChatPage,MessageTimeline,MessageParts,ToolActivity,ChatComposer,ExtensionDialog}.vue`、`src/stores/session.ts`、`src-tauri/src/pi/sessions.rs`、`src-tauri/src/changes.rs`、`src/features/workbench/{ChangedFiles,FileDiff}.vue`；修改 `src/services/pi/reducer.ts`、`src/contracts/session.ts`、`src-tauri/src/pi/tasks.rs`、`src/services/native.ts`、ContextPanel、路由与侧栏。

- [x] 按锁定 pi 消息类型定义 ID、role、typed parts；text/thinking/tool/error/产物分开，toolCallId 配对；最终消息覆盖校正增量，不重复追加累计正文。
- [x] 接入 markstream-vue，只渲染文本；工具参数/输出折叠，失败可见，未提供的推理不展示；禁用原始 HTML、脚本和任意组件。
- [x] 高频更新按帧归并，终态前冲刷；用户向上阅读停止跟随；长工具输出截断，全文日志可打开。文本链接走应用校验，文件只能定位允许的工作区。
- [x] 发送需求统一调用 `start_mod_task`；输入变更通过任务 1 的草稿接口保存，缺 Key、拒绝发送和进程启动失败保留原需求；运行中追加草稿不进入任务队列。
- [x] Rust 在任务启动、发送 prompt 前保存 source 的文件清单与文本基准到 `logs/<taskId>/changes/before/`；排除通用产物目录及 skill 声明的编译输出目录，不读取工作区外文件。工具写入/命令结束后刷新摘要，任务停止时保存最终差异记录；命令造成的文件变更也计入，不只依赖工具返回的路径。
- [x] 实现 `list_task_changes(mod_id, task_id)` 与 `get_task_file_diff(mod_id, task_id, relative_path)`，返回新增/修改/删除、路径、文本增删行摘要及 before/after；原生校验任务归属和路径，历史任务读取保存的差异，不用当前源码冒充旧任务结果。二进制或大于 1 MiB 的单文件只返回类型、大小和变化摘要，界面明确未展示全文。
- [x] 右侧“文件/变更”展示 ChangedFiles，选中后 FileDiff 用基础文本行差异展示新增/删除/修改，并提供复制和文件定位；删除文件仍能查看基准内容。不引入增强 diff、Mermaid 或任意 HTML 渲染。
- [x] select/confirm/input 映射 Dialog，取消和超时各发一次对应 UI 响应，不把 UI 响应放入普通 pending 命令队列。
- [x] pi 文件是历史权威：应用保存 MOD 与会话关联，读取历史不另存可编辑副本。RPC 可读取的能力使用 RPC；仅列表/只读历史确实未覆盖时使用锁定 SDK 辅助入口，不创建第二执行 Agent。
- [x] 切换项目/会话先确认活动任务已空闲，切换前冲刷旧事件；恢复使用相同 source cwd。重启只恢复历史不重发 prompt，无 Key 可只读查看。

**交付：** B 检查点，正文与工具分层、需求草稿、取消/重启/历史恢复及基础文件差异完整；本阶段离线检查不要求 Key，真实模型验证统一在 E。

**候选自动化：** `src/services/pi/reducer.test.ts`，输入 text_delta/tool start/tool end/message_end 序列，检查不丢片段、不重复最终文本、工具失败不改为成功、旧代次不污染当前会话；`src-tauri/src/changes_tests.rs` 使用临时源码夹具覆盖新增/修改/删除、命令改文件、二进制摘要、越界拒绝和历史任务差异不随当前源码变化。渲染器 Windows WebView 兼容性单独做真实界面验收。

### 任务 6：skills 管理、依赖与加载隔离

**文件：** 创建 `src-tauri/src/skills/{mod,models,registry}.rs`、`src/features/skills/{SkillsPage,SkillDetails}.vue`、`skills/registry.json`；修改运行时参数、`src/contracts/skill.ts`、`src/services/native.ts`。

- [x] registry 保存分类、gameId、version、dependencies、来源和启用状态；SKILL.md 保持 pi 标准 name/description，不靠额外字段实现加载。
- [x] 导入本地目录时校验名称、描述、引用路径、重复名称，复制至应用数据；展示脚本，不执行导入脚本。内置目录只读，用户副本不同名。
- [x] 所选游戏 skill 加载依赖闭包，缺依赖、禁用依赖或循环依赖显示明确错误；只加载该游戏及其通用依赖，不把所有启用游戏 skill 加入会话。
- [x] 运行参数使用 `--no-skills` 加显式 `--skill`；关闭自动 extension 发现并显式加载应用 extension。隔离 pi 配置目录，核对其他自动资源来源，避免全局或项目资源混入已选择上下文。
- [x] 安装/启用/本会话加载分开显示，`get_commands` 只做实际对照。会话记录内容哈希；导入更新/禁用空闲生效，改变上下文后新建会话，旧会话只读。

**交付：** 不依赖游戏安装即可选择 Duckov 能力，启停与依赖状态真实影响 pi 加载。

**候选自动化：** `src-tauri/src/skills/registry_tests.rs`，检查重复 name、引用越界、依赖缺失/环、仅选择闭包、内置不能覆盖；授权后 `cargo test --manifest-path src-tauri/Cargo.toml skills::`。

### 任务 7：通用命令、ZIP 与版本登记

**文件：** 创建 `runtime/extensions/{mod-tools,commands,delivery}.ts`、`skills/common/mod-workflow/SKILL.md`、`references/delivery.md`、`src-tauri/src/artifacts/{mod,models,registry}.rs`；修改 `src-tauri/src/pi/tasks.rs`、bridge、skills registry、运行时与原生命令注册。

- [x] 通用 skill 定义需求澄清、读取资料、源码修改、命令执行、ZIP 与安装清单交付；不包含游戏 API，不要求测试游戏。
- [x] extension 从 `GEN_MOD_TASK_CONTEXT` 读取任务上下文，注册 `run_command` 与 `prepare_delivery`；没有上下文时拒绝执行。命令输入可执行程序、参数数组、工作目录；输出 CommandReceipt，commandId 由实际执行器生成。脚本作为文件参数运行，路径不拼入任意 shell 文本。
- [x] 支持取消信号和 Windows PowerShell 执行，完整日志写 taskId 路径；模型生成的命令结果不能代替实际退出码。进程均在任务 3 管理的生命周期内。
- [x] Rust bridge 记录实际 `run_command` 结束事件，将 commandId/toolCallId/退出码归属当前 TaskRecord；日志路径限制在本任务 logs 目录。交付验证使用此记录，不读取助手正文伪造的命令结果。
- [x] `prepare_delivery` 在绑定的 stagingDirectory 整理本次文件、install.json、ZIP，返回 `{ taskId, stagingId, commandIds, manifestPath, zipPath }`；commandIds 对应本次打包及 skill 要求的必要编译命令，不将 taskId/stagingId 作为模型可自行填写的参数。
- [x] Rust 在当前 `prepare_delivery` 成功结束事件中发起登记，核对 TaskContext、已记录的必要成功命令、文件哈希、安装清单与 ZIP；拒绝旧任务申请、未知命令 ID、缺文件、旧文件伪装、ZIP 越界条目及重复目标。成功分别写 `version.json`、`files/`、`install.json`、`mod.zip` 后更新版本列表，并向 UI 发送带 taskId/versionId 的登记结果；同 stagingId 重复申请返回已有结果，写入失败不登记半成品。
- [x] 登记后版本不可变；导出复制已选版本的 ZIP，不重新从当前 source 打包。并行交付和安装不读取可变临时输出。

**交付：** C 检查点，任意游戏使用同一交付协议，生成两个独立版本且保留第一版。

**候选自动化：** `src-tauri/src/artifacts/registry_tests.rs`：用纯文本文件做无游戏夹具，登记 A/B 两个版本，验证 A 不变；第二任务收到第一任务申请时拒绝；未知/失败 commandId、取消任务、ZIP 文件集不符不登记；同 stagingId 重复事件仅生成一个版本；导出 A 不返回 B。授权后 `cargo test --manifest-path src-tauri/Cargo.toml artifacts::`。

### 任务 8：Duckov skill 与源码开发闭环

**文件：** 创建 `skills/games/duckov/duckov-mod/SKILL.md`、`references/{api-sources,dependencies}.md`、`assets/template/`、`scripts/{prepare,build,package}.ps1`；修改 `skills/registry.json`。

- [x] 核实设计文档引用的 modding 示例，记录来源 URL、修订版本、netstandard2.1、入口和 info.ini 关系；查不到的 API 不编造。
- [x] 以简单信息展示 C# MOD 为首个模板，统一名称/DLL/namespace/ModBehaviour；模板预览图来源和发布说明明确，保留 AI 内容标注。
- [x] `prepare.ps1` 接收独立参考程序集目录并报告缺失项，不扫描或强制安装游戏、不伪造 DLL；依赖来源写在 skill 资料。
- [x] `build.ps1` 调用必要编译工具并返回真实退出码，`package.ps1` 整理 DLL/info.ini/预览与说明，交给任务 7 打 ZIP/登记。脚本只写当前任务输出，不覆盖旧版本。
- [ ] 无依赖演练走到源码已生成并准确报告缺项；有用户提供的有效依赖时才执行编译交付验收。游戏运行不在该验收内。

**交付：** Duckov 能力完全由 skill 和资料脚本承载，Rust/Vue 没有 Duckov 执行分支。缺参考文件阻塞编译验收，不阻塞通用 ZIP/安装开发。

### 任务 9：版本面板与通用按钮安装

**文件：** 创建 `src-tauri/src/install/{mod,models,installer}.rs`、`src/features/mods/{ModManagerPage,VersionList,ArtifactPanel,InstallDialog}.vue`；修改原生命令注册、存储模型和 feature 服务。

- [x] 面板列出全部版本，选定 versionId 后绑定打开目录/安装/导出，不因新版本出现自动切换。列表显示最新产物与目标当前安装版本。
- [x] `preview_install(mod_id, version_id, target_directory)` 返回完整路径映射和覆盖清单；校验真实路径位于所选版本 files 与目标目录，拒绝符号链接/重解析点导致的越界，拒绝重复目标。
- [x] `install_version` 只在用户按钮确认后执行，重新核对预览与目标现状；不执行任意安装命令。按 MOD 与目标串行安装，版本文件只读。
- [x] 先备份将替换的文件并记录原先不存在的目标，失败恢复备份、移除本次新增文件；保存结果与备份路径。回退失败显示异常且目标状态 uncertain。
- [x] 只有全部复制成功更新 installedTargets；A 成功/B 新生成显示 B 待安装，B 失败回退成功仍显示 A，B 成功才显示 B。不表示游戏验证通过。
- [x] 提供打开源码/版本目录与导出 ZIP，目录选择由通用原生能力提供，不增加游戏发现逻辑。

**交付：** D 检查点，完整版本管理与安装闭环。目标目录默认使用隔离临时夹具验收，不安装到用户真实游戏目录。

**候选自动化：** `src-tauri/src/install/installer_tests.rs`，使用复制函数的可控失败入口验证第二文件失败可恢复第一文件、移除新文件、保留 A 关联；另测越界拒绝、回退失败 uncertain。授权后 `cargo test --manifest-path src-tauri/Cargo.toml install::`。

### 任务 10：Windows 分发与最终交付

**文件：** 修改 `scripts/prepare-runtime.mjs`、`runtime/runtime-lock.json`、`src-tauri/tauri.conf.json`、相关 capabilities 文件、`package.json`；创建 `docs/verification/windows-release.md`、更新 `README.md`。

- [x] 打包适配目标 Windows 架构的完整 Node/pi、extension 与内置 skills；按锁文件校验资源，不附带真实 env、Key、用户源码、日志或参考游戏程序集。
- [x] `bundle.resources` 映射与原生 resourceDir 路径一致；开发/发布启动器使用明确路径，包含空格和中文时以参数传递。
- [x] 原生能力和前端权限只覆盖文件定位、目录选择、通信与当前安装流程；核对 opener 对 URL/文件的输入处理，不扩大为任意系统操作页面。
- [x] 授权后运行前端 `pnpm build` 和 `pnpm tauri build`，记录命令、依赖与结果；不签名、不上传、不发布。
- [ ] 授权真实验收后在无全局 Node/pi 环境启动安装包，验证 env 配置目录、无 Key 草稿、重启恢复、版本保留与临时目标安装；含中文/空格目录。不把开发机启动成功当无全局环境通过。
- [ ] 最终验收由用户手动补充 `.env` Key，执行指定 StepFun Responses 的文本流、一次文件工具调用、需求到 ZIP/版本登记及临时目标按钮安装往返；核对 taskId、命令记录、文件变更和版本归属。认证错误用用户确认的无效测试值验收，连接错误用隔离测试配置验收并恢复原配置，不泄露真实 Key、不自动切换端点或协议。
- [x] README 说明 env、skills、默认 ZIP、必要编译依赖、版本目录与按钮安装；明确游戏验证由用户负责。

**交付：** E 检查点，安装包与可追溯记录，以及用户补 Key 后的真实 Responses 验收结果。Key 只影响最终真实模型验收，不影响前面实现与打包；无法进行的真实验收逐项标明未执行。

## 6. 检查命令与授权后的执行方式

以下命令不在编写计划时运行。测试框架安装需依赖授权，自动化执行另需测试授权；若未获授权，只保留验收场景和未验证标记。

```powershell
# 每次理解相关实现前
codegraph explore "当前任务的符号或模块"

# 源文件变更后索引维护
codegraph sync
codegraph status

# 授权后：已配置的单文件前端测试，不默认跑全仓
pnpm exec vitest run src/services/pi/client.test.ts
pnpm exec vitest run src/services/pi/reducer.test.ts

# 授权后：Rust 当前模块测试
cargo test --manifest-path src-tauri/Cargo.toml storage::
cargo test --manifest-path src-tauri/Cargo.toml pi::tasks
cargo test --manifest-path src-tauri/Cargo.toml changes
cargo test --manifest-path src-tauri/Cargo.toml skills::
cargo test --manifest-path src-tauri/Cargo.toml artifacts::
cargo test --manifest-path src-tauri/Cargo.toml install::

# 授权后：本地界面检查与最终 E 集成/打包验收；真实模型仅在 E 补 Key 后调用
pnpm tauri dev
pnpm build
pnpm tauri build
```

采用 Vitest 仅覆盖 RPC 归并与请求生命周期，不为纯布局和直观字段映射机械补测试；Rust 测试覆盖版本与安装事务等有实际价值的逻辑。测试配置归任务 3 的协议验证交付物，新增 `vitest.config.ts`，不改 Vite/Tauri 的开发端口。

## 7. 规格覆盖与完成标准

| 设计要求                              | 任务    | 完成证据                                               |
| ------------------------------------- | ------- | ------------------------------------------------------ |
| Vue/shadcn/Tailwind 与 Codex 布局     | 2、5    | 组件职责、实际窗口验收记录                             |
| 需求草稿持久化                        | 1、4、5 | 无 Key 创建/输入/重开需求不丢失，发送失败保留          |
| 文件变更与基础差异                    | 3、5    | 任务源码基准、新增/修改/删除、历史差异与路径校验       |
| Orbit 参考、RPC 主链路与必要 SDK 辅助 | 3、5    | 锁定协议、进程生命周期、恢复路径                       |
| Responses、env、指定模型              | 4、10   | 4 完成脱敏配置与无 Key 行为；10 用户补 Key 后真实往返  |
| 通用 + 游戏 skills、导入与启停        | 6、7、8 | 加载清单、依赖闭包、上下文变化新会话                   |
| 不要求安装游戏、用户自行游戏验证      | 1、8    | 无游戏创建、缺依赖保存源码；没有自动游戏验收           |
| 通用命令与 ZIP 交付                   | 7、8    | 真实退出码、ZIP/文件一致、必要编译由 skill 执行        |
| 应用任务与交付登记关联                | 3、7    | 任务上下文注入、实际命令记录、旧任务拒绝、重复登记幂等 |
| 固定目录、版本隔离                    | 1、7    | A/B 版本不覆盖，重开与导出固定版本                     |
| 按钮安装与版本状态                    | 9       | 覆盖预览、成功关联、失败回退、异常可见                 |
| Windows 内置运行时                    | 3、10   | 无全局依赖验收与资源完整记录                           |

全部复选框仅在相应实现完成后勾选，检查记录不能用“计划会验证”替代。API Key 缺失不阻塞实现，真实模型检查只在最终 E 等待用户补 Key；参考依赖或 Windows 验收环境缺失时，相关验收保留未完成。可以报告实现已完成，不能报告整体已验收。

本次计划基于当前设计、项目配置及 CodeGraph 探索结果编写，并核对官方 [RPC](https://pi.dev/docs/latest/rpc)、[模型配置](https://pi.dev/docs/latest/models)、[extensions](https://pi.dev/docs/latest/extensions)、[shadcn-vue Vite](https://www.shadcn-vue.com/docs/installation/vite)、[Tailwind Vite](https://tailwindcss.com/docs/installation/using-vite) 与 [Tauri Channel](https://v2.tauri.app/develop/calling-frontend/) 资料。版本锁定和运行检查属于上述执行任务，本次未安装依赖、未修改源码、未运行测试或应用。
