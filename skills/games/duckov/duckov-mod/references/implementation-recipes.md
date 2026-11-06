# 从需求到实际实现

原生 Duckov ModBehaviour / netstandard2.1 路线。2026-11-06 对照下列上游源码与文档编写；示例未针对玩家 DLL 编译或运行。先核对 [依赖](dependencies.md) 和本次用到的签名，不把它们当成跨版本 SDK。

## 完整示例：物品悬浮时显示原始价值

这是“显示 GetTotalRawValue 返回值”的示例，不声称表示成交价。放入现有模板 ModBehaviour.cs，namespace、AssemblyName、info.ini name 同步改成实际名称。需有定义 ItemHoveringUI、GameplayDataSettings、Item/ItemMetaData、TMP 和 uGUI 的参考程序集。

```csharp
using Duckov.UI;
using Duckov.Utilities;
using ItemStatsSystem;
using TMPro;
using UnityEngine;

namespace ExampleMod
{
    public sealed class ModBehaviour : Duckov.Modding.ModBehaviour
    {
        private TextMeshProUGUI? label;

        private void OnEnable()
        {
            ItemHoveringUI.onSetupItem += ShowValue;
            ItemHoveringUI.onSetupMeta += HideForMeta;
        }

        private void OnDisable()
        {
            ItemHoveringUI.onSetupItem -= ShowValue;
            ItemHoveringUI.onSetupMeta -= HideForMeta;
            ClearLabel();
        }

        private void OnDestroy() => ClearLabel();

        private void HideForMeta(ItemHoveringUI ui, ItemMetaData data)
        {
            if (label != null)
            {
                label.gameObject.SetActive(false);
            }
        }

        private void ShowValue(ItemHoveringUI ui, Item item)
        {
            if (ui == null || item == null)
            {
                ClearLabel();
                return;
            }
            if (label == null)
            {
                label = Instantiate(GameplayDataSettings.UIStyle.TemplateTextUGUI);
            }
            label.transform.SetParent(ui.LayoutParent, false);
            label.text = $"Raw value: {item.GetTotalRawValue()}";
            label.gameObject.SetActive(true);
        }

        private void ClearLabel()
        {
            if (label != null)
            {
                Destroy(label.gameObject);
            }
            label = null;
        }
    }
}
```

本例等待悬浮事件后才创建文字，禁用时退订并清理；UI 重建后旧的 Unity 对象可能已销毁，下次事件重新获取。需要把显示文字本地化时按 [游戏功能](gameplay.md#对话与本地化) 接入，而非假设字体支持所有字符。

验收：悬浮实际物品显示当前数值；元数据预览不残留；换物品更新；禁用消失，重新启用不叠加；重建背包 UI 后仍有效。来源：[DisplayItemValue 作者示例](https://github.com/xvrsl/duckov_modding/blob/main/DisplayItemValue/ModBehaviour.cs)、[Unity OnDisable](https://docs.unity3d.com/ScriptReference/MonoBehaviour.OnDisable.html)、[Destroy](https://docs.unity3d.com/ScriptReference/Object.Destroy.html)。自己的 GameObject 与游戏 UI 分清所有权，不销毁 LayoutParent 或游戏模板。

## 发物品：生成、配置、交付、确认归属

按 [游戏 API](gameplay.md#物品与背包) 的实际目标版本执行：

1. 从当前数据核对 TypeID 和请求数量；先确认是一次触发、限次奖励还是可重复操作。
2. 在已核对的角色/场景就绪条件内生成实例。异步写法是 `Item item = await ItemAssetsCollection.InstantiateAsync(typeId);`，方法返回 UniTask，不能直接把异步调用赋给 Item。同步版本只适合已确认可接受同步成本的路径。
3. 异步回来重新检查启用状态、场景与目标角色；迟到结果尚未投递时仍需负责释放或收容，具体方法先查 Item 的所有权/销毁实现，不能假定销毁 Transform 就完成存档清理。
4. 根据需求选择 SendToPlayer 或只尝试玩家背包的接口。默认 SendToPlayer 的 sendToStorage=true；需要“背包满就失败”时不能直接照用默认。
5. 核对失败时实例仍由谁持有，成功后不再次投递。要发多件时确认堆叠 API 与上限，不能从创建一个实例推断已获得请求数量。

异步调用要有异常归属：由调用者 await，或采用游戏现有 UniTask 版本支持的异常回调；不要裸 async void 隐藏异常。忙碌标记在 finally 清除，防止双击重复生成；禁用与销毁会结束当前启用周期，旧结果不得影响重启后的新周期。具体取消和清理规则见 [UI 生命周期](ui-assets.md)。

验收：正确物品和数量、正常背包、背包满、重复操作、生成中禁用/换场景。只有奖励实际完成才更新领取记录；记录存储接口未核对时不承诺跨存档或跨会话限次。来源：[NotableAPIs_CN](https://github.com/xvrsl/duckov_modding/blob/main/Documents/NotableAPIs_CN.md)、[UniTask](https://github.com/Cysharp/UniTask)。

## 数值效果与战斗：先确定计算位置

例如“玩家造成伤害翻倍”至少涉及伤害来源、受击者、计算阶段和最终结果；只改一个名为 Damage 的值无法证明满足需求。

| 步骤 | 需要取得的证据 | 实现决策 |
| --- | --- | --- |
| 找入口 | 当前程序集中的伤害计算/应用方法及直接调用者 | 是计算数值、实际扣血还是仅显示 UI |
| 找对象 | 来源角色、目标角色、玩家身份判定 | 玩家增伤不会变成敌人增伤；宠物/投射物按需求处理 |
| 找阶段 | 护甲、暴击、倍率、最小伤害和其他修正的位置 | 选定一个阶段应用一次，避免多层重复乘算 |
| 找生命周期 | 属性是基础值、修正器、缓存还是最终值 | 持续效果不能每帧把结果再乘一次 |
| 找撤销 | 当前 API 是否支持独立修正标识或移除句柄 | 移除自己的贡献，避免恢复旧快照覆盖其他 MOD 的新值 |

优先公开修正器/事件；确实要改原方法才用 [Harmony](harmony.md)。不能用寻找相似名字的反射循环替代目标查证。未确认玩家、战斗、生成器或任务入口时，该部分保持阻塞并报告所缺签名和调用路径；其余已授权 UI、配置和交付说明可继续。

验收围绕用户效果：同等装备/目标下的预期数值，适用武器/角色过滤，重复启用不叠加，禁用恢复自己的影响；未验证的组合不宣称兼容。

## 配置与存档

`info.ini` 是加载与工坊元数据，上传时可能重写，不放玩家运行配置。原生模板也不自带 BepInEx ConfigFile。先核对项目已有配置实现与游戏 MOD 路径来源，再决定自有 JSON 等存储方式。来源：[上游 README](https://github.com/xvrsl/duckov_modding/blob/main/README.md)。

区分全局偏好、当前存档进度、单个物品状态。存档进度需要当前存档标识、读写时机、数据格式/迁移和主客端归属；本技能尚无经目标版本核对的通用存档 API，不把 Application.persistentDataPath 下一个公共文件当成正确的每存档记录。新增自定义物品要另读 [自定义物品与存档](gameplay.md#自定义物品与存档)。

## 图片、模型与 Bundle

已有 PNG 可用于需求明确的贴图/UI，不能直接当成已配置的游戏物品 prefab。已有 Bundle 的步骤：确认目标平台/引擎与资源依赖 → 从实际安装目录构造路径 → 加载 Bundle 并检查 null → 按真实名称和类型加载资源 → 在正确场景创建实例 → 按所有权释放。读取资源列表有助于定位名称，但不能证明 prefab 的组件、Shader 与游戏注册完整。

包外依赖与 Bundle 自带依赖分开处理，显式加入 ResourceManifest 并检查真实 package 文件。只有 `.blend`、图片或模型需求时，先说明还需要哪些导出资产与可用工具；不生成空 Bundle 占位。加载/卸载语义和地图需求见 [UI 与资源](ui-assets.md)。
