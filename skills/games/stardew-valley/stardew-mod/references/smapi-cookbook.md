# SMAPI C# 实现配方

适用桌面 1.6 / SMAPI 4.x。2026-11-06 核对公开接口与 Wiki；下述代码是实现参考，未编译或游戏验证。直接游戏成员仍须与玩家参考 DLL 对齐。使用现有 [工程模板](../assets/code/ExampleMod.csproj)，不要为这些例子增加 NuGet 依赖。

## 事件与对象有效期

| 时机 | 用途 | 不应据此推断 |
| --- | --- | --- |
| Entry | 读取配置、订阅事件 | 已加载存档或玩家存在 |
| GameLaunched | 向已加载的其他 MOD 获取 API | 正在进入某个存档 |
| SaveLoaded | 初始化当前存档状态 | 当前进程以后只会使用此存档 |
| DayStarted | 当日逻辑 | 在所有客户端各写一遍共享奖励是安全的 |
| Saving | 将当前状态写入本 MOD 的存档数据 | 写盘绝对成功或退出不保存也持久化 |
| ReturnedToTitle | 清空当前存档缓存和临时引用 | SMAPI 会卸载并重新运行 Entry |
| Input.ButtonPressed | 单次按键动作 | 菜单/对话期间允许执行世界操作 |

只为实际功能订阅事件。用 `Context.IsWorldReady` 保护世界访问，用 `Context.IsPlayerFree` 约束要求自由操作的快捷键；这与判断主机身份是不同条件。SMAPI Mod 不是 MonoBehaviour，没有 Unity 的 Awake/Update 生命周期。来源：[GameLoop 接口](https://github.com/Pathoschild/SMAPI/blob/develop/src/SMAPI/Events/IGameLoopEvents.cs)、[Input 接口](https://github.com/Pathoschild/SMAPI/blob/develop/src/SMAPI/Events/IInputEvents.cs)、[Context](https://github.com/Pathoschild/SMAPI/blob/develop/src/SMAPI/Context.cs)。

## 完整示例：可配置按键显示提示

需求示例：自由操作时按 K 显示提醒，不改世界或存档。用下面内容替换模板 ModEntry.cs；名称随实际项目调整。它展示配置、输入过滤和本地化，不代表自动实现其他玩法。

```csharp
using StardewModdingAPI;
using StardewModdingAPI.Events;
using StardewValley;

namespace ExampleMod;

public sealed class ModConfig
{
    public bool Enabled { get; set; } = true;
    public SButton ShowHintKey { get; set; } = SButton.K;
}

public sealed class ModEntry : Mod
{
    private ModConfig Config = new();

    public override void Entry(IModHelper helper)
    {
        Config = helper.ReadConfig<ModConfig>();
        helper.Events.Input.ButtonPressed += OnButtonPressed;
    }

    private void OnButtonPressed(object? sender, ButtonPressedEventArgs e)
    {
        if (!Config.Enabled || !Context.IsPlayerFree || e.Button != Config.ShowHintKey)
        {
            return;
        }

        Game1.addHUDMessage(new HUDMessage(Helper.Translation.Get("hint").ToString()));
        Helper.Input.Suppress(e.Button);
    }
}
```

随包提供 `i18n/default.json`：

```json
{ "hint": "Remember to bring your tools." }
```

`Helper.ReadConfig<T>()` 读取本 MOD 配置；需要保存玩家操作产生的配置变化时才调用 WriteConfig，不在每次启动覆盖玩家设置。多键组合按 SMAPI 的 KeybindList 文档适配，不把 ButtonPressed 当作每帧长按。Suppress 仅在本次按键确实由功能消费时调用。

来源：[IModHelper](https://github.com/Pathoschild/SMAPI/blob/develop/src/SMAPI/IModHelper.cs)、[输入帮助器](https://github.com/Pathoschild/SMAPI/blob/develop/src/SMAPI/IInputHelper.cs)、[翻译接口](https://github.com/Pathoschild/SMAPI/blob/develop/src/SMAPI/ITranslationHelper.cs)、[游戏 HUD 示例](https://stardewvalleywiki.com/Modding:Common_tasks#Banner_message)。

验收：进入存档自由操作按键显示一次提示；菜单或标题界面不执行；改配置后按对应重载方式复核；翻译文件确实进入 ResourceManifest。此功能只需本地显示，不声称具备网络同步。

## C# 编辑游戏内容

纯数据需求优先 CP。需要结合 C# 逻辑时，在 Entry 注册 `helper.Events.Content.AssetRequested += OnAssetRequested`，处理方法示例：

```csharp
private void OnAssetRequested(object? sender, AssetRequestedEventArgs e)
{
    if (!e.NameWithoutLocale.IsEquivalentTo("Data/Objects"))
    {
        return;
    }

    e.Edit(asset =>
    {
        var objects = asset.AsDictionary<string, StardewValley.GameData.Objects.ObjectData>().Data;
        if (objects.TryGetValue("388", out var data))
        {
            data.Price = 5;
        }
    });
}
```

使用前确认条目 ID 与 ObjectData 字段；这里只设置目标字段，其他数据保持原值。AssetRequested 可因缓存失效再次触发，不在此发奖励或写存档。需要动态重算时核对 `Helper.GameContent.InvalidateCache`，不要每帧清空全部内容缓存。

`Helper.ModContent.Load<Texture2D>("assets/icon.png")` 读取自己包内图片；`Helper.GameContent.Load<T>(assetName)` 读取游戏内容，二者路径和类型不同。PNG 加载放在绘制循环外，绘制事件只使用已准备好的资源。复杂菜单按实际 IClickableMenu/绘制事件接口实现，不使用 Unity Canvas。

来源：[内容 API](https://stardewvalleywiki.com/Modding:Modder_Guide/APIs/Content)、[ModContent](https://github.com/Pathoschild/SMAPI/blob/develop/src/SMAPI/IModContentHelper.cs)、[对象数据](https://stardewvalleywiki.com/Modding:Objects)。

## 状态存储与多人

| 数据 | 存储方式 | 时机与约束 |
| --- | --- | --- |
| 玩家偏好/快捷键 | ReadConfig/WriteConfig | 本机配置，不作为某个农场的奖励领取记录 |
| 本 MOD 的农场进度 | Helper.Data.ReadSaveData/WriteSaveData | 已加载存档且 Context.IsMainPlayer；SaveLoaded 读，Saving 写，ReturnedToTitle 清缓存 |
| 自有独立 JSON | Helper.Data.ReadJsonFile/WriteJsonFile | 路径相对 MOD；文件不存在读取可为 null；不是自动随当前存档隔离 |

存档写入属于当前天，退出不保存可能丢失。进度对象用可序列化公共属性、稳定键与显式结构版本；缺失记录初始化默认值，已有数据增量迁移，未知较新结构先报告不覆盖。来源：[IDataHelper](https://github.com/Pathoschild/SMAPI/blob/develop/src/SMAPI/IDataHelper.cs)。

多人按功能明确执行者：本地提示在当前屏幕；共享世界修改由明确的权威端执行。`Context.IsMainPlayer` 用于主机权限，`Context.IsWorldReady` 用于状态就绪，两者不能互代。需要跨端消息时使用 Multiplayer API；接收时核对发送 MOD、消息类型、发送玩家与业务权限，自己定义的 C# 字段不会自动同步。分屏缓存使用 `PerScreen<T>`，避免一个屏幕的选中对象影响另一个。来源：[多人 API](https://stardewvalleywiki.com/Modding:Modder_Guide/APIs/Multiplayer)。

一次性奖励必须把“是否已发”和发放操作关联起来：确认真实获得成功后再更新状态；失败保留可恢复信息，不在每次 SaveLoaded/DayStarted 无条件发放。如何处理背包满、跨天重试、断线和未保存退出以用户需求与实际 API 为准；需要这些规则时先确定，再实现。未核对存档写入与物品所有权时不提供假装可用的奖励代码。
