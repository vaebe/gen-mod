# 游戏与 Skills 能力归属

游戏展示、图片及目录调查归 `src/features/games`，类型化读取与调查命令集中在 `service.ts`。调查采纳写入项目记忆，继续调用 `projectMemoryService.applyInspection`。游戏与调查的跨端类型保留在 `src/contracts/game.ts`、`inspection.ts`。

Skills 管理、正文读取、导入、开关及暂保留的云端更新归 `src/features/skills`。所有消费者（包括新建项目与长期记忆设置）通过 `skillsService` 使用明确参数和返回类型；类型位于 `src/contracts/skill.ts`。云端认证仍由 auth 维护，云端更新界面没有移除。

`resources/games/catalog.json` 的可选 `installHint` 保存游戏特有目录说明和示例。交付清单读取仍先验证版本路径；没有特有说明时沿用交付清单的通用提示。提示只用于展示，安装预览、令牌、路径及文件校验仍由 Rust 执行。目录支持状态没有变化，当前仅 Duckov 和 Stardew Valley 已适配。

资源分发继续使用 Tauri 的 `resources/games/` 与 `skills/` 映射。Skills 本地、已下载官方版本的注册校验、选择、内容哈希与任务快照仍由原生 `skills` 实现。新增已适配游戏的提示修改目录资源，不添加 Rust 游戏 ID 分支。
