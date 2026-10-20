# 生命周期、UI 与资源

## Mod 生命周期

Duckov 加载器通过 AddComponent(Type) 创建 ModBehaviour。初始化自身状态与等待游戏角色、UI 准备好是两个阶段，不能假设 Awake 时已经进入游戏场景。

- OnEnable 注册事件，OnDisable 配对退订；需要禁用即停止的任务使用与启用周期匹配的取消机制。
- OnDestroy 释放自己创建的文本、GameObject、资源句柄。销毁和禁用路径可能连续发生，清理应避免重复副作用。
- 进入关卡、返回基地或重建 UI 后，原引用可能失效；重新确认对象，避免静态字段持有已销毁对象。
- 不在 Update/OnGUI 每帧扫描全场景、创建文本或加载 Bundle；按实际事件更新，必要刷新频率按需求设定。
- Unity 对象操作留在主线程，不用 Task.Run 执行游戏生成、Transform 或 UI 操作。异步返回后检查对象和任务是否仍有效。

生命周期参考：[Unity OnDisable](https://docs.unity3d.com/ScriptReference/MonoBehaviour.OnDisable.html)。异步生命周期参考：[UniTask](https://github.com/Cysharp/UniTask)；销毁取消并不覆盖“仅禁用”的情况，所用 API 须适配游戏版本。

## 物品悬浮 UI 的真实示例

[DisplayItemValue](https://github.com/xvrsl/duckov_modding/blob/main/DisplayItemValue/ModBehaviour.cs) 展示了以下结构，当前程序集仍需核对：

- Duckov.UI.ItemHoveringUI.onSetupItem 与 onSetupMeta 在启用时订阅、禁用时退订。
- 处理实际 Item 时展示文字，处理元数据或空物品时隐藏，避免上一次内容残留。
- 从 Duckov.Utilities.GameplayDataSettings.UIStyle.TemplateTextUGUI 创建 TextMeshProUGUI，并放到 uiInstance.LayoutParent。
- OnDestroy 销毁自己创建的文本。示例用 item.GetTotalRawValue() 取得数值，但显示公式不是已确认的商店定价契约。

按需求复用这种事件方式，不复制整个示例。不同 UI 状态使用不同数据入口，不能把只有元数据的预览当作已有 Item 实例。

OnGUI 模板适合最小加载提示；正式交互按需求核对 uGUI/TMP、锚点、缩放、字体和输入遮挡。新增 UI 不应吞掉无关游戏输入。

## 小地图与场景类需求

当前知识库没有已核对的小地图专属游戏 API。实现时依次查证：场景/区域边界、玩家定位、世界坐标到平面坐标转换、地图纹理或渲染来源、可显示目标、场景切换生命周期。

区分“显示玩家标记”“显示已探索地图”“显示敌人或战利品”，这些需要不同数据与业务授权。不能用一个角落方框或固定背景声称实现地图；没有场景数据或资源时明确缺项。

不要硬编码未经验证的场景尺寸、坐标轴、玩家单例或敌人列表。若选择额外相机渲染方案，核对 Layer/CullingMask、RenderTexture 释放和渲染成本；这属于可选方案，不能默认所有地图都支持。

## 模型与 AssetBundle

仅 C# DLL 通常不能替代需求中的模型、贴图、prefab 或动画。先确认所需资源、授权、游戏 Unity 版本、目标平台和 Shader/渲染管线；不能直接选择最新版 Unity 导出。

已有 Bundle 时核对资源名与依赖，再用对应引擎版本的加载 API。AssetBundle.LoadFromFile 加载失败可以返回 null；后续加载具体资源前须确认 Bundle 和资源存在。Bundle.Unload(false) 保留已加载对象，Unload(true) 会销毁已加载对象，按资源所有权安排释放，不破坏仍在使用的物品。

资源路径相对该 Mod 实际安装目录解析，不从 source、工作目录或开发机绝对路径读取。获取 Mod 目录的游戏成员尚未在本技能核对，应查当前 ModBehaviour 或加载器实现，不编造路径属性。

来源：[Unity LoadFromFile](https://docs.unity3d.com/ScriptReference/AssetBundle.LoadFromFile.html)、[Unity Unload](https://docs.unity3d.com/ScriptReference/AssetBundle.Unload.html)。使用时切换到游戏对应版本文档。

模型框架任务读取 [Duckov Custom Model 文档](https://github.com/Duckov-Custom-Model/DuckovCustomModel-Docs)；它的 Mod 加载与资源约定需单独遵循，不替代本工具交付协议。

## 资源交付

当前 package.ps1 默认不自动收集资源或额外依赖。需求需要额外文件时，通过 ResourceManifest 明确文件和 target 映射，说明文件放在哪个 Mod 子目录。未经实际打包与登记确认，不宣称已包含资源。

更新实际 Mod 的 README：说明功能、前置、资源清单、操作方式与未验证内容；不要保留“仅在角落显示信息”作为已实现业务的说明。预览图或模板存在不能替代实际资源。

## 显式图片资源清单

package.ps1 已支持可选 `-ResourceManifest`。清单保存在本任务 `.delivery/<taskId>/resources.json`，内容为 JSON 数组，例如：

```json
[{"source":"assets/player-icon.png","target":"images/icon.png"}]
```

source 相对工程 SourceDirectory，target 相对当前 MOD 的安装子目录，使用 `/` 分隔。脚本在固定四类文件之外只复制明确列出的普通文件，拒绝越界、重解析点、重复目标和覆盖固定文件。未传清单时保持原行为，不扫描目录收集资源。

玩家从“全部文件”原样导入文件，按普通相对路径使用，不做裁剪、缩放或 JPEG 转换。加入源码、实现加载、构建、package、prepare_delivery 是不同阶段。交付映射覆盖真实 package 文件，包括额外资源；参考程序集和前置运行库的分发约束仍适用。

本能力尚未确认目标版本的 MOD 安装目录获取成员与运行时图片加载接口。实现游戏加载前按 API 查证取得对应版本依据；仅处理或打包图片时，如实说明尚未实现游戏中的加载和用途，不编造路径属性。
