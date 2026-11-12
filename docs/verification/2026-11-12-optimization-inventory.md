# 项目优化覆盖基线与首批闲置清理

日期：2026-11-12。对应本地任务 01；基线提交 `e125e3c7b4927f3c52fef7df94c833adb74640fb`。

本文记录优化开始时的静态盘点及本票变更，不代表整体优化已完成或应用已验收。后续任务按当前源码更新自己的覆盖结果；路径迁移后由收尾任务 23 汇总，不将本文的基线路径当作最终架构。

## 分类与覆盖边界

在用指存在入口、消费者或明确资料用途；暂保留指已有明确保留决策；确认闲置必须完成引用和分发核对；生成物不因可重建而自动删除；历史资料不因日期旧而删除。以下覆盖七类目录的入口和资源链，未声称逐个证明全仓函数是否可达。

| 范围 | 已核对的入口、配置或清单 | 分类与结论 | 后续负责范围 |
| --- | --- | --- | --- |
| Vue 前端 | `index.html`、`src/main.ts`、`src/App.vue`、`src/router/index.ts`、`vite.config.ts`、`vitest.config.ts` | 在用；路由进入项目、聊天、版本、Skills 和设置。`SourceFileIcon.vue` 的动态 glob 仅加载 material-icon-theme 图标，不涉及本票 SVG | 02 版本；03–07 会话；15–19 能力目录；22 设置与登录 |
| Rust 原生 | `src-tauri/src/lib.rs` 模块声明、状态注册及 `generate_handler!`；`pi/runtime.rs`、`pi/bridge.rs`、`pi/sessions.rs` 的资源入口 | 在用；CLI、工具扩展、历史 helper 均有实际启动消费者。命令或模块没有前端显式入口，不能据此判废弃 | 08–14 任务、仓储及生命周期；15–19 能力；21 运行时 |
| Node 运行时 | `runtime/extensions/mod-tools.ts` 注册函数及导入；`runtime/helpers/read-session.mjs`；`runtime/runtime-lock.json` | 在用；mod-tools 是显式加载扩展，helper 由 Rust 调用。commands 中上下文职责与命令职责待任务 20 分离，不作为闲置删除 | 20 上下文；21 分发与版本 |
| 图片与资源 | `resources/games/catalog.json`、`resources/games/README.md`、`public/`、`src/assets/`、Tauri resources/icon 清单 | 游戏封面与图标按清单动态读取；planned 游戏仍有展示资料用途。品牌图有图标生成与设计源文件用途。只有下表三份模板确认闲置 | 19 游戏资源；21 分发；本票模板删除 |
| Skills | `skills/registry.json`、两个游戏 `SKILL.md` 的模板链接、`src-tauri/src/skills/registry.rs` 与 `official.rs`、`cloud/catalog.rs` | 在用；common 与两个游戏 Skill 通过清单选择。Duckov assets/template、Stardew assets/code 与 assets/content-pack 是工作模板，不能当脚手架残留删除；官方读取是共享能力 | 19 Skills；20 工具协议 |
| 脚本与构建 | `package.json`、`scripts/prepare-runtime.mjs`、`materialize-runtime.mjs`（由 prepare 导入）、`build-app.mjs`、`collect-build-artifacts.mjs`、`.github/workflows/release-windows.yml` | 在用；prepare 下载/部署/实体化并复制 extensions/helpers，保留已实现的精确退役清理；build 同步 Cargo/Tauri 版本；collect 复制固定版本安装包。legacy deploy 后恢复开发依赖是当前机制 | 21 构建治理 |
| 文档 | `docs/README.md`、`docs/project-context.md`、`docs/architecture/` 文件清单、`docs/design/2026-11-06-ui-and-brand.md`、兼容清理计划、`GLOSSARY.md` | 当前说明与历史设计/验证混合；README 有“当前工作区未提交”的历史状态。历史计划、架构产物和验证记录保留供逐份判断，不在本票迁移或删除 | 23 文档归档与最终覆盖核对 |

## 本票确认闲置并删除的文件

删除前在干净的独立 worktree 对全部 Git 跟踪内容执行 `git grep -n -E 'vue[.]svg|vite[.]svg|tauri[.]svg'`，无匹配（退出码 1 表示未找到）。主仓未跟踪优化方案只包含删除候选说明，非消费入口。另核对前端动态加载、HTML、路由、原生命令注册、资源清单、脚本复制和测试发现规则。

| 文件 | 静态/动态入口与注册 | 构建、资源及文档结论 | 处理 |
| --- | --- | --- | --- |
| `src/assets/vue.svg` | 无导入；无覆盖它的 glob；非路由或原生命令资源 | 无显式复制、资源清单、测试或既有跟踪文档引用 | 删除 Vue 模板图 |
| `public/vite.svg` | 无 HTML/CSS/组件引用；favicon 为 `/gen-mod-icon.png`；无动态拼接消费者 | Vite 默认 public 复制会分发该无用文件；Tauri 前端入口为 dist；原生 resources 不引用它。无测试或既有跟踪文档引用 | 删除 Vite 模板图，停止随 public 分发 |
| `public/tauri.svg` | 无 HTML/CSS/组件引用；非 Tauri bundle icon；无动态拼接消费者 | 同属 public 静态复制；Tauri icon 清单使用 icons 下 png/icns/ico。无测试或既有跟踪文档引用 | 删除 Tauri 模板图，停止随 public 分发 |

本票不修改运行时源文件，不涉及 runtime/resources 的增量旧副本。没有执行 prepare、构建或清空已有 dist；既有本地产物不被视为已更新安装包。

## 保留项与候选交接

| 对象 | 证据/约束 | 决定 |
| --- | --- | --- |
| `src/features/auth/`、`src/features/skills/CloudUpdates.vue`、`src-tauri/src/cloud/` | 当前路由没有登录入口，但保留决策明确；lib.rs 仍注册 cloud 命令 | 暂保留，任务 22 迁移消费者时复核，不作为闲置清理 |
| `src-tauri/src/skills/official.rs` | registry 的 available_on/definition 与 cloud/catalog 的写入共同依赖 | 在用共享能力，保留离线已下载官方 Skills |
| `public/gen-mod-logo.svg`、`public/gen-mod-app-icon.svg` | 品牌设计说明记录源文件用途，并提供以 app-icon.svg 生成应用图标的命令 | 在用设计资产，保留，不以零组件引用删除 |
| `public/gen-mod-icon.png`、`src-tauri/icons/` | HTML、登录品牌和 Tauri icon 清单使用 | 在用，保留 |
| 版本页面及 ContextPanel 的重复操作 | 现有功能在用，重复不等于废弃 | 任务 02 复用后再核对旧实现消费者 |
| 会话可写代理、bridge 编排、聚合写入、workbench 能力组件 | 当前为业务路径，尚未证明可删除 | 分别交 03–18；只有替代入口和全部消费者完成迁移后移除失去用途的部分 |
| 独立废弃链路 | 本票未发现有完整入口、动态调用、分发及文档证据支持删除的其他独立链路 | 不虚构新任务；后续发现须先记录有限文件范围与删除证据再交接，不标记已完成 |

## 生成物、历史资料及用户资料

- `.gitignore` 列明 `node_modules`、`dist`、`dist-ssr`、`build-output/`、`runtime/resources/`、`output/`、`work/`。依赖及构建输出归生成物；`work/` 可含 prepare 保留的失败部署，`output/` 可含人工图像资产，不能统一当垃圾。这里只核对忽略与生产脚本，不遍历用户文件内容。
- `docs/verification/` 保存过去执行证据，旧设计和 plans 保存决策来源；`docs/architecture/` 中 HTML/JSON 为有来源的文档产物，任务 23 逐份区分有效说明与历史记录。
- `.scratch/` 是本地任务资料；本票不修改主仓票状态。数据库、凭证、MOD 工作文件、草稿、安装记录、游戏目录及长期记忆全部排除删除范围，不执行数据重置或批量清理。

## 检查、限制与回退

已完成 Git 基线/差异、全部跟踪内容中的模板名称引用、入口与分发静态核对；执行 `git diff --check` 及删除后引用复核。CodeGraph 在独立 worktree 初始化后执行 explore，变更后执行 sync/status；索引维护不等于功能验证。

本票没有业务行为或接口变更，按规格不为模板删除增加镜像测试。未运行测试、Lint、类型检查、构建、桌面应用、模型或游戏；未宣称 UI 或打包运行验收。其他能力的闲置判定随对应任务继续，本文不将基线盘点等同于全量清理完成。

本批实际变更仅三份 SVG 删除及本文新增。回退可恢复这三份基线文件并移除本文，或对本票独立提交执行 revert；无需清空数据库、工作目录或生成目录。操作前仍需核对当时工作区，避免覆盖后续变更。
