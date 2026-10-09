# 编译依赖

需要 .NET SDK（提供 dotnet build 与 netstandard2.1 编译能力）以及用户独立提供的 Duckov/Unity 参考 DLL 目录。目录可以来自用户合法获取的对应游戏版本资料，不要求桌面端找到游戏安装位置，也不下载或附带游戏 DLL。

模板需要包含 Duckov.Modding.ModBehaviour 的游戏程序集，以及 UnityEngine.CoreModule.dll、UnityEngine.IMGUIModule.dll。官方示例使用 TeamSoda.* 引用组，未据此猜测单个游戏 DLL 文件名；prepare.ps1 检查 TeamSoda.*.dll 引用组与两项 Unity DLL，实际类型是否完整由编译确认。复杂功能按源码补充引用，缺失 API 不用空 DLL 冒充。工程从 ReferenceDirectory 引用 DLL，Private=false；只交付当前 MOD DLL，不打包游戏运行库。

首次 SDK 构建可能需要还原 netstandard 目标资源；网络不可用或 SDK 缺失应准确保留错误与源码。缺依赖只阻塞此 MOD 的编译验收，不阻塞通用工具开发。参考 DLL 必须与用户游戏版本相匹配，用户自行验证运行效果。
