# Content Patcher 实现配方

适用：桌面 1.6、CP 2.x。2026-11-06 对照作者文档编写；示例 Format 为 2.9.0，玩家需有相应版本。完整 manifest 和构建参数见 [开发与交付](development.md)。下面每个完整 content.json 都是独立配方，按需求选用或合并 Changes，不交付全部示例。

## 先分清三种名字

- `Target` 是游戏内容管线资产名，例如 `Data/Objects`，不是磁盘上的 Content 路径，不带 `.xnb`。
- `FromFile` 是包内真实文件，例如 `assets/portrait.png`，需随包交付。
- 数据键是目标资产中的条目 ID；可显示名称和翻译文本不能代替 ID。

`Load` 提供/替换整个资产；只修改其中一部分时用 `EditData`、`EditImage` 或 `EditMap`，减少覆盖其他包的内容。一个包可有多个补丁，给每项写可定位的 `LogName`。来源：[作者指南](https://github.com/Pathoschild/StardewMods/blob/develop/ContentPatcher/docs/author-guide.md)。

## 修改已有数据：优先 Fields

下面只改变已存在对象 `388` 的基础 Price；ID 和目标条目在玩家数据中仍须核对。它不是所有商店的最终成交价，也不改变购买价格规则。

```json
{
  "Format": "2.9.0",
  "Changes": [
    {
      "LogName": "Change one object's base price",
      "Action": "EditData",
      "Target": "Data/Objects",
      "Fields": {
        "388": { "Price": 5 }
      }
    }
  ]
}
```

`Fields` 修改已有条目的个别字段；`Entries` 添加或替换整条，值为 null 时删除。对字典、列表、模型以及仍使用 `/` 分隔字符串的资产，编辑方式不同；先查该资产数据格式。不要把 1.5 的斜杠格式写进 1.6 的对象模型。嵌套列表先确定 `TargetField` 和条目标识，避免替换整个列表。来源：[EditData](https://github.com/Pathoschild/StardewMods/blob/develop/ContentPatcher/docs/author-guide/action-editdata.md)。

完成条件：补丁命中实际条目；导出结果只有目标字段变化；游戏中触发真正读取该字段的行为。若已有实例缓存旧值，重新进入相应场景或按该功能的实际刷新机制复核。

## 对话与玩家开关

此例修改 Abigail 的春季周一默认对话；特定节日、关系、事件等可能使用别的对话键，不能要求所有对话场景都显示这一句。

对话键、优先顺序和文本指令查 [Dialogue](https://stardewvalleywiki.com/Modding:Dialogue)，不要从画面上的一句文本反推唯一的目标键。

```json
{
  "Format": "2.9.0",
  "ConfigSchema": {
    "EnableGreeting": { "AllowValues": "true, false", "Default": "true" }
  },
  "Changes": [
    {
      "LogName": "Spring Monday greeting",
      "Action": "EditData",
      "Target": "Characters/Dialogue/Abigail",
      "Entries": { "spring_Mon": "{{i18n:greeting}}" },
      "When": { "EnableGreeting": "true" }
    }
  ]
}
```

包内 `i18n/default.json`：

```json
{ "greeting": "Hi! A new week is starting." }
```

可另交付 `i18n/zh.json` 使用同一键。`config.json` 由 CP 为玩家生成，续改时保留玩家配置。默认配置模式放在 ConfigSchema，翻译放在 i18n。来源：[配置](https://github.com/Pathoschild/StardewMods/blob/develop/ContentPatcher/docs/author-guide/config.md)、[作者指南的翻译章节](https://github.com/Pathoschild/StardewMods/blob/develop/ContentPatcher/docs/author-guide.md#translations)。

`When` 的键不加外层双花括号；多个条件全部满足，同一条件的多个候选值通常任一匹配即可。token 当前不可用会使依赖它的补丁暂不启用；标题界面没有存档相关 token 不等于格式错误。季节、天气、关系等条件须查对应 token 的取值和可用时机；CP token 与游戏的 Game State Query 是不同语法。来源：[tokens](https://github.com/Pathoschild/StardewMods/blob/develop/ContentPatcher/docs/author-guide/tokens.md)。

完成条件：开关启用时，在确实选择该对话键的场景看到译文；关闭后不再覆盖该键；不要用“包已加载”代替此结果。

## 局部图片替换

假设自有 `assets/portrait.png` 是与目标表情区域匹配的 64×64 PNG，以下替换 Abigail 肖像表左上角区域。64×64 仅用于此例，不能套给人物行走图、物品图或其他资产。

```json
{
  "Format": "2.9.0",
  "Changes": [
    {
      "LogName": "Replace one portrait region",
      "Action": "EditImage",
      "Target": "Portraits/Abigail",
      "FromFile": "assets/portrait.png",
      "FromArea": { "X": 0, "Y": 0, "Width": 64, "Height": 64 },
      "ToArea": { "X": 0, "Y": 0, "Width": 64, "Height": 64 },
      "PatchMode": "Replace"
    }
  ]
}
```

区域单位为像素。Replace 会用源透明像素覆盖目标；Overlay 让目标从透明部分透出。先核对目标图集布局、表情/动画索引和源图实际尺寸，再决定区域与模式；坐标正确不代表角色当前使用这个表情。来源：[EditImage](https://github.com/Pathoschild/StardewMods/blob/develop/ContentPatcher/docs/author-guide/action-editimage.md)。

完成条件：打开实际使用该区域的画面，检查透明、边界、其他表情与动画未被误改。只有内容文件、没有真实图片时，此功能尚未完成。

## 资源和拆分文件

以上对话例的 ResourceManifest 为：

```json
[{ "source": "i18n/default.json", "target": "i18n/default.json" }]
```

图片例另列 `assets/portrait.png`。`Include` 拆分的补丁 JSON、地图及其自有 tilesheet 也要逐个列入；脚本不会沿 FromFile 自动搜集。清单路径相对 ModDirectory，包内路径应与引用一致。不要在清单中重复固定交付的 manifest.json/content.json/README.md。

交付前从每个 FromFile、Include、翻译和自有纹理引用追踪到真实文件与交付映射；有动态路径时覆盖可达取值。补丁冲突先看 [排错](troubleshooting.md)，不要靠改文件夹名或提高所有补丁优先级处理。
