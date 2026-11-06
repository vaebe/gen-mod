# 游戏功能定位

下列成员来自上游文档，相关 README 与 API 页面已于 2026-11-06 重新读取；尚未针对用户参考程序集编译或运行。使用前按 [API 查证](api-verification.md) 核实当前版本；实际开发顺序见 [功能配方](implementation-recipes.md)。未列出的武器、伤害、任务、技能、生成器接口不能凭名称推测。

## 物品与背包

| 目标 | 上游提供的定位入口 | 需要核实的语义 |
| --- | --- | --- |
| 创建物品实例 | ItemStatsSystem.ItemAssetsCollection.InstantiateAsync(int typeID)，返回 UniTask<Item>；InstantiateSync(int typeID)，返回 Item | TypeID 是否有效、数据是否初始化、同步加载成本 |
| 送给玩家 | ItemUtilities.SendToPlayer(Item, bool dontMerge=false, bool sendToStorage=true) | 合并、背包空间和仓库行为，以当前实现为准 |
| 尝试投递 | SendToPlayerCharacter、SendToPlayerCharacterInventory，返回 bool | 返回 false 时物品归属、是否已经发生部分变更 |
| 查询物品位置 | IsInPlayerCharacter、IsInPlayerStorage 扩展方法 | 玩家装备、背包和仓库的实际判定范围 |
| 安装部件 | TryPlug(Item main, Item part, bool emptyOnly=false, Inventory backupInventory=null, int preferredFirstIndex=0) | 槽位类型、替换规则、失败与备选背包行为 |
| 移除原归属 | Item.Detach() | 会解除槽位或 Inventory 关系，不等于销毁或成功投递 |

不要把向玩家投递自动当作“只进入背包”；需要背包限制时确认实际方法与失败路径。投递前后不要重复实例化同一奖励，异步结果回来后重新确认场景、角色和需求是否仍有效。

异步创建应在返回 UniTask 的方法中 await；不把 UniTask<Item> 当 Item，也不用 .Result/Wait 阻塞 Unity 主线程。游戏方法不支持取消时，取消外围任务不等于取消已发生的生成副作用，应核对迟到结果的归属和清理。

TypeID 必须从当前数据、可信资源定义或程序集相关实现确认，不从名字、旧 Wiki 或示例数字猜测。修改物品属性时分清共享 prefab、数据定义与实例，避免把一次奖励的变化应用到全部同类物品。

## 自定义物品与存档

上游 README 提供 ItemAssetsCollection.AddDynamicEntry(Item prefab) / RemoveDynamicEntry(Item prefab) 的注册线索。先确认调用时机、prefab 配置与 TypeID 冲突处理，不把复制 C# 类当作完整物品资源。

同一物品的 TypeID 在后续版本中保持稳定；同时核对游戏和依赖 Mod 的占用情况。上游明确提示：缺少对应 Mod 时，存档中的自定义物品可能直接消失。需求涉及这类物品时，在交付说明中写明前置和卸载影响，不承诺移除 Mod 后存档自动恢复。

## 角色、敌人、战斗与任务

CharacterMainControl 是角色核心组件的定位入口；上游列出 SetTeam(Teams)。改变阵营前确认目标角色与 Teams 枚举含义，不能默认应用到所有角色。

敌人生成章节尚未提供实现。玩家获取、伤害、射击、敌人生成、任务推进、存档写入等需查当前程序集：先找实际入口及过滤条件，再核对调用链。通过 class 名称或字段名字猜测“玩家单例”“无敌开关”“任务完成函数”不构成证据。

对玩家专属效果确认玩家身份判定；对数值效果确认单位、计算阶段、是否叠加、是否同时影响 NPC。用户未定义的倍率、默认状态、持续时间不自行补成业务规则。

## 对话与本地化

- 大对话：Dialogues.DialogueUI 的 DoSubtitle(SubtitlesRequestInfo)，返回 UniTask。参数类型定位到 NodeCanvas.DialogueTrees.SubtitlesRequestInfo，构造方式与单例有效期另查；多个并发调用可能相互影响。
- 气泡：Duckov.UI.DialogueBubbles.DialogueBubblesManager.Show(string text, Transform target, float yOffset=-1, bool needInteraction=false, bool skippable=false, float speed=-1, float duration=2f)，返回 UniTask。核对目标 Transform 生命周期与交互需求。
- 本地化：SodaCraft.Localizations.LocalizationManager.SetOverrideText(string key, string value) 与 OnSetLanguage 事件是入口。核对当前版本的覆盖清理能力；没有证据时不编造 RemoveOverride 方法。语言切换更新已有 UI，退出时退订事件。

依赖 UniTask 时使用参考目录已有版本；异步操作的异常、取消和对象有效期一起处理。来源：[UniTask 官方说明](https://github.com/Cysharp/UniTask)。

本页游戏成员来源：[NotableAPIs_CN](https://github.com/xvrsl/duckov_modding/blob/main/Documents/NotableAPIs_CN.md)、[上游 README](https://github.com/xvrsl/duckov_modding/blob/main/README.md)。
