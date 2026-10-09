---
name: duckov-mod
description: 为逃离鸭科夫开发 C# MOD、准备参考程序集并编译交付
---

# 逃离鸭科夫 MOD

先读取[来源与入口](references/api-sources.md)和[编译依赖](references/dependencies.md)，再按用户需求修改 C# 源码。不要编造游戏成员；新增游戏 API 必须从实际参考程序集或已核实资料确认。默认模板只在屏幕角落显示 MOD 信息，不改变游戏规则。模板为本工具自行编写，不复制示例的物品 UI 逻辑。

将 [C# 工程](assets/template/ExampleMod.csproj)、[入口](assets/template/ModBehaviour.cs)、[元数据](assets/template/info.ini) 和 [说明](assets/template/README.md) 复制到当前源码目录。选择合法 C# 标识符作为 ModName，统一 AssemblyName、namespace、info.ini name 和 DLL 文件名；更名时同步修改这四处。

用户提供独立参考程序集目录 ReferenceDirectory；不需要本机安装游戏。依次使用 `run_command` 调用本 skill 的 [prepare.ps1](scripts/prepare.ps1)、[build.ps1](scripts/build.ps1)、[package.ps1](scripts/package.ps1)。必需编译命令必须调用此 skill 原始 build.ps1 绝对路径，不以“存在 DLL”代替成功编译。OutputDirectory 使用 `.delivery/<当前 taskId>/build`，PackageDirectory 使用 `.delivery/<当前 taskId>/package`；本次目录不得预先存在，确保不用旧 DLL。

脚本参数示例：`powershell.exe`，参数数组包含 `-NoProfile`, `-NonInteractive`, `-ExecutionPolicy`, `Bypass`, `-File`, `<skill绝对路径>/scripts/build.ps1`, `-ProjectPath`, `<source绝对路径>/ExampleMod.csproj`, `-ReferenceDirectory`, `<用户提供目录>`, `-OutputDirectory`, `<source绝对路径>/.delivery/<taskId>/build`。路径均作为独立参数，不拼入 shell 文本。

缺少 SDK 或参考程序集时，保存源码并说明具体缺项，停止编译交付，不伪造 DLL。成功后收集 package 下当前 MOD DLL、info.ini、preview.png、README.md，调用通用 prepare_delivery，target 统一 `<ModName>/<文件名>`，targetDirectoryHint 提示 `Duckov_Data/Mods`，commandIds 包含本次成功 build/package 的 ID。安装按钮与目标选择由用户操作；游戏内测试由用户负责，不能声称已验证效果。
