# 官方资料与版本依据

核对日期：2026-11-06。以下是查证入口，不表示当前用户已安装对应版本；开发前读取玩家目标版本及本机真实前置。

- [SMAPI Mod 基类](https://github.com/Pathoschild/SMAPI/blob/develop/src/SMAPI/Mod.cs)：核对 Entry 和生命周期。
- [IModHelper](https://github.com/Pathoschild/SMAPI/blob/develop/src/SMAPI/IModHelper.cs)：核对事件、命令、配置和内容接口。
- [SMAPI 工程](https://github.com/Pathoschild/SMAPI/blob/develop/src/SMAPI/SMAPI.csproj)：当前源码目标框架为 net6.0；游戏 API 与框架须与玩家版本一致。
- [官方构建引用](https://github.com/Pathoschild/SMAPI/blob/develop/src/SMAPI.ModBuildConfig/build/smapi.targets)：核对游戏程序集、SMAPI 及 smapi-internal 中 CoreInterfaces 的真实位置；本模板采用直接引用，不引入该 NuGet 包及其自动部署行为。
- [SMAPI 入门](https://stardewvalleywiki.com/Modding:Modder_Guide/Get_Started)：星露谷 Wiki 的开发入门，包含项目、入口和前置要求。
- [SMAPI manifest](https://stardewvalleywiki.com/Modding:Modder_Guide/APIs/Manifest)：身份、EntryDll、ContentPackFor、最低版本及依赖声明。
- [Content Patcher 作者指南](https://github.com/Pathoschild/StardewMods/blob/develop/ContentPatcher/docs/author-guide.md)：当前文档 Format 为 2.9.0；需核对玩家已安装 CP 版本再选择格式。
- [Content Patcher EditData](https://github.com/Pathoschild/StardewMods/blob/develop/ContentPatcher/docs/author-guide/action-editdata.md)：数据编辑规则。
- [Content Patcher EditImage](https://github.com/Pathoschild/StardewMods/blob/develop/ContentPatcher/docs/author-guide/action-editimage.md)：图片区域与布局。
- [SMAPI 下载与玩家入口](https://smapi.io/)：由玩家自行准备，不在构建时静默安装。

优先公开 API 和事件。只有真实需求需要且玩家已确认前置时才引入额外框架；不猜测数据 ID，不复制其他版本签名，不把文件存在视为运行兼容性验证。

## 按功能继续查证

下面的入口已于 2026-11-06 读取。GitHub develop 为移动分支，Wiki 也会更新；它们描述的最新结构不保证玩家旧版本支持。具体写法已整理进专题，无网络时先用专题，再准确报告新增资料缺项。

| 需求 | 本地知识 | 一手入口 |
| --- | --- | --- |
| CP 字段、图片、配置与条件 | [CP 配方](content-patcher.md) | [EditData](https://github.com/Pathoschild/StardewMods/blob/develop/ContentPatcher/docs/author-guide/action-editdata.md)、[EditImage](https://github.com/Pathoschild/StardewMods/blob/develop/ContentPatcher/docs/author-guide/action-editimage.md)、[config](https://github.com/Pathoschild/StardewMods/blob/develop/ContentPatcher/docs/author-guide/config.md)、[tokens](https://github.com/Pathoschild/StardewMods/blob/develop/ContentPatcher/docs/author-guide/tokens.md) |
| C# 生命周期、数据与内容 | [SMAPI 配方](smapi-cookbook.md) | [GameLoop](https://github.com/Pathoschild/SMAPI/blob/develop/src/SMAPI/Events/IGameLoopEvents.cs)、[IDataHelper](https://github.com/Pathoschild/SMAPI/blob/develop/src/SMAPI/IDataHelper.cs)、[AssetRequestedEventArgs](https://github.com/Pathoschild/SMAPI/blob/develop/src/SMAPI/Events/AssetRequestedEventArgs.cs) |
| 新物品、商店、作物、NPC、地图 | [数据与内容](game-data.md) | 该专题在相应章节提供各自数据格式来源，按当前功能读取 |
| 补丁不生效 | [排错](troubleshooting.md) | [CP troubleshooting](https://github.com/Pathoschild/StardewMods/blob/develop/ContentPatcher/docs/author-guide/troubleshooting.md) |
