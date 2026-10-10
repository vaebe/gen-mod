# 实施进度

本文件保留阶段实施记录。后续需求与实现状态请先查阅 [当前项目上下文](../project-context.md)，旧安装包和早期描述不代表最新源码状态。

2026-10-09。用户已授权按执行计划实施及引入计划内依赖。任务 1–9 的实现已写入，任务 10 已生成 Windows x64 MSI/NSIS 安装包。前端/extension 类型检查、原生库检查、前端正式构建和 Windows 打包通过；打包后资源一致性检查通过。测试、应用启动与真实模型验收未执行。

## 任务 1

已写入原生存储接口、TypeScript/Rust 元数据契约、UUID 目录定位、草稿串行保存和临时文件原子替换。Cargo.lock 已生成，原生库检查通过。未在真实应用数据目录创建项目或验收重开、重命名行为。

游戏标识从注册表 gameId 读取；get_mod 会校正实施早期临时以能力名称保存的 gameId。游戏选项来自注册表，无 Rust/Vue 游戏执行分支。

## 任务 2

已生成官方 shadcn-vue 基础组件，接入 Tailwind 4、Pinia、Vue Router 和工作台页面。路由使用 hash history；新建与读取调用真实原生接口，未编造对话和交付结果。已固定 pnpm 11.17.0；shadcn 当前组件采用 @lucide/vue。

源码已经写入，800×600/1280×800 窗口、窄屏与 Codex 当前视觉参考验收未执行。没有通过声明替代运行证据。

## 任务 3–5

- 锁定 @earendil-works/pi-coding-agent 1.1.0、Node 22.19.0 Windows x64，RPC 类型以安装包声明和实现核对。明确运行时路径、JSONL、代次过滤、请求超时、Job Object、取消后重启入口和每任务上下文已写入。
- env 开发/发布路径、指定 StepFun openai-responses provider 与前端脱敏状态已接入。models.json 仅引用 `$STEPFUN_API_KEY`，命令子进程移除该 Key，日志脱敏；无 Key 不创建生成任务，草稿保留。
- 正文、推理、工具状态、扩展 Dialog 与错误分层；按帧归并消息，终态冲刷、滚动跟随；原始 HTML、图像、数学和 Mermaid 不参与渲染。文本链接走原生校验。
- pi 历史为权威，普通会话使用 RPC 恢复，旧上下文会话用 SDK 只读投影；恢复不重发需求。源码基准、命令后变更及历史差异保存到任务目录，二进制/大文件仅摘要。完整工具结果与命令日志可打开。

以上通过静态类型/库检查，RPC 启动、真实窗口、取消与历史恢复未做运行验收。

## 任务 6–8

- 标准 YAML frontmatter、应用 registry、能力分类/来源/启用状态、依赖闭包、引用越界检查、本地资源导入与内容哈希已写入。能力错误单独展示；get_commands 对照实际加载的 skill 命令，不用启用状态冒充加载状态。活动任务期间禁止修改能力。
- 通用 run_command 与 prepare_delivery 以原生任务上下文绑定。命令记录使用实际 toolCallId、commandId 与退出码；prepare_delivery 使用 PowerShell 的 .NET ZIP API 打包。Rust 依据成功结束事件核对命令、文件集、SHA-256、安装清单及 ZIP，登记独立版本；前端和助手正文没有登记入口。
- 需要编译的 skill 通过 requiredCommandScripts 声明必要脚本，交付需当前任务 `.delivery/<taskId>/` 新目录中的文件。版本导出固定选定 ZIP，不重新打包 source。
- Duckov 模板、来源、依赖说明与 prepare/build/package 脚本已加入 skill；资料固定 xvrsl/duckov_modding 修订 32721af32edb387757ff536a6e3925f7856c934e。使用 netstandard2.1，名称/DLL/namespace/入口一致，预览图由脚本自行绘制并标注 AI 内容。参考程序集采用官方示例的 `TeamSoda.*.dll` 引用组及两个 Unity 模块，不猜测具体游戏程序集名；入口是否可解析仍须真实编译确认。

未运行 skill 压力测试、缺依赖演练、C# 编译、通用 ZIP 登记 A/B 版本验收；没有游戏参考程序集或真实模型结果。

## 任务 9–10

- 版本管理页、选定版本操作、安装完整映射/覆盖预览、原生预览令牌与重新核对、备份/复制/回退、异常状态、历史记录和导出已写入。安装元数据持久化失败也尝试回退并保存失败记录。
- 原生目录选择与 opener 的输入校验已接入；前端移除直接 opener 默认权限。没有添加通用 shell 页面权限或游戏扫描。
- 运行时准备实体化 122 个依赖目录，资源清单包含 17,374 个运行时文件和 13 个 skills 文件。只映射固定 Node、pi-portable、helper/extension、skills 与清单；开发机 pnpm 链接、旧部署失败目录、Node 下载 ZIP、真实 env 与用户数据不进入 bundle.resources。
- README 和 Windows 验收场景已更新。Windows MSI/NSIS 构建通过，产物位置及 SHA-256 见 windows-release.md；未安装、签名、上传或发布。

## 实际检查记录

- `codegraph explore "应用入口与Tauri命令注册"`：返回现有 Vite 入口符号；另读取当前原生入口与配置。
- 多轮 `codegraph sync` / `codegraph status`：同步成功，返回 `[OK] Index is up to date`；结束前再同步最终源文件。
- pnpm 安装计划内页面依赖：完成；首次沙箱无法打开全局缓存数据库，放行后成功。
- `pnpm dlx shadcn-vue@latest add ... -y`：首次 Corepack 版本导致 store 冲突；固定项目 packageManager 后成功，生成 86 个基础组件文件。
- `node scripts/prepare-runtime.mjs`：成功准备上述分发资源；固定 Node ZIP SHA-256 校验成功。
- `node work/check-implementation-syntax.mjs`：127 个本轮新增/修改前端文件，0 语法错误；该脚本生成限定范围临时 TS 配置。
- `node node_modules/vue-tsc/bin/vue-tsc.js --noEmit -p work/tsconfig.implementation.json`：通过。首次沙箱未解析 pnpm 链接，正常权限下定位并修正实际重复字段错误后通过。
- `node node_modules/typescript/bin/tsc --noEmit -p work/tsconfig.runtime.json`：三个 extension 文件类型检查通过。
- `rustfmt --edition 2021 --emit stdout <本轮原生源文件>`：解析通过，输出到 work，不改源码；不能作为类型检查。
- `cargo check --manifest-path src-tauri/Cargo.toml --lib`：通过。最初 Cargo 未使用系统代理，下载超时；仅本次进程采用现有系统代理后成功，未写全局配置。
- `cargo check --manifest-path src-tauri/Cargo.toml --lib --locked --offline`：最终原生修改复核通过。
- `node node_modules/pnpm/bin/pnpm.cjs build`（项目固定 pnpm 的 build 脚本）：通过，执行 vue-tsc 与 Vite 生产构建。ChatPage 分块约 693 kB，有大于 500 kB 的提示，未因此扩大范围重构。
- `node node_modules/@tauri-apps/cli/tauri.js build`（Tauri build）：首次原生 release 编译通过；因 Duckov skill 在该次构建后更新，主动停止旧资源的 WiX 打包，首次命令退出 1，未交付旧包。随后从最新资源重新构建成功，退出 0，生成 MSI/NSIS；进程使用已有系统代理，不修改全局设置。
- `node work/check-release-resources.mjs`：重新构建期间及打包结束后均通过；release 资源清单与准备清单一致，17,387 个运行时及 skills 文件的大小与 SHA-256 全部一致。这是 release 复制资源检查，未解包或安装生成的安装程序。
- 未运行测试、Lint 或应用；未调用真实模型、编译 Duckov DLL、操作真实游戏目录、初始化 Git、提交或发布。

## 尚未验收

检查点 A–E 的真实运行场景仍未执行。Key 仅影响 E 的模型往返；前面实现与资源准备没有等待 Key。用户提供有效参考程序集后可验收 Duckov 编译；游戏效果由用户自行验证。静态检查通过不能声称上述闭环已运行通过。

## 后续会话交互调整（2026-10-09）

- 项目右侧新增会话图标，左侧项目下管理会话；首次需求生成标题，支持重命名和确认删除。删除只移除会话及历史文件，保留项目源码、交付版本和任务日志。旧元数据缺少标题或结束时间时保持兼容。
- 输入框改为发送／停止图标，接入实际 pi 思考档位；Step 5 显式声明低／中／高三档、1M 上下文和 64k 最大输出，依据 [StepFun 官方模型资料](https://platform.stepfun.ai/docs/en/guides/models/step-5-preview)。选择会传递至每次启动的执行进程。
- 每轮中间说明、思考与工具记录收入可展开的“用时”区域，终态正文保留在外。运行时显示活动摘要，长时间没有新事件时显示等待返回；失败保留简短提示与可展开明细。结束时间保存到任务记录，旧记录没有结束时间时不猜测时长。
- 使用 pi 原生 `get_session_stats.contextUsage` 显示当前窗口占用、剩余与百分比，标注估算；不使用会话累计 token 冒充窗口占用。压缩后用量待更新，旧连接及压缩前统计不得覆盖新状态。
- 自动压缩在 pi 内启用；手动压缩只允许空闲会话，前端与原生均检查生成／压缩状态。压缩响应超时后查询权威状态，不假定已结束；不引入插件或新依赖。
- 本轮限定范围命令：`node node_modules/vue-tsc/bin/vue-tsc.js --noEmit -p work/tsconfig.chat-ui.json`、`cargo check --manifest-path src-tauri/Cargo.toml --lib --locked --offline` 均通过。首次沙箱类型检查未正确解析 pnpm 依赖，正常读取依赖后通过。
- 本轮未运行测试、未启动应用、未调用模型、未实际删除会话或执行压缩，也未重新构建安装包。此前 MSI/NSIS 不包含本轮改动；本地使用 `pnpm dev:app` 验收当前源码。

## 2026-10-09：Codex 输入框布局、项目折叠与接续文档

- 输入框功能菜单改为卡片同宽并在上方展示，图标、名称和说明同一行；“+”放左侧，上下文、模型、思考强度和发送/停止放右侧。仍只提供选择文件、压缩，不新增截图中的其他能力。
- 项目标题切换会话列表展开/收起，FolderOpen / Folder 区分状态；新增会话时展开。展开状态只在组件内存中。
- 新增 docs/README.md 和 docs/project-context.md，保存当前决策、数据存储、恢复流程、附件、交付回执、已知限制和关键代码入口。旧阶段记录保留，标注最新文档入口。
- 本次只读核对用户“小地图”mod.json：已有 1 个登记版本。这不证明安装或游戏内效果。
- 限定检查 `node node_modules/vue-tsc/bin/vue-tsc.js --noEmit -p work/tsconfig.chat-ui.json` 通过。未运行测试、启动应用或检查真实窗口。CodeGraph 维护结果以结束前实际输出为准。

## 2026-10-09：安装与手动版本删除

- 用户撤回“最多保留 6 个版本”：不限制数量，不自动清理；在选定版本面板提供手动删除及确认弹窗。
- 版本列表根据当前 known 部署记录显示绿色“已安装”标识。删除应用内版本包保留部署文件、源码、会话、任务日志和安装历史；删除最新版本后选择剩余最新版本。
- 项目元数据新增可选 lastDeploymentDirectory，选择目录并通过预览校验时保存。再次安装提供使用之前的位置/重新选择。旧数据兼容读取，并从最近安装记录提供候选目录。
- 新安装直接覆盖，无备份和自动回退。复制后失败将目标标为 uncertain；新增 notAttempted 回退状态，历史备份记录保持可读。
- 原生删除先核对绝对目录边界，再暂时移走选定版本，元数据写入失败恢复目录；成功后清理应用内版本包。未实际删除用户版本或部署文件。
- 限定 vue-tsc（work/tsconfig.mod-install.json）与 cargo check --lib --locked --offline 通过；未运行测试、启动应用或验收覆盖安装与删除行为。当前决策已同步到 project-context.md。


## 2026-10-09：多会话并发、运行中引导与 SQLite

### 范围及实施结果

按 docs/2026-10-09-multi-session-steering-plan.md 任务 1–7 主代理顺序实施，没有使用子代理。执行前核对已提交源码与已有改动，保留 docs/README.md 及既有 output/work/索引内容，不提交。

- 数据库：新增 database/repository/migration 与 schema 001，rusqlite 0.40.2 bundled SQLite 3.53.2；WAL、外键、短事务、项目局部更新及 revision，旧项目按事务幂等导入、错误隔离；项目/会话/任务/摘要/引导/统计/恢复记录改为 SQLite。
- 多槽：原生 manager 与前端 runtime registry 按 modId/sessionId 独立维护代次、任务、pi 进程、消息、计时、草稿和附件；所有事件带身份，导航只切展示，后台持久事件及版本通知继续接收。
- 共享源码：FIFO 工具租约、读取哈希及 stale 写入拒绝；命令排队 revision 校验、后代进程回收后释放，prepare_delivery 沿用租约；回执和交付原生 revision 校验；保留脚本、fresh output、哈希等原有保护，记录任务操作和共享差异。
- 引导：受控 IPC 提交文本/图片，同 taskId/generation/进程；SQLite inputId 与队列/正式历史 entryId 关联，多个占位按 ID 替换；未确认不会自动重发，重启显示未关联项提示。
- 界面：运行时发送引导与独立停止按钮，精确禁用准备/压缩/停止；侧栏圆环与 hover 菜单分开占位，reduced motion 静态标记；恢复上下文统计注明历史采样；无关 AI 任务不禁用既有产物操作。
- 恢复：启动中断旧活动任务、处理未确认引导，保留稳定会话与 JSONL；安装及交付文件操作恢复记录避免数据库持久化失败被误报为成功。没有直接操作用户现有数据。

### 实际静态检查

- CodeGraph 已先执行 explore 获取入口；再核对当前源码、SDK 协议及旧元数据入口。
- Vue：node node_modules/vue-tsc/bin/vue-tsc.js --noEmit -p work/tsconfig.multi-session.json，退出 0。最初沙箱未正确读取 pnpm 链接，正常依赖权限下检查通过。
- runtime：node node_modules/typescript/bin/tsc --noEmit -p work/tsconfig.runtime.json，退出 0。
- Rust：cargo check --manifest-path src-tauri/Cargo.toml --lib --locked --offline，退出 0，无警告。中途修正实际编译错误后，对最终源码重新检查通过。
- 依赖：cargo fetch --manifest-path src-tauri/Cargo.toml --locked --target x86_64-pc-windows-msvc 完成；初次沙箱 DNS 失败，未修改全局网络配置。

### 未执行与用户验收

未运行测试、Lint、应用、模型或游戏；没有对现有用户数据运行迁移，没有修改游戏部署目录，没有新版安装包构建、提交或推送。现有安装包不包含此次代码。

计划每个 Gate 均待真实运行。用户补 Key 后在临时数据副本中验收迁移幂等/错误隔离、并发会话及切换、共享工具冲突/取消/后代停止、连续文本图片引导、过期交付拒绝、后台产物与重启恢复；旧历史、版本、安装记录和 skill 校验分别核对。静态检查不证明这些行为已经实际运行成功。


### 最终收尾结果

- node scripts/prepare-runtime.mjs：退出 0；Node 22.19.0 / pi 1.1.0，17,375 个运行时文件与 13 个 skills 文件已同步。之后只修改前端/Rust/文档，没有再改 runtime 扩展。
- git -c core.safecrlf=false diff --check：退出 0；另对本次 11 个新增文件进行限定空白检查，0 处问题。Git 初次默认检查仅提示 CRLF/LF，不是空白错误。
- codegraph sync → codegraph status：依次退出 0，status 返回 [OK] Index is up to date。仅维护索引，不作为运行验收。
- 最后 Vue 限定类型检查、Rust lib locked/offline 编译均再次退出 0，无 Rust 警告；runtime 检查通过后该范围未再改动。
- 最后核对补齐：空会话已归档但旧任务保留时 initialDraft 不丢失；新建会话使用事务重读；会话删除历史文件前有恢复日志；意外 stdout 退出记录 interrupted，手动停止记录 cancelled，退出通知刷新任务、项目和界面状态。


## 2026-10-10：启动时缺少 sessions 字段

- 用户实际启动报告 Tauri setup hook 失败：missing field sessions。确认由 SQLite 仓储的 DTO 组合顺序导致：save_on 按设计从 projects.payload 移除 sessions，read_on 却先将 payload 解为 ModProject，尚未查询独立 sessions 表便报错；initialize → interrupt_previous / reconcile_files → list → read_on 可达此路径。
- 本次只修改 repository::read_on：先解析项目 JSON 对象，查询未归档的 sessions 并合并，再反序列化完整 ModProject。保持旧文件迁移的必需字段校验，不给全局 DTO 添加缺省值；无需删除 SQLite、重新导入或修改用户数据。
- cargo check --manifest-path src-tauri/Cargo.toml --lib --locked --offline：退出 0，无警告。
- 未运行测试、未启动应用、未访问或修改用户应用数据。用户需重新执行 pnpm run dev:app 验收启动；本轮静态检查不等同于启动验收。

- 本次 codegraph sync → codegraph status 均退出 0，返回 [OK] Index is up to date。限定文档 git diff --check 退出 0；未跟踪 repository.rs 的 git diff --no-index --check 无空白错误输出（退出 1 表示存在文件差异），另对该源码和计划逐行空白检查退出 0，0 处问题。
