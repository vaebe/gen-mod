# 游戏数据、ID 与内容依赖

适用桌面 Stardew Valley 1.6。2026-11-06 对照所列资料整理；具体字段使用目标版本文档或当前 StardewValley.GameData.dll，已有游戏数据用导出结果确认。下面的入口可定位功能，但未覆盖游戏的全部数据结构。

## 物品：定义、图片、获得方式分开实现

`ItemId` 是不带类型前缀的字符串，如 `388`；`QualifiedItemId` 带类型，如 `(O)388`。两者都不是显示名，也不能用整数覆盖所有自定义 ID。普通物品 `(O)`、大工艺品 `(BC)`、武器 `(W)` 等属于不同类型。同一基础 ID 不应脱离类型判定对象。

1. 用稳定的作者/ModID 前缀命名新增条目，续版保持 ID 不变。
2. 在对应资产添加定义，例如普通物品在 `Data/Objects`。给出真实名称、描述、分类和所需字段。
3. 提供纹理资产与 SpriteIndex，确认图格尺寸、行列和索引；纹理路径不是本地 PNG 文件路径，需通过 Load 提供对应内容资产。
4. 实现需求中的获得途径：商店、配方、掉落或奖励。注册定义本身不会发给玩家。
5. 说明卸载后的物品和存档影响，并要求玩家在相关变更前备份；不承诺自动恢复缺失内容。

C# 用 `ItemRegistry.Create(qualifiedId)` 创建实例；先用 Exists 核对自定义条目或处理 allowNull 返回，避免把默认 Error Item 当作真实物品。直接构造某类物品时参数可能要求不带前缀的 ItemId，按构造签名核对。来源：[Items](https://stardewvalleywiki.com/Modding:Items)。

`Data/Objects` 在 1.6 是字符串键到 ObjectData 模型，Name 是内部名，DisplayName/Description 是显示字段；Texture/SpriteIndex 决定图片。修改售价时分清定义中的基础 Price 与商店商品 Price、品质和职业修正。来源：[Objects](https://stardewvalleywiki.com/Modding:Objects)。

## 商店：操作商品列表，不覆盖整个商店

下面是独立 content.json：向 FishShop 的 Items 列表加入一个自有商品行，出售原版物品 `(O)388`。价格仅用于演示；按实际需求替换。

```json
{
  "Format": "2.9.0",
  "Changes": [
    {
      "LogName": "Add one shop offer",
      "Action": "EditData",
      "Target": "Data/Shops",
      "TargetField": [ "FishShop", "Items" ],
      "Entries": {
        "{{ModId}}_WoodOffer": {
          "Id": "{{ModId}}_WoodOffer",
          "ItemId": "(O)388",
          "Price": 10
        }
      }
    }
  ]
}
```

商品行 Id 是列表身份，ItemId 是出售物品，二者用途不同。重复销售同一物品也应有稳定的行 Id。限购、库存刷新和条件需要查对应字段，不用本地变量假装全服限购。验收真实开店入口、所需日期/条件、购买结果、价格和库存；只导出 Data/Shops 不代表能买到。来源：[Shops](https://stardewvalleywiki.com/Modding:Shops)。

## 作物：从种子到收获的闭环

从 `Data/Crops` 找到种子键与作物模型，核对 Seasons、DaysInPhase、HarvestItemId、RegrowDays、NeedsWatering，以及纹理和索引。新增作物还要有对应种子和产出物定义、图像、种子获得方式；成熟天数、再生天数和可种季节是不同设置。

完成条件：能实际取得种子并播种，经过约定生长阶段后收获正确物品；按本需求检查跨季、再生或温室情形。仅加入收获物或种子商品不等于新作物完成。来源：[Crop data](https://stardewvalleywiki.com/Modding:Crop_data)。

## NPC：角色数据与实际出场

1.6 以 `Data/Characters` 为主数据入口。按该模型核对身份、显示、生日、社交和生成条件；不要套用旧教程的 Data/NPCDispositions 格式。新增角色按功能配齐 `Characters/<ID>` 行走图、`Portraits/<ID>` 肖像、`Characters/Dialogue/<ID>`、`Characters/schedules/<ID>` 及必要的礼物偏好和事件。

先做一个“能在确定地点出现并交谈”的最小闭环，再扩展日程、婚恋和节日；每项额外功能都有自己的数据与资源。角色 ID、地图名、日程地点和坐标要相互对应；只声明角色或只替换肖像不能称为新增 NPC。完成条件包括当天日程出场、跨地点移动、对话、保存重读；其他功能按实际范围单列。来源：[NPC data](https://stardewvalleywiki.com/Modding:NPC_data)。

## 地图：地图文件以外还有入口和通行

地图需求先确认替换现有区域还是新建地点。地图资产、地点实例、出入口、碰撞与地图属性是不同部分。采用 CP 的地图动作或 CustomLocations 时查对应版本字段；TMX 引用的自有 tilesheet 按地图相对路径交付。

先核对可用编辑器与地图资源，不把空 JSON 或 PNG 背景当成可走入的新地点。完成条件：入口能到达、出口能返回、碰撞正确、必要交互可用；涉及季节 tilesheet、NPC 路径和保存位置时分别检查。来源：[Maps](https://stardewvalleywiki.com/Modding:Maps)、[CP 作者指南](https://github.com/Pathoschild/StardewMods/blob/develop/ContentPatcher/docs/author-guide.md)。

## 新数据与 API 的查证记录

本次用到的新字段或方法至少能回答：来自哪个目标资产/程序集或版本文档、条目是否存在、值类型与单位、何时被读取、修改定义还是当前实例、是否写存档/同步。网页与 DLL 不一致时以玩家目标版本的真实契约为准，说明兼容边界；没有资料时列出具体缺项，不猜测模型属性。
