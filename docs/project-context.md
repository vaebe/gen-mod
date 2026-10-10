# 当前项目上下文

更新日期：2026-10-10。此文保存已确认的需求与实现状态，供后续开发接续。与早期设计或实施记录冲突时，以本文件中的后续决策为准；具体行为仍须核对当前代码。本次同步覆盖今天的提交及当前未提交的实时思考展示改动，不代表运行验收通过。

## 目标与技术方案

- 项目目录：`D:\codetest\gen-mod`。Windows 桌面工具，通过 AI 生成和持续修改游戏 MOD。
- Vue 3 + TypeScript + Vite 8 + Tauri 2；shadcn-vue + Tailwind CSS 4；Pinia、Vue Router、markstream-vue。图标使用现有 `@lucide/vue`，不要引入另一个图标包。
- 当前使用 pnpm 11.17.0；参考 Orbit 的 pi 使用方式，不因 Orbit 使用 Bun 而切换包管理器。
- 内置 Node 22.19.0 Windows x64 与 `@earendil-works/pi-coding-agent` 1.1.0。Agent 执行使用 RPC，SDK helper 仅用于只读历史投影。
- 模型使用 Step Plan 的 OpenAI Responses API；默认 `https://api.stepfun.com/step_plan/v1`、`step-5-preview`。API Key 从 `.env` 设置，由用户配置，不作为无凭据阶段的实现卡点。
- `step-5-preview` 配置文本和图片输入，思考强度 low / medium / high，默认 medium；上下文上限 1,000,000，输出上限 64,000。不要将这些参数套用到其他模型。
- 游戏能力由通用 skills + 对应游戏 skills 扩展。优先逃离鸭科夫；不要在 Vue/Rust 中为每个游戏编写执行分支。
- 开发项目不强制安装游戏。游戏内测试和效果验收由用户负责；构建需求、验证步骤、安装清单由 skills 规范。
- MOD 源码必须持续复用，不能每轮重新初始化或清除。交付版本隔离，提供简单管理面板与用户点击安装入口。版本数量不限制，由用户手动删除，不自动清理旧版本。

## 已确认的交互

### 左侧项目与会话

- 会话统一在左侧管理，不恢复聊天顶部的会话选择器或新建按钮。
- 项目右侧的新增会话图标采用圆角编辑形状；只在项目标题行 hover / 键盘 focus 时显示，不因 hover 会话列表显示。
- 项目标题点击展开或收起会话列表，展开用 `FolderOpen`，收起用 `Folder`；只改变展示，不切换或停止会话。新增会话时自动展开。
- 分组展开状态目前保留在组件内存中，未跨应用重启保存。
- 会话不带图标、不带左侧竖线；背景占满分组宽度，左内边距 12px、右内边距 6px。
- 新建会话统一显示“新会话”，不显示编号。首轮成功结束后，取首条需求折叠空白后的前 40 个字符作为标题；不是额外调用模型生成标题。手动重命名不覆盖。
- 支持运行时切换、新建及重命名；只有目标会话运行、准备或压缩时禁止删除。各会话独立显示圆环状态，后台事件继续归并及持久保存。
- 已移除侧栏底部“本地项目 · 版本交付”。可点击按钮显示 pointer，禁用按钮不显示可点击样式。

### 输入框与功能菜单

- 按用户提供的 Codex 截图组织布局：“+”在左侧；右侧依次为上下文圆环、模型、思考强度、发送 / 停止按钮。不新增截图中的麦克风、MCP 等功能。
- “+”与输入 `/` 使用同一功能列表：选择文件、压缩上下文。菜单铺在输入框上方，与卡片同宽；图标、名称、灰色说明在一行。
- 斜杠菜单可筛选，可用上下键、Enter、Escape；执行操作时移除末尾斜杠指令，不发送给模型。发送快捷键为 Ctrl + Enter。
- 空闲时显示向上箭头发送；生成时支持文本及图片补充引导，并显示独立停止按钮。准备、压缩和停止中不能引导；补充要求沿用原任务、进程与计时。
- 思考强度采用自定义菜单，向上打开，最小宽度 176px、圆角和宽松选项留白，勾选当前值；不使用浏览器原生窄下拉框。
- 上下文圆环只保留悬浮用量提示，不再点击打开压缩弹窗；显示使用比例、剩余比例、已用 token 与窗口上限。
- 自动压缩沿用 pi。手动压缩只在已有消息且目标会话空闲时允许，必要时连接运行时；忙碌时禁用菜单项。
- 保留“描述你希望实现的 MOD…”占位文字。文字区与底部工具栏留白统一；不显示保存中闪烁提示或缺 Key 的常驻说明。
- 输入文字区最小高度从 96px 减至 56px，最大高度从 240px 减至 200px，卡片内边距和底部按钮尺寸不变。

### 回复与执行过程

- 当前未提交实现只在运行中的展开明细展示当前实时思考段；思考结束、工具开始或任务结束后清除展示状态，恢复历史不展示思考文本。原始思考数据仍可能包含在 pi 历史中。
- 生成中显示用时、当前活动和最新一条公开过程文字。文字区保留轻微掠光；遵循 reduced motion。
- 折叠状态不再单独展示开头带 loading 的工具行。工具、命令与历史过程正文留在展开明细中。
- 展开区域末尾不增加第二条分隔线。Markdown 字号和段落间距已缩小。
- 执行明细使用独立滚动区域，用户上滚后停止跟随，可点击“回到最新”；实时思考区也可独立滚动。最终回复提供复制按钮及成功或失败反馈。
- 手动停止用灰色“已手动停止”，不把 aborted 消息显示为红色流错误。真实错误不统一伪装成停止。
- 断连显示灰色状态栏和圆角刷新图标按钮；其他错误保留错误说明。
- 右侧日志面板读取持久运行日志，显示最近 200 条，面板可见时每 3 秒刷新；完整工具结果和命令日志另存任务目录。

### 附件

- 支持选择图片、UTF-8 文本 / 源码文件，也支持粘贴图片；发送前可预览、移除。
- 图片：PNG、JPEG、WebP、GIF，单张上限 10 MB；通过 RPC `images` 发送 Base64 内容，再由 pi 转换成 Responses 图片输入。
- 文本：单文件上限 1 MB；拒绝无法按 UTF-8 解码或含 NUL 的二进制文件，正文随提示词发送。
- 最多 6 个附件，合计上限 20 MB；选择任一无效文件时整批不添加，已选附件保留。
- 发送失败保留附件；成功后移除已发送附件。切换会话保留各自内存附件，并丢弃旧的异步读取结果。
- 未发送附件只在会话运行注册表的内存中，重启需要重选；已发送图片和文本随 pi 会话 JSONL 保存。恢复用户消息时显示图片。
- 已发送图片独立显示在用户文字气泡上方，靠右排列为 80px 方形圆角缩略图，多张可换行；不再在文字气泡内显示大图。此展示不改变发送或历史存储内容。
- 缩略图可点击打开大图弹窗，按 Esc 或点击关闭按钮退出；实时消息及恢复历史共用 `ChatImage.vue`。

### 新建项目与上下文面板

- 新建 MOD 表单包含项目名称、游戏能力及选填需求；能力列表只显示启用且无错误的游戏能力，下拉与触发器同宽，显示名称与说明。创建失败保留输入，保存成功后进入工作台。
- 右侧上下文默认收起，仅在当前路由会话初始化完成时显示开关；宽度至少 1100px 时用可调整面板，否则用抽屉。隐藏时保留组件内展开偏好，不跨重启保存。
- “文件”提供“全部文件”与“当前变更”。全部文件按目录加载，不依赖是否有任务；只读预览当前 source，支持行号、语法高亮、复制全文和定位文件。文本上限 1 MiB，支持的位图上限 10 MiB，图片可切换适应面板与原尺寸；不支持的编码、二进制、超限或损坏图片显示原因。
- 当前变更分别展示本任务工具操作和任务开始后的共享目录差异；后者可能包含其他会话操作，不能视为本任务独占改动。

## 数据与会话恢复

应用数据根目录通过 Tauri `app_local_data_dir` 取得，Windows 当前路径为 `%LOCALAPPDATA%\com.ygdpz.gen-mod`。不使用浏览器 localStorage 保存聊天历史，应用元数据采用 SQLite；pi JSONL 仍是正式历史权威。

```text
应用数据目录/
  gen-mod.sqlite3                 应用元数据（WAL，配套 -wal / -shm）
  pi/config/models.json            模型配置，Key 仅引用环境变量
  mods/<modId>/
    mod.json                      旧迁移源，保留但不再活动写入
    source/                       所有轮次共用的源码工作区
    logs/sessions/*.jsonl          pi 会话历史（消息、工具结果等）
    logs/sessions/*.runtime.jsonl  旧摘要迁移源；新摘要在 SQLite
    logs/<taskId>/                 差异、命令全文日志、工具结果、操作记录与 staging
```

- 版本文件与 ZIP 的实际目录由 `src-tauri/src/artifacts/registry.rs` 管理；登记后的内容不可变，用户可以删除应用内的版本包，安装走预览与直接覆盖流程。
- 开发 `.env`：项目根目录；发布 `.env`：Tauri 应用配置目录。API Key 不返回给前端，命令子进程移除 Key，日志做脱敏。`.env` 是文件配置，不宣称凭据加密。
- pi 会话 JSONL 是历史权威，SQLite sessions 保存 sessionId / sessionFile / 标题 / skill 快照关联；不重写正式历史。
- 有历史文件时启动只传 `--session <文件>`；无文件时使用 `--session-id <ID>`。两者互斥，同时传入会使 pi 退出。
- 连接失败仍保留目标 ID；重新连接只重试已选会话，不能用空 ID 隐式新建。
- 原生按 (modId, sessionId) 维护独立进程槽；每轮生成只重建目标槽的 pi 进程，以绑定新 taskId；这不代表新会话。沿用历史文件和稳定 source，不重发旧需求。
- 发送时先在前端显示本轮用户消息和准备状态，随后保存草稿、提交任务；pi 正式用户事件替换本地占位，沿用消息 ID。正式确认前失败时移除占位，输入与附件保留。本地占位不写入会话历史。
- 提交快照与下一份输入分离：点击发送立即清空文本，自动保存等待提交结束；失败只在输入未继续编辑时恢复原文本，成功按原会话和快照条件清理草稿，不覆盖新输入。附件按本次快照清理。
- 原生 start_mod_task 使用后台阻塞线程执行源码快照和进程启动，使用目标会话锁、共享源码快照租约及原有哈希校验，避免同步准备阻塞 IPC 主线程；准备耗时本身并未省略。
- 每轮 createdAt / finishedAt 存入 SQLite tasks，恢复时据此显示用时。
- 会话初始化由 SDK 历史投影与 SQLite 会话偏好并行读取，不启动模型或等待 RPC 命令列表，项目初始加载显示恢复提示。首次挂载历史列表自动贴底，正文尺寸变化时继续跟随；用户向上滚动后停止跟随。
- 上下文当前窗口估算通过 pi 查询，SQLite 保存采样时间、generation 与估算快照；断连或恢复显示为历史值；压缩后暂时未知，后续回复再更新。累计计费 token 与当前窗口占用不同。

## 命令执行与交付

### 版本管理与部署（后续决策）

- 版本管理改为倒序列表，显示短版本号、生成时间、文件数及状态，窄面板合并时间和文件数。点击版本号复制完整 ID；每行菜单提供详情、安装、导出 ZIP、打开版本目录和确认删除。详情显示版本 ID、任务 ID、时间及文件路径和大小；安装历史独立弹窗展示。无关 AI 任务不禁用菜单，实际文件操作使用存储锁协调。
- 右侧产物倒序展示版本、文件数、时间和当前安装标记，点击直接打开同一详情弹窗，另提供版本与安装管理入口。

- 当前仍有 `installedTargets.state === known` 的部署位置引用某版本时，在版本列表显示绿色“已安装”标签；不拿历史成功记录冒充当前安装状态，也不宣称已检测游戏目录中的外部改动。
- 每个项目保存 `lastDeploymentDirectory`，选择目标并通过预览校验后更新，取消实际安装也会保留。旧项目没有此字段时，界面从最近安装记录提供之前的位置。
- 再次安装时弹窗让用户选择“使用之前的位置”或“重新选择”；复用也需重新校验目标和文件，再确认安装。
- 安装弹窗的目录操作与确认按钮右对齐。界面隐藏 Windows 本地盘路径的 `\\?\` 扩展前缀，扩展 UNC 路径显示为普通网络路径；存储、校验和安装仍使用原始路径。
- 新安装直接覆盖清单中的同名文件，不备份、不自动回退。复制失败且可能已改动文件时，将目标状态标为 uncertain；安装记录用 notAttempted 表示没有执行回退。
- 原有备份与历史回退记录不自动删除，新记录不再产生 backupDirectory。
- 取消“只保留 6 个版本”要求：版本数量不限制。选定版本可手动删除，操作前确认；只删除应用内的文件、ZIP 和版本条目，不删除源码、会话、任务日志、部署文件或安装历史。
- 删除最新版本后，最新指向剩余版本中的最后一个；删完后版本列表为空，项目源码仍可继续开发。仍已部署的旧版本即使删除本地包，其部署记录仍保留。

### 执行环境与登记

- `run_command` 用程序 + 参数数组直接启动（`shell:false`），不会把 `pwd` / `ls` 当作 shell 内建命令执行。读取 / 搜索优先使用 pi 文件工具。
- 每个任务进程探测一次已有 shell / dotnet 路径，Windows 优先 `pwsh.exe`，没有时使用 Windows PowerShell。探测只检查路径，不安装程序。
- 通用扩展把实际路径与 `-Command` / `-File` 调用方式写入系统说明；命令子进程 PATH 补充探测到的 shell / SDK 目录。
- Duckov `build.ps1` 内部仍使用 `powershell.exe` 调用 prepare.ps1；2026-10-10 已为 prepare.ps1 添加 UTF-8 BOM，供 Windows PowerShell 读取。未在本次文档同步中执行脚本或真实编译。
- Duckov skill 已补充 API 查证、游戏功能、UI/资源、Harmony 和排查专题；API 入口须核对实际参考程序集，不能把示例当作所有游戏版本的契约。当前 package.ps1 只收集 MOD DLL、info.ini、preview.png、README.md，额外资源和第三方 DLL 需另核对交付能力。
- skill 快照按内容哈希校验；修改已使用的 skill 后，旧会话可能成为只读。不要为了修一处脚本而擅自放宽哈希校验。
- `prepare_delivery` 先生成 staging ZIP，通过内部 `extension_ui_request` 等待原生登记回执；原生成功后才向 AI 返回 versionId。这个请求由桥接拦截，不弹出用户输入框。
- 原生校验任务归属、实际工具参数、必要脚本成功记录、文件集、哈希与 ZIP。包装脚本成功不能自动代替原始 requiredCommandScripts 的记录。
- 登记失败必须告诉 AI 失败原因，不能仅因 ZIP 存在宣称交付成功。
- 命令结束后最多等待新增后代 10 秒，再仅回收本次命令新增进程；确认退出后释放租约并保留 pi。继续等待 5 秒仍失败才停止整个运行时，无法确认退出则保留租约。全文日志记录等待、回收、命令输出及关联 ID；命令回执附最近 16,384 个字符输出。共享工作区排队超过 25 秒会返回占用信息。
- 已知限制：prepareDelivery 的 staging/files 使用独占目录创建；同一任务登记失败后再次调用可能不能直接复用该目录。尚未实现同轮重试方案。

## 当前实际项目记录

仅用于定位用户已提及的“小地图”项目，不复制会话正文或附件。

- modId：`4f821e98-8b57-43ef-8826-fb80c5575683`。
- sessionId：`fc603b97-e131-469b-9d44-3ca313a4927a`。
- 曾出现恢复启动参数冲突，以及空 ID 重连新建空会话；源码已修改，不再同时传两种参数，并保留重连目标。
- 早期某轮已有 DLL / ZIP，但原始构建脚本记录不符合要求，未登记成功。不要据此推断所有后续轮次仍失败。
- 2026-10-09 本次只读核对 mod.json：已有 1 个登记版本，latestVersionId 为 `c14be674-7fff-4a71-b493-b989bcd57e7b`。这是登记元数据证据，不代表已安装或游戏内验收。后续使用前重新读取当前数据。

## 关键代码入口

| 内容 | 文件 |
| --- | --- |
| 项目分组、会话管理 | `src/features/workbench/ProjectSidebar.vue` |
| 输入框、附件、功能菜单 | `src/features/chat/ChatComposer.vue`、`attachments.ts`、`ChatPage.vue` |
| 时间、活动、过程渲染 | `RoundActivity.vue`、`MessageParts.vue`、`conversationRounds.ts` |
| 上下文用量 | `ContextUsageIndicator.vue`、`src/stores/session.ts` |
| RPC 请求匹配与历史归并 | `src/services/pi/client.ts`、`reducer.ts` |
| 每会话运行状态、提交与引导 | `src/services/pi/sessionRuntime.ts`、`src/stores/session.ts`、`src/features/chat/useDraft.ts` |
| 图片放大预览 | `src/features/chat/ChatImage.vue` |
| 工作台布局与源码浏览 | `src/features/workbench/WorkbenchLayout.vue`、`ContextPanel.vue`、`SourceBrowser.vue`、`useSourceBrowser.ts`、`SourceFilePreview.vue`、`src-tauri/src/files.rs` |
| 版本列表与详情 | `src/features/mods/ModManagerPage.vue`、`VersionList.vue`、`ArtifactPanel.vue` |
| 原生启动、任务与交付回执 | `src-tauri/src/pi/bridge.rs`、`runtime.rs` |
| 历史与运行日志读取 | `src-tauri/src/pi/sessions.rs`、`logs.rs` |
| 项目存储与模型配置 | `src-tauri/src/storage/`、`src-tauri/src/config.rs` |
| 命令、持续工作区说明、交付工具 | `runtime/extensions/commands.ts`、`mod-tools.ts`、`delivery.ts` |

表中省略目录的 Vue 文件均在 `src/features/chat/`。

## 开发与验证边界

- 本地运行：`pnpm dev:app`。用户自行启动；除明确要求，不代为运行测试或启动应用。
- 每次修改前重新检查工作区差异，保护已有源码、文档和未跟踪文件，不擅自暂存 / 提交 / 清理；不沿用旧记录推断当前工作区状态。
- 源码探索先 CodeGraph，再核对当前源码；源文件改动后执行 `codegraph sync`、`codegraph status`。
- 按本次范围使用限定检查：
  - `node node_modules/vue-tsc/bin/vue-tsc.js --noEmit -p work/tsconfig.multi-session.json`
  - `node node_modules/typescript/bin/tsc --noEmit -p work/tsconfig.runtime.json`
  - 原生相关改动：`cargo check --manifest-path src-tauri/Cargo.toml --lib --locked --offline`
- 修改 runtime/extensions 后运行 `node scripts/prepare-runtime.mjs`，同步便携扩展与资源清单；旧安装包仍需要重新构建才能包含新改动。
- 此前附件 / 菜单 / 命令环境改动的上述类型与编译检查通过；这不证明实际图片请求、菜单焦点、真实窗口或游戏行为。
- 当前没有由助手执行真实模型与游戏内验收。本次 UI 改动的检查结果记录在阶段实施记录，不将计划中的检查写成已通过。


## 多会话、共享源码与 SQLite（2026-10-09 实施）

2026-10-10 审核修复：工具结束、任务收尾和进程停止的差异刷新异步等待源码租约，取得后核对会话代次；不得持读租约反向等待会话锁。发送成功按原会话清理附件及未编辑的草稿，SQLite 条件更新保护较新的输入；任务启动只保存思考强度，不把含附件的 prompt 覆盖回草稿。进程退出收敛未完成工具终态，缓存会话同步最新只读状态，按需连接先应用持久思考强度再查询状态。限定 Vue 类型检查与两个 Rust 文件语法解析通过；未运行测试、应用或 Rust 编译。

- 基于本地 77d5bd2 和执行前已有工作区核对，按计划任务 1–7 实施；保留已有 docs/README.md、output、work 内容。根目录未发现实体 AGENTS.md，本轮遵循用户提供的规范。
- 前端 registry 以 modId/sessionId 选择展示对象，runtime 独立处理消息、附件、统计、工具请求及后台事件；读取历史使用 SDK helper，发送或压缩才启动 pi。切换路由不结束任务。
- 同项目只有一份 source，不创建分支，也不对整轮 AI 调用串行排队。原生协调 read/grep/find/ls 的共享读与 write/edit/run_command/prepare_delivery 的 FIFO 独占租约；排队只等待工具操作。排队超时显式失败并取消，AI 可重试。
- 覆盖已有文件前必须先 read，原生按任务及文件记录哈希；实际写入前发现基线变化则拒绝，要求重新读取。命令排队期间源码 revision 变化也拒绝。命令租约覆盖后代进程生命周期，停止先回收目标 Windows Job，确认退出后释放源码租约。
- 每任务 workspace-operations.jsonl 记录工具归属、revision 和前后文件哈希；共享源码前后差异与本任务操作记录分别展示。构建回执与交付登记绑定原生 revision，交付租约覆盖打包及登记，并保留 receipt、必需脚本、fresh output、文件集和 ZIP 哈希校验。
- steering 先在 SQLite 记录 inputId/taskId/generation，再通过目标进程投递；以 queued/handled/rejected/unknown 和正式 JSONL entryId 区分状态。正式消息替换本地占位，同任务补充归入原轮次；退出和重启不自动重发未确认项，无法关联到历史的项显示状态提示。
- SQLite 数据库位于 app_local_data_dir/gen-mod.sqlite3，使用 rusqlite 0.40.2 bundled、WAL、外键、5 秒 busy timeout 和短事务。元数据包含项目、会话草稿与偏好、任务、运行摘要、引导、统计、迁移和文件操作恢复记录；skills/config、源码、pi JSONL、日志全文、交付目录及安装历史原文件保留。
- 启动时按项目事务导入旧 mod.json/task.json/runtime 摘要，校验身份、路径、版本及安装关联；坏项目隔离并显示迁移错误，已成功项目不重复导入。迁移失败不向该项目发送任务。旧项目草稿分配给最后可用会话；新项目首会话在事务内消费 initialDraft。
- 元数据更新使用短事务重读及 revision，避免后台事件覆盖同时新增的版本/安装信息。文件与数据库跨边界操作先记恢复记录；启动修复 tombstone/未完成安装状态并报告孤立文件，不删除历史备份、安装记录或孤立交付。
- 重启只把旧活动任务标为 interrupted、未确认引导标为 unknown；保持会话 ID 与 JSONL 路径，不恢复旧进程、不重放需求。迁移仅在用户启动新应用时发生，本轮没有运行数据库迁移或访问现有应用数据。
- 最新静态检查及用户验收见实施记录和实施计划末尾。尚未运行多进程、SQLite 迁移、真实模型/图片、UI 或游戏验收；类型/编译通过不能替代这些 Gate。


### 2026-10-10 启动修复记录

用户启动发现 missing field sessions：SQLite projects.payload 按设计不存 sessions，读取时必须先与 sessions 表组合，再解析完整 ModProject。repository::read_on 已调整组合顺序；不删除数据库、不重新迁移、不放宽旧文件校验。Rust lib 编译通过，助手未启动应用；启动结果待用户重新运行确认。
