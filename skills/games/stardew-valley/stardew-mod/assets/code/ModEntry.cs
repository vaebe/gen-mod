using StardewModdingAPI;

namespace ExampleMod;

// 仅用于确认入口；生成任务应在此实现玩家需求，不能以该日志代替功能验收。
public sealed class ModEntry : Mod
{
    public override void Entry(IModHelper helper)
    {
        Monitor.Log("ExampleMod loaded. Gameplay changes are not implemented in this template.", LogLevel.Info);
    }
}
