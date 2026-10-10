# GenMod

Vue 3 + Tauri 2 的本地 AI MOD 工作台。使用 shadcn-vue、Tailwind CSS 4、Pinia、Vue Router 和 markstream-vue；pi 1.1.0 通过 RPC 执行单个活动任务，SDK 仅辅助读取只读历史。

## 开发准备

Windows x64，Node >=22.19、pnpm 11.17、Rust 与 Tauri 的 Windows 编译环境。分发资源包含固定 Node 22.19.0 和 pi，普通用户无需全局 Node/pi。MOD 开发不要求安装对应游戏。

```powershell
pnpm install --ignore-scripts
pnpm prepare:runtime
if (-not (Test-Path -LiteralPath .env)) { Copy-Item .env.example .env }
pnpm tauri dev
```

依赖生命周期脚本默认不执行。运行时准备校验官方 Node ZIP、锁定 pi 版本、复制运行时依赖闭包并生成哈希清单，不复制真实 env、用户源码、日志或游戏 DLL。

## 模型配置

开发读取项目根 `.env`；发布读取 Tauri `appConfigDir/.env`，设置页可打开该目录。配置变更后，空闲时重新连接或开始新任务读取新值。

```dotenv
STEPFUN_BASE_URL=https://api.stepfun.com/step_plan/v1
STEPFUN_MODEL=step-5-preview
STEPFUN_API_KEY=
```

API 固定为 `openai-responses`。Key 由 Rust 读取并传给 pi 子进程，前端仅展示配置状态。无 Key 仍能创建项目、保存草稿和查看管理页，最终真实模型验收时由用户补充。

## Skills

`skills/registry.json` 管理分类、gameId、version、dependencies、相对路径和输出排除目录；`SKILL.md` 使用标准 YAML name/description。内置 `mod-workflow` 负责通用命令、ZIP 与交付协议，`duckov-mod` 负责游戏资料、C# 模板和编译脚本。

管理页支持本地目录导入及启停。导入填写对应元数据，校验名称、引用及重复项后复制资源，不执行导入脚本；内置能力不可覆盖，用户副本需改名称及 SKILL.md。每个项目只加载所选游戏能力的通用依赖闭包。内容哈希覆盖全部资源；上下文改变后旧会话只读，新会话重新加载。活动任务期间不能变更能力。

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
mod.json                        项目、草稿、会话、版本与安装关联
```

排查运行问题时，工作台“日志”页展示最近 200 条摘要，“打开任务全文日志”进入本轮日志目录：

- `runtime-events.jsonl`：完整诊断、工具开始/结束、租约请求/响应及命令后代等待记录。用 `taskId`、`toolCallId`、`leaseId`、`commandId` 关联一次操作；不记录模型思考和环境变量，当前 API Key 会脱敏。
- `commands/<commandId>.log`：命令、参数、工作目录、开始时间、完整标准输出/错误、退出码、退出信号和耗时。命令回执同时包含最近 16,384 个字符的输出，编译错误可直接返回给 AI。
- `tool-results/`：工具最终结果；协调失败时也应结合事件和命令日志判断命令是否已经执行，不能仅凭工具超时认定“沙箱不可用”。

命令取得独占租约后记录 Job 内现有 PID，结束时只等待新增进程，避免误等 pi 的常驻控制台。后代等待超过 10 秒时，只回收本次命令新增的进程，记录 PID、程序路径和回收结果，确认退出后释放租约并返回命令结果，保留 pi 继续处理。再等待 5 秒仍未确认退出或回收失败时，才停止整个运行时；无法确认整个进程树退出则保留租约。工作区排队超过 25 秒时返回占用租约的任务、工具及操作信息。

工作台右侧“产物”可进入版本管理。选定版本后可打开目录、导出 ZIP 或安装。安装先选目录并展示全部映射与覆盖清单；用户确认后备份、复制，失败尝试回退，回退失败显示异常状态。新版本不自动覆盖旧版本，也不自动安装或启用 MOD。

Duckov 模板使用 `netstandard2.1`。用户独立提供有效参考程序集目录；缺 DLL/SDK 时保留源码并报告缺项。编译由通用命令完成，不设手动构建按钮。游戏内启用、效果和兼容性验证由用户负责。

## 检查与验收

实施记录见 [实施进度](docs/verification/implementation-progress.md)，最终场景见 [Windows 验收记录](docs/verification/windows-release.md)。静态检查不能代替应用、模型、安装事务或游戏运行验收。

构建与真实验收按记录中的授权边界执行：

```powershell
pnpm build
pnpm tauri build
```

真实 Responses 验收需用户补 Key；安装验收先使用隔离临时目录，不默认操作真实游戏目录。不签名、不上传、不发布。
