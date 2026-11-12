# 运行时与构建生命周期

`runtime/runtime-lock.json` 是内置 Node/pi 的版本来源。Rust 编译期嵌入该文件，统一解析 CLI、显式 `mod-tools.ts` 扩展、`read-session.mjs` 历史 helper 及它们的 Node 路径；不回退到系统 Node。

依赖安装仍由 pnpm 锁文件管理。升级时修改 runtime lock，并同步根和 runtime 的 package.json 依赖声明、pnpm-lock.yaml 及 Tauri 精确资源白名单。`node scripts/runtime-contract.mjs` 在准备/构建之前检查必要声明是否一致，拒绝旧版本白名单。准备/构建还检查分发 pi 包版本、真实内置 Node 版本与三个入口文件。文件存在检查不代表入口运行成功。

| 阶段 | 副作用与边界 |
| --- | --- |
| `prepare:runtime` | 下载并校验锁定 Node ZIP；保留 `pnpm deploy --prod --legacy --ignore-scripts`；实体化依赖闭包；覆盖更新 extensions/helpers；仅删除确认退役的 `extensions/assets.ts`、`helpers/process-image.ps1`；写资源哈希清单。失败或规则变化的旧部署移动至 work 留存，不批量清目录。 |
| 恢复开发依赖 | legacy deploy 可改变源工作区依赖状态，CI 在准备后执行 `pnpm install --frozen-lockfile --ignore-scripts --prod=false`。本地准备后构建也应保留此步骤。 |
| `build:app` | 先验证运行时；应用版本仍取 package.json，同步 Cargo.toml 与 tauri.conf.json；调用 Tauri（触发前端 build）；只在无附加参数成功构建后收集产物。 |
| collect | 复制当前产品/版本的 Windows x64 NSIS 到 build-output，不删除旧安装包。 |

CI 宿主 Node 负责 pnpm/Vite/Tauri 脚本，与随应用分发的 Node 用途不同；宿主升级不隐式升级内置运行时。直接调用 `tauri build` 不经过 build-app 的前置校验，应使用项目 `build:app` 入口。

本轮工具职责拆分新增 task-context、task-contracts、diagnostics，原 commands.ts 仍是命令入口，通过覆盖复制更新；没有新增退役路径。生成目录、work、output、build-output 和失败现场不做批量清理。

验证分开记录：Node fixture 测试覆盖声明漂移拒绝及干净/重复资源复制和定点清理；无模型烟测使用已准备依赖、当前扩展/helper 源文件及内置 Node，分别执行 CLI 版本启动、pi SDK 扩展加载和 fixture 历史读取。它们不证明下载、安装包构建、安装后的资源定位、桌面任务启动或真实模型/游戏验收。
