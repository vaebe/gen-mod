---
name: stardew-mod
description: 当在 GenMod 为星露谷物语制作、续改或交付 Content Patcher 内容包或 SMAPI C# MOD 时使用。
---

# 星露谷物语 MOD

面向桌面版 Stardew Valley 1.6、SMAPI 4.x。先读取[开发与交付](references/development.md)，按实际目标版本核对[官方资料](references/sources.md)。不适用于手机版、主机版或 1.5 兼容分支。

## 按需求读取知识

| 需求 | 必须读取 | 得到什么 |
| --- | --- | --- |
| 修改对话、数值、贴图、条件开关 | [Content Patcher 配方](references/content-patcher.md) | 完整 JSON 示例、字段编辑、配置、图片坐标与资源清单 |
| 按键、事件、配置、C# 内容编辑 | [SMAPI 配方](references/smapi-cookbook.md) | 完整入口示例、事件选择、内容缓存与翻译 |
| 物品、商店、作物、NPC、地图 | [游戏数据与内容依赖](references/game-data.md) | 数据入口、ID、功能之间的关联和逐项完成条件 |
| 写入进度、一次性奖励、多人 | [SMAPI 状态与多人](references/smapi-cookbook.md#状态存储与多人) | 存储位置、主机权限、重复执行与同步边界 |
| 加载失败、补丁无效、反馈修复 | [排错与验收](references/troubleshooting.md) | 分阶段定位、CP 控制台证据和具体反馈项 |

从表中读取本次功能对应资料后再写实现，不只复制模板。专题中的示例是已对照来源编写的起点，未在玩家环境编译或运行；采用前核对所用版本。外部资料不可访问时仍可用专题中的已知结构推进，新增且没有依据的成员保留为明确缺项。

## 选择实现路线

- 对话、图片替换、物品与游戏数据调整：优先 Content Patcher；采用 [manifest](assets/content-pack/manifest.json)、[content](assets/content-pack/content.json) 和[玩家说明](assets/content-pack/README.md)。这条路线不需要编译器或游戏程序集，但运行需要 SMAPI 与 Content Patcher。
- 新逻辑、输入交互或事件处理：采用 SMAPI C#；模板为[工程](assets/code/ExampleMod.csproj)、[入口](assets/code/ModEntry.cs)、[manifest](assets/code/manifest.json) 和[玩家说明](assets/code/README.md)。构建需要 .NET 6 SDK 与合法参考程序集。
- 已有项目沿用适用的路线和 UniqueID，不重新初始化源码，不因一次续改重置配置或存档。只有当前需求无法由原路线实现时才讨论改变。

新建时把模板复制到 source 的工程目录后再修改，不编辑技能安装目录。替换示例名称、作者、UniqueID 和 README 中的占位内容，实现真实需求。空 Changes 和模板日志都不能作为功能完成的证据。

先完成一个可观察功能，再扩展剩余需求。新增游戏 API、数据键、事件、图片布局或 Content Patcher token 必须核对目标版本的真实资料；不把其他游戏的 Unity/Harmony 用法套到星露谷。涉及游戏存档只使用经核对的 API，明确单人/多人、主机/客机和卸载影响。

## 构建与安装

两条路线均通过原始 [build.ps1](scripts/build.ps1) 校验并整理到本任务全新输出目录，再遵循通用 MOD 工作流的 `prepare_delivery` 协议登记版本。内容包路线的成功回执表示结构校验与整理，不表示 C# 编译或游戏加载成功。

安装目标选择游戏的 **Mods 目录**；交付 target 为 `<ModFolder>/manifest.json` 等相对路径，不再添加一层 Mods。游戏和 SMAPI/Content Patcher 由玩家准备，GenMod 不自动下载、安装或打包这些前置与游戏程序集。

依赖不足时保留源码，明确缺项；不要伪造 DLL、构建回执或游戏验收。运行时技能只读，需求和环境事实写入项目记忆，复用知识按通用协议写入长期记忆。

交付时通过 `prepare_delivery` 的 `summary` 参数提供本轮实际交付变化的简短摘要。摘要缺失或为空时先补充后重试，不把编译成功或玩家尚未验证的效果当成改动摘要。
