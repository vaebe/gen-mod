# GenMod

Vue 3 + Tauri 2 的本地 AI MOD 工作台。使用 shadcn-vue、Tailwind CSS 4、Pinia、Vue Router 和 markstream-vue；pi 1.1.0 通过 RPC 执行任务，SDK 仅辅助读取只读历史。支持多会话并发，同项目共享源码，应用元数据由 SQLite 保存。

当前实现说明见 [项目上下文](docs/project-context.md)，文档同步日期为 2026-10-12。早期设计、实施记录与安装包的适用范围见 [文档入口](docs/README.md)。

UI 开发优先复用 `src/components/ui` 中的 shadcn-vue 组件，具体规范见 [项目协作规范](AGENTS.md) 和 [UI 组件约定](docs/project-context.md#ui-组件约定)。本次统一调整已完成限定文件的静态检查，尚未进行界面运行验收。

## 账号与 API 环境

业务 API 通过 Vite env 文件配置，变量为 `VITE_CLOUD_API_URL`。登录页不再选择环境或编辑服务地址；注册时填写邮箱、密码和确认密码，注册成功后自动登录。

- 本地开发：`.env.development` 默认使用 `http://127.0.0.1:8787`，启动 gen-mod-admin API 后运行 `pnpm dev:app`。
- 正式环境：将 `.env.production.example` 复制为 `.env.production`，替换实际 API 地址后运行 `pnpm build:app`。使用管理端同源代理时，地址应包含 `/api`；直接访问 API 时不加。
- 本机覆盖：可使用 `.env.development.local` 或 `.env.production.local`，这些文件不会提交。

未配置 `VITE_CLOUD_API_URL` 时默认请求本地 API。修改 env 后需要重启开发服务或重新打包。API 地址在检查登录状态前同步给原生端，账号、游戏与 Skill 请求使用同一地址；切换地址会清除旧登录凭证，保留本机项目与 Skill 文件。

env 中只填写业务 API 地址，不填写七牛密钥或其他服务端凭据。模型服务配置仍使用设置页，与业务 API 环境独立。

## 开发准备

Windows x64，Node >=22.19、pnpm 11.17、Rust 与 Tauri 的 Windows 编译环境。分发资源包含固定 Node 22.19.0 和 pi，普通用户无需全局 Node/pi。MOD 开发不要求安装对应游戏。

```powershell
pnpm install --ignore-scripts
pnpm prepare:runtime
pnpm tauri dev
```

依赖生命周期脚本默认不执行。运行时准备校验官方 Node ZIP、锁定 pi 版本、复制运行时依赖闭包并生成哈希清单，不复制真实 env、用户源码、日志或游戏 DLL。

## Windows 打包

```powershell
pnpm prepare:runtime
pnpm build:app
```

仅生成 NSIS `.exe` 安装包，默认 `pnpm build:app` 成功后自动复制到根目录 `build-output/`，原始产物保留在 `src-tauri/target/release/bundle/nsis/`。也可执行 `node scripts/collect-build-artifacts.mjs` 收集已有安装包。使用 LZMA 压缩减小安装包体积，制作时间会比 zlib 更长。

自动收集仅适用于不带额外参数的默认 Windows x64 release 构建；`build-output/` 不纳入 Git。收集脚本只复制已有安装包，不会重新构建。当前 LZMA 配置尚未重新打包，已有安装包仍使用 zlib；构建耗时、产物信息及验证边界见 [Windows 发布记录](docs/verification/windows-release.md#2026-10-12打包优化与当前配置)。

Rust release 使用 Thin LTO 和 16 个代码生成单元，保留 `opt-level = 3`。首次切换编译参数会重编译依赖；后续构建复用缓存。重复准备时，内容相同的扩展、helper 和资源清单不会重写，避免仅因时间戳变化触发 Rust 重建。

Node 分发仅包含 `node.exe` 和许可证；pi 保留运行依赖、类型声明及许可证，排除 source map，因此分发版本的依赖错误堆栈不能通过这些文件映射回原始源码。首次应用精简规则时，准备脚本将旧 pi 目录保留在 `work/pi-portable-before-slim-*`，再生成新目录与资源哈希清单。

### GitHub Actions 与 Release 下载

[Release Windows](.github/workflows/release-windows.yml) 在 Windows x64 runner 上沿用上述打包流程，使用 `package.json` 声明的 pnpm 版本和冻结锁文件安装依赖。工作流只生成 NSIS `.exe` 安装包，包含固定版本的 Node/pi 运行资源。

- **正式发布**：先修改 `package.json` 的 `version`，提交需要发布的代码，再推送同版本 `v*` 标签，例如版本 `1.0.1` 对应标签 `v1.0.1`。版本不一致会在构建前失败；Rust 和 Tauri 版本仍由现有 `build:app` 脚本同步。
- **下载入口**：标签构建成功后，工作流上传 `GenMod_<版本>_x64-setup.exe` 并自动公开 GitHub Release，用户可从仓库 **Releases → 对应版本 → Assets** 直接下载。带预发布标识的版本（例如 `1.0.1-beta.1`）会标记为 Pre-release。
- **手动构建**：在 **Actions → Release Windows → Run workflow** 选择分支运行，从该次运行的 **Artifacts** 下载，保留 14 天。手动运行标签时也会发布该标签的 Release。

发布使用 GitHub 自动提供的 `GITHUB_TOKEN`，工作流需要 `contents: write` 权限，无需配置个人 Token。新 Release 在安装包上传成功后才公开；失败时已有草稿保留，重跑会替换同名安装包附件。仓库可见性决定 Release 的访问范围，面向所有人下载时仓库须为公开仓库。安装包沿用现有配置，未添加 Windows 代码签名或自动更新。

实现参考 [Orbit 的发布工作流](https://github.com/ErKeLost/Orbit/blob/main/.github/workflows/release.yml)，本项目保留 pnpm 和现有 Windows 构建入口。此工作流尚未在 GitHub 执行，首次发布仍需核对 Actions 结果、Release 附件与实际安装行为。

## 模型配置

在设置页点击“添加 provider”，通过弹窗填写名称、唯一 ID、Base URL、API 协议和 API Key；在同一弹窗添加模型 ID、显示名称、上下文长度、最大输出以及图片输入和推理能力。模型参数按服务商规格填写。支持 OpenAI Chat Completions、OpenAI Responses、Anthropic Messages 和 Google Generative AI。

添加到列表后，选择默认模型并点击“保存配置”。编辑和移除同样在保存后生效；保存失败保留当前输入。修改不打断正在执行的任务，下次启动任务读取新配置。

开发和发布均使用 Tauri `appConfigDir/pi/models.json` 与 `settings.json`，不读取项目或应用目录的 `.env`。配置由 Rust 保存，Key 以明文保存在本机，读取接口不返回已保存的 Key；编辑时留空保留原 Key。命令子进程移除应用传入的 Key，日志继续脱敏。本地无鉴权服务可填写任意占位 Key。

每个 pi 进程使用独立临时配置快照，仅包含当前 provider 和模型，Key 通过环境变量引用；进程结束后清理快照。pi 不写回设置页的配置文件。命令后代移除 Key、脱敏集合和配置目录环境变量，日志遮蔽启动时全部已配置的 Key。Step Plan 的 `step-5-preview` 保留 low / medium / high 限制；模型的 `thinkingLevelMap` 在读取、编辑和保存时保留。

未配置默认模型或 Key 时仍能创建项目、保存草稿和管理已有版本。首版手动填写模型 ID，不包含远程模型列表获取、OAuth 登录或联网连接测试。

## Skills

`skills/registry.json` 管理分类、gameId、version、dependencies、相对路径和输出排除目录；`SKILL.md` 使用标准 YAML name/description。内置 `mod-workflow` 负责通用命令、ZIP 与交付协议，`duckov-mod` 负责游戏资料、C# 模板和编译脚本。

管理页支持本地目录导入、内容查看及启停。导入填写对应元数据，校验名称、引用及重复项后复制资源，不执行导入脚本；内置能力不可覆盖，用户副本需改名称及 SKILL.md。每个项目只加载所选游戏能力的通用依赖闭包。内容哈希覆盖全部资源；上下文改变后旧会话只读，新会话重新加载。活动任务期间不能变更能力。

Duckov 能力新增 API 查证、游戏功能、UI 与资源、Harmony、排查与验收专题，按任务读取；具体 API 仍须核对用户参考程序集。当前打包脚本不会自动收集额外资源或第三方 DLL。

## 会话与源码

会话在左侧项目下管理，切换页面不会停止后台任务。运行中可补充文本或图片要求，并单独停止目标会话；未确认输入不会在重启后自动重发。共享源码工具按读写租约协调，旧读取基线或排队期间源码变化会拒绝写入或命令，交付也校验源码 revision。

右侧上下文可收起；宽窗口使用可调整面板，窄窗口使用抽屉。“文件”提供全部源码的只读目录树、文本高亮、图片预览，以及当前任务操作记录和共享目录差异。“产物”可直接查看版本详情或进入版本管理。

发送时输入立即清空并显示本地提交状态；失败且输入未再编辑时恢复文本，成功只清理本次发送快照。已发送图片可点击放大，最终回复可复制。执行过程可展开查看；当前工作区还包含实时思考段展示，结束后及恢复历史时不展示思考文本。

需要编译的能力可声明 `requiredCommandScripts`（相对本 skill 的脚本路径）。原生交付校验要求本任务的成功脚本记录，并要求交付源文件位于 `.delivery/<taskId>/` 的新目录。静态资源可在本任务重新整理。

## 交付与安装

`run_command` 执行程序和参数数组，保存真实退出码与全文日志。`prepare_delivery` 在原生绑定的 staging 目录整理文件、安装清单和 ZIP；原生核对任务、必要命令、文件哈希和 ZIP 内容后登记版本。助手回复不作为成功证据。

项目存放在 Tauri `appLocalDataDir/mods/<modId>/`：

```text
source/                         当前源码与开发资料
artifacts/<versionId>/
  files/                        该版本交付文件
  version.json                  任务、skills 与哈希
  install.json                  通用相对路径安装映射
  mod.zip                       该版本 ZIP
logs/<taskId>/                  任务、命令、工具结果与保存的文件差异
installations/<installationId>/ 安装预览、备份与结果
mod.json                        旧元数据迁移源，保留但不再活动写入
```

`appLocalDataDir/gen-mod.sqlite3` 保存项目、会话草稿与偏好、任务、版本与安装关联、运行摘要及恢复记录；正式消息仍保存在 `logs/sessions/*.jsonl`。启动按项目事务迁移旧元数据，迁移错误隔离；重启中断旧活动任务，不恢复旧进程或重放需求。

排查运行问题时，工作台“日志”页展示最近 200 条摘要，“打开任务全文日志”进入本轮日志目录：

- `runtime-events.jsonl`：完整诊断、工具开始/结束、租约请求/响应及命令后代等待记录。用 `taskId`、`toolCallId`、`leaseId`、`commandId` 关联一次操作；不记录模型思考和环境变量，当前 API Key 会脱敏。
- `commands/<commandId>.log`：命令、参数、工作目录、开始时间、完整标准输出/错误、退出码、退出信号和耗时。命令回执同时包含最近 16,384 个字符的输出，编译错误可直接返回给 AI。
- `tool-results/`：工具最终结果；协调失败时也应结合事件和命令日志判断命令是否已经执行，不能仅凭工具超时认定“沙箱不可用”。

命令取得独占租约后记录 Job 内现有 PID，结束时只等待新增进程，避免误等 pi 的常驻控制台。后代等待超过 10 秒时，只回收本次命令新增的进程，记录 PID、程序路径和回收结果，确认退出后释放租约并返回命令结果，保留 pi 继续处理。再等待 5 秒仍未确认退出或回收失败时，才停止整个运行时；无法确认整个进程树退出则保留租约。工作区排队超过 25 秒时返回占用租约的任务、工具及操作信息。

版本管理按生成顺序倒序列出版本，显示时间、文件数、最新及当前已安装状态；行菜单提供详情、安装、导出 ZIP、打开目录和确认删除。点击短版本号可复制完整 ID，详情和安装记录在弹窗中查看。版本数量不限制，删除应用内版本包保留源码、会话、日志、部署文件和安装历史。

安装先选择或复用之前的目录，再展示完整映射与覆盖清单；用户确认后直接复制覆盖，不备份、不自动回退。复制失败且可能已修改目标时标记 uncertain，历史备份记录保留。新版本不自动安装或启用 MOD；“已安装”仅依据当前 known 部署记录，不代表检测过游戏目录的外部改动。

Duckov 模板使用 `netstandard2.1`。用户独立提供有效参考程序集目录；缺 DLL/SDK 时保留源码并报告缺项。编译由通用命令完成，不设手动构建按钮。游戏内启用、效果和兼容性验证由用户负责。

## 检查与验收

实施记录见 [实施进度](docs/verification/implementation-progress.md)，最终场景见 [Windows 验收记录](docs/verification/windows-release.md)。静态检查不能代替应用、模型、安装事务或游戏运行验收。

构建与真实验收按记录中的授权边界执行：

```powershell
pnpm build
pnpm tauri build
```

真实 Responses 验收需用户补 Key；安装验收先使用隔离临时目录，不默认操作真实游戏目录。不签名、不上传、不发布。

## 参考项目

本项目参考了 [Orbit](https://github.com/ErKeLost/Orbit)。Orbit 是基于 Tauri 2 和 React 构建的 Pi 编程智能体桌面工作台，通过真实的 Pi RPC 和 SDK 提供会话与任务交互能力。
