---
name: duckov-mod
description: 当在 GenMod 为逃离鸭科夫（Duckov）新建、续改、排查或交付 C# MOD 时使用。
---

# 逃离鸭科夫 MOD

用于 Duckov 原生加载器的 C# MOD 开发，不默认采用 BepInEx 或第三方框架。核心上游资料已于 2026-11-06 重新核对；网页中的成员是查找入口，新增调用须核对目标版本。先读[来源](references/api-sources.md)与[依赖及构建](references/dependencies.md)，再按任务读取专题。

## 选择当前任务

| 任务 | 读取与行动 |
| --- | --- |
| 新建或续改 | 检查 source 中的工程和改动，明确目标、验收场景、版本、依赖与缺项；只有新工程采用下方模板 |
| 新 API、版本不符 | [API 查证](references/api-verification.md)：对象与触发时机 → 当前签名 → 保护条件和副作用 → 证据边界 |
| 需要实际写法、从需求开始实现 | [功能配方](references/implementation-recipes.md)：悬浮 UI 完整入口、物品投递、数值效果、异步与资源的实现顺序 |
| 物品、背包、角色、对话、本地化 | [游戏功能](references/gameplay.md) |
| 生命周期、悬浮 UI、小地图、模型或资源 | [UI 与资源](references/ui-assets.md) |
| 改原方法或补丁兼容 | [Harmony](references/harmony.md)；先判断公开 API/事件能否满足需求 |
| 构建和交付 | [依赖及原始脚本参数](references/dependencies.md#构建与交付命令)、通用能力的交付协议 |
| 玩家反馈或切场景失效 | [排查与验收](references/troubleshooting.md)，先核对反馈对应的交付版本 |

只读取当前任务需要的专题。复用仍适用的事实，查证发生变化的部分。路线优先已有配置/数据，再用公开 API/事件；确需 Harmony 时说明目标和原因。安装新前置需玩家授权。

每项功能先确定触发入口、操作对象、所需 API/资源和可观察结果。直接采用已核对的配方结构；目标版本差异只查相关类型与调用链。缺少玩家获取、伤害、存档等接口依据时准确列出需要的符号或实现，不用看似合理的名称补齐代码。

运行时游戏技能、参考资料与原始脚本只读。需求与环境事实按通用能力写入项目记忆，可复用开发知识保存长期记忆；不向本技能写开发笔记、猜测 API 或玩家私人信息。记忆只作参考，当前用户决定与已核对的目标版本优先。导入素材是 source 中的普通文件，不要求素材 ID 或格式转换。

## 工程入口与开发节奏

新工程使用 [工程文件](assets/template/ExampleMod.csproj)、[入口](assets/template/ModBehaviour.cs)、[元数据](assets/template/info.ini)、[玩家说明](assets/template/README.md)。模板只显示 MOD 信息；按实际需求替换功能、名称、路径和验收内容，玩家说明中不保留编辑指令。

ModName 使用合法 C# 标识符；AssemblyName、namespace、info.ini 的 name 与 DLL 文件名保持一致。入口是公开的 `<ModName>.ModBehaviour`，继承 `Duckov.Modding.ModBehaviour`，由加载器创建。

多项功能先贯通一项可观察行为的入口、触发、业务效果和文件交付，再扩展其余已授权功能。加载日志或模板文字只能证明入口，不代表业务完成。范围和方案已确认时连续推进；新的业务歧义、行为变化或授权需求才集中确认。

游戏内结果由玩家反馈。等待某一步反馈时继续不依赖该结果的已授权工作，并明确阶段未完成项。构建依赖不足时保存源码并报告缺项，不伪造 DLL、API、命令结果或验证记录。
