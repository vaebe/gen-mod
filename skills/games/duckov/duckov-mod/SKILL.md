---
name: duckov-mod
description: 为逃离鸭科夫开发和排查 C# MOD，查证游戏 API，处理物品、角色、UI、资源与 Harmony，并编译交付
---

# 逃离鸭科夫 MOD

用于本工具中的 Duckov C# MOD 开发与排查。先读取[来源与入口](references/api-sources.md)和[编译依赖](references/dependencies.md)，再按需求读取下面的专题。资料核对日期为 2026-10-10；文档中的 API 是查找入口，不代表所有游戏版本均支持。

## 按任务读取

| 任务 | 参考资料 |
| --- | --- |
| 新 API、找方法、游戏更新后失效、参考 DLL 不完整 | [API 查证](references/api-verification.md) |
| 物品生成、背包、价格、角色、对话、本地化 | [游戏功能](references/gameplay.md) |
| 生命周期、物品 UI、小地图、模型、AssetBundle | [UI 与资源](references/ui-assets.md) |
| 修改原有方法、Harmony 前置、补丁兼容性 | [Harmony](references/harmony.md) |
| 无法加载、功能未生效、异常、用户验收 | [排查与验收](references/troubleshooting.md) |

不要一次读取全部专题。新增游戏 API 必须核对实际参考程序集或对应版本的可信源码；网上示例与当前程序集冲突时，以当前程序集签名为准，并查清行为差异。无法确认的成员保留为缺项，不用猜测、空实现或不断尝试反射名称冒充已实现。

## 实现入口

先检查当前源码目录中的工程和已有改动。已有 MOD 持续复用，不重复覆盖模板；只有新建工程才复制 [C# 工程](assets/template/ExampleMod.csproj)、[入口](assets/template/ModBehaviour.cs)、[元数据](assets/template/info.ini) 和 [说明](assets/template/README.md)。默认模板只显示 MOD 信息；实际任务须实现用户要求的功能，并更新交付说明，不把模板显示效果当作业务完成。

ModName 使用合法 C# 标识符，统一 AssemblyName、namespace、info.ini name 和 DLL 文件名；更名时同步修改这四处。入口为公开的 `<ModName>.ModBehaviour`，继承 `Duckov.Modding.ModBehaviour`，由游戏加载器创建，不自行创建第二个入口实例。

优先使用已有公开 API 和事件；只有无法满足需求时才评估 Harmony。安装或新增第三方前置需要用户明确授权。只修改当前需求涉及的行为；数值公式、物品去向、存档影响等业务含义不明确时先查明再确认。

## 编译与交付

用户提供独立参考程序集目录 ReferenceDirectory；不需要本机安装游戏。要求交付可安装版本时，依次使用 `run_command` 调用本 skill 的 [prepare.ps1](scripts/prepare.ps1)、[build.ps1](scripts/build.ps1)、[package.ps1](scripts/package.ps1)。只读讨论与排查不自动编译或改文件。必需编译命令必须调用此 skill 原始 build.ps1 绝对路径，不以“存在 DLL”代替成功编译。OutputDirectory 使用 `.delivery/<当前 taskId>/build`，PackageDirectory 使用 `.delivery/<当前 taskId>/package`；本次目录不得预先存在，确保不用旧 DLL。

脚本参数示例：`powershell.exe`，参数数组包含 `-NoProfile`, `-NonInteractive`, `-ExecutionPolicy`, `Bypass`, `-File`, `<skill绝对路径>/scripts/build.ps1`, `-ProjectPath`, `<source绝对路径>/ExampleMod.csproj`, `-ReferenceDirectory`, `<用户提供目录>`, `-OutputDirectory`, `<source绝对路径>/.delivery/<taskId>/build`。路径均作为独立参数，不拼入 shell 文本。

缺少 SDK 或参考程序集时，保存源码并说明具体缺项，停止编译交付，不伪造 DLL。成功后收集 package 下当前 MOD DLL、info.ini、preview.png、README.md，调用通用 prepare_delivery，target 统一 `<ModName>/<文件名>`，targetDirectoryHint 提示 `Duckov_Data/Mods`，commandIds 包含本次成功 build/package 的 ID。安装按钮与目标选择由用户操作；游戏内测试由用户负责，不能声称已验证效果。

当前 package.ps1 只整理上述四类文件，额外资源或第三方 DLL 不会自动进入交付包。需要 AssetBundle、配置或前置运行库时，先读取 UI 与资源、编译依赖中的交付约束，核对实际交付能力；不交付缺少必要文件的版本，也不擅自绕过原始构建脚本。
