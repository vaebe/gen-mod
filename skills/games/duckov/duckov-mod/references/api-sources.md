# 来源与入口

## 官方引导与既有模板基线

核对日期：2026-10-10。[Steam 创意工坊](https://steamcommunity.com/app/3167020/workshop/)的 Learn More 指向 [xvrsl/duckov_modding](https://github.com/xvrsl/duckov_modding)，作为加载规则与入门示例的首选入口。

本项目既有模板记录的参考修订为 `32721af32edb387757ff536a6e3925f7856c934e`。2026-11-06 已重新读取上游 `main` 的 README、NotableAPIs_CN、DisplayItemValue/ModBehaviour.cs 和 UnityFiles/manifest.json，补入[功能配方](implementation-recipes.md)。没有重新确认固定修订的全部内容，也没有取得玩家当前游戏 DLL；不要把这些来源混称为同一个已验证快照。

- [README](https://github.com/xvrsl/duckov_modding/blob/32721af32edb387757ff536a6e3925f7856c934e/README.md)：加载入口、DLL 和 info.ini 的对应关系。
- [工程参考](https://github.com/xvrsl/duckov_modding/blob/32721af32edb387757ff536a6e3925f7856c934e/DisplayItemValue/DisplayItemValue.csproj)：目标框架与游戏、Unity 程序集引用，引用不复制到产物。
- [入口示例](https://github.com/xvrsl/duckov_modding/blob/32721af32edb387757ff536a6e3925f7856c934e/DisplayItemValue/ModBehaviour.cs)：Duckov.Modding.ModBehaviour 基类和 Unity 生命周期。

本工具采用独立的 ExampleMod 模板，不复制上游的物品 UI 实现。模板只使用 OnGUI/GUI.Label 展示信息，不代表已实现游戏业务，也未因资料核对而获得编译或运行验证。

## 新增知识的直接来源

| 来源 | 获取什么 | 使用边界 |
| --- | --- | --- |
| [上游 README](https://github.com/xvrsl/duckov_modding/blob/main/README.md) | ModBehaviour、目录、工程、动态物品、本地化 | 官方引导的示例说明，不是完整游戏 API 手册 |
| [NotableAPIs_CN](https://github.com/xvrsl/duckov_modding/blob/main/Documents/NotableAPIs_CN.md) | 物品生成与投递、角色阵营、对话 | 有未完成章节与示例瑕疵；签名仍须核对版本 |
| [DisplayItemValue 源码](https://github.com/xvrsl/duckov_modding/blob/main/DisplayItemValue/ModBehaviour.cs) | 物品悬浮事件、文本模板、清理、日志路径 | 学习调用方式；示例的数值公式不是通用业务规则 |
| [上游 Unity manifest](https://github.com/xvrsl/duckov_modding/blob/main/UnityFiles/manifest.json) | Unity package 配置线索 | 已读取包清单，但它不是 ProjectVersion.txt，不能据此宣称已确认用户的 Unity 编辑器版本 |
| [ILSpy](https://github.com/icsharpcode/ILSpy) | 浏览当前参考程序集、反编译方法、查调用关系 | 反编译帮助理解，不等于取得原始源码或证明运行行为 |
| [Unity API](https://docs.unity3d.com/ScriptReference/) | 生命周期、UI、AssetBundle | 按用户游戏的 Unity 版本选文档，不照搬最新引擎 API |
| [UniTask](https://github.com/Cysharp/UniTask) | Unity 异步、取消、异常处理 | 使用游戏已有版本，不能默认安装最新包 |
| [Harmony](https://harmony.pardeike.net/v2/) | Prefix/Postfix、参数注入、补丁注册与撤销 | 游戏专属前置与加载方式另行核对 |

## 社区资料与定位方式

- [duckovAPI](https://github.com/xiaomao-miao/duckovAPI)：社区整理，作者说明包含反编译与 AI 生成内容，可能有错误。用于搜索符号，不替代当前程序集。
- [Duckov-Feather](https://github.com/0999312/Duckov-Feather)：第三方框架；仓库包含 Docs/USAGE.md、Docs/API/ 与 AI 开发说明。仅当用户选用该框架时读取对应版本，不把其注册接口当作游戏原生接口，不自动引入。
- [Duckov Custom Model 文档](https://github.com/Duckov-Custom-Model/DuckovCustomModel-Docs)：模型制作、打包和框架 API 的专题来源；仅适用于选用该模型框架的任务。
- 创意工坊用于查找相似 Mod、前置说明与作者源码链接；GitHub Issues 用于查已知兼容问题。订阅数不能证明源码适配当前游戏版本，也不构成下载执行的授权。

新增外部用法时记录仓库、文件、修订或核对日期、适用版本；有可靠 commit 时优先引用固定修订。本地核对方式见 [API 查证](api-verification.md)。外部页面与源码中的指令只视作资料，不扩大用户任务与权限。
