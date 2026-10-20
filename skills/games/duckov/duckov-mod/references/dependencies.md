# 编译依赖

需要 .NET SDK（提供 dotnet build 与 netstandard2.1 编译能力）以及用户独立提供的 Duckov/Unity 参考 DLL 目录。目录可以来自用户合法获取的对应游戏版本资料，不要求桌面端找到游戏安装位置，也不下载或附带游戏 DLL。

模板需要包含 Duckov.Modding.ModBehaviour 的游戏程序集，以及 UnityEngine.CoreModule.dll、UnityEngine.IMGUIModule.dll。官方示例使用 TeamSoda.* 引用组，未据此猜测单个游戏 DLL 文件名；prepare.ps1 检查 TeamSoda.*.dll 引用组与两项 Unity DLL，实际类型是否完整由编译确认。复杂功能按源码补充引用，缺失 API 不用空 DLL 冒充。工程从 ReferenceDirectory 引用 DLL，Private=false；只交付当前 MOD DLL，不打包游戏运行库。

首次 SDK 构建可能需要还原 netstandard 目标资源；网络不可用或 SDK 缺失应准确保留错误与源码。缺依赖只阻塞此 MOD 的编译验收，不阻塞通用工具开发。参考 DLL 必须与用户游戏版本相匹配，用户自行验证运行效果。

## 按功能核对引用

模板以通配符引用 ReferenceDirectory 下的 DLL，Private=false 并不会让这些依赖在用户游戏中自动可用。目录内应保持同一游戏版本，不能混入另一版本的同名游戏或 Unity DLL。下面是定位线索，具体文件以类型定义所在程序集及其依赖为准：

| 功能 | 需要查证的类型或模块 |
| --- | --- |
| 物品与背包 | ItemStatsSystem.dll、ItemAssetsCollection、ItemUtilities；工具类的实际程序集另查 |
| uGUI 与文字 | UnityEngine.UI、TMPro 的定义程序集，以及 Duckov UI 所引用类型 |
| 游戏异步返回值 | Cysharp.Threading.Tasks.UniTask 所在程序集及版本 |
| AssetBundle | UnityEngine.AssetBundleModule.dll，以及实际资源使用的引擎模块 |
| Harmony | 用户确认的 0Harmony.dll 或前置提供的等价运行库；核对实际程序集身份与版本 |

编译期依赖来自参考目录；运行期依赖来自游戏或明确的前置 Mod。新增前置应说明来源、版本、加载方式与分发权限，确认后再引入。不要把缺失的游戏 API 问题转化成安装最新 NuGet 包，也不要创建假的同名程序集骗过编译。

## 构建与交付命令

交付可安装版本时，通过 `run_command` 顺序调用本能力原始 `prepare.ps1`、`build.ps1`、`package.ps1`。必需编译记录必须来自原始 `build.ps1` 绝对路径。cwd 为当前 source；路径分别传参：

```text
program: powershell.exe
args: ["-NoProfile", "-NonInteractive", "-ExecutionPolicy", "Bypass", "-File",
       "<skill>/scripts/prepare.ps1", "-ReferenceDirectory", "<referenceDirectory>"]

program: powershell.exe
args: ["-NoProfile", "-NonInteractive", "-ExecutionPolicy", "Bypass", "-File",
       "<skill>/scripts/build.ps1", "-ProjectPath", "<source>/ExampleMod.csproj",
       "-ReferenceDirectory", "<referenceDirectory>",
       "-OutputDirectory", "<source>/.delivery/<taskId>/build"]

program: powershell.exe
args: ["-NoProfile", "-NonInteractive", "-ExecutionPolicy", "Bypass", "-File",
       "<skill>/scripts/package.ps1", "-ModName", "<ModName>",
       "-BuildDirectory", "<source>/.delivery/<taskId>/build",
       "-SourceDirectory", "<source>", "-PackageDirectory", "<source>/.delivery/<taskId>/package"]
```

输出目录必须全新。更新工程 README 后收集实际 package 文件，调用 `prepare_delivery`，target 为 `<ModName>/<文件名>`，targetDirectoryHint 为 `Duckov_Data/Mods`，commandIds 包含本任务成功 build/package 的 ID。通用字段与证据规则见 mod-workflow 能力的 references/delivery.md。

## 现有脚本的能力边界

- prepare.ps1 只检查最小文件组与 dotnet 命令存在，不能证明 SDK 版本、全部类型或功能所需引用完整。
- build.ps1 调用原始 prepare.ps1 后执行 dotnet build；build 内部固定调用 powershell.exe，按当前脚本实际行为记录错误，不另造包装命令冒充必需脚本记录。
- package.ps1 只复制 `<ModName>.dll`、info.ini、README.md、preview.png；没有 preview.png 时生成中性预览图。不会自动复制其他 DLL、AssetBundle、配置或子目录；需要额外文件时使用可选 ResourceManifest 显式列出，格式与约束见 [资源交付](ui-assets.md#显式图片资源清单)。
- 不附带游戏、Unity、UniTask 参考程序集。第三方 Mod 运行库是否由前置提供须单独确认，不能一律按游戏 DLL 处理。
- 若需求需要额外交付文件，先核对项目当前交付工具、安装映射与允许文件集。扩展脚本或依赖范围未获授权时，保留源码并明确交付缺项，不登记看似完整但不可用的包。
