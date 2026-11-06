# 开发、构建与交付

## 前置与工程

Content Patcher 内容包保留 `manifest.json`、`content.json`、玩家 README 和实际资源。`ContentPackFor.UniqueID` 必须为 `Pathoschild.ContentPatcher`，不要填写 EntryDll。Format 按采用的功能与玩家 CP 版本确定；模板的 2.9.0 仅是已核对作者指南的格式，不代表玩家已安装该版。

C# MOD 使用 `net6.0`，入口继承 `StardewModdingAPI.Mod` 并实现 `Entry(IModHelper)`。`EntryDll` 必须对应当前工程生成的 DLL。模板直接引用独立 `ReferenceDirectory`，不含 NuGet 包，也不会自动部署到游戏目录。基础参考集：`StardewModdingAPI.dll`、`Stardew Valley.dll`、`StardewValley.GameData.dll`、`MonoGame.Framework.dll`、`xTile.dll`、`SMAPI.Toolkit.CoreInterfaces.dll`（游戏安装目录中位于 `smapi-internal/`，独立参考目录可平铺）；新增 API 需要其他依赖时核对合法来源，不把参考 DLL 放进交付包。

可通过项目上下文“检查所选目录”选择游戏或独立参考目录并采纳已找到的程序集。文件存在只证明可读，不推断游戏版本、API 完整性或加载成功。

UniqueID 是稳定身份；变更显示名不改变 UniqueID。单人和多人功能分别确认触发端、同步和权限。操作世界时核对 `Context.IsWorldReady`，不能在标题界面访问玩家存档。配置、i18n 和必要图片通过 SMAPI/CP 的接口读取。

## 原始脚本参数

`run_command` 的 program 为 `powershell.exe`，args 以 `-NoProfile -NonInteractive -ExecutionPolicy Bypass -File` 开始，之后是当前加载 Skill 的原始 `scripts/build.ps1` 绝对路径。cwd 为 source 根目录。

共同参数：

- `-Kind`：`code` 或 `content-pack`。
- `-ModDirectory`：source 内工程/内容包目录。
- `-OutputDirectory`：`.delivery/<本任务taskId>/stardew-<唯一后缀>`，目录必须尚不存在。
- `-ResourceManifest`：可选，source 内严格 JSON 数组 `[{"source":"assets/example.png","target":"assets/example.png"}]`；source 相对 ModDirectory，target 相对最终 MOD 文件夹。i18n、图片、额外 JSON 和必要私有依赖逐文件列入。

C# 另传 `-ProjectPath`（ModDirectory 内 .csproj）与 `-ReferenceDirectory`（已确认参考目录）。内容包不传这两个参数，不运行 dotnet。资源列表不得包含 manifest.json、README.md、EntryDll、content.json 等本路线固定文件，不自动收集整个工程。

脚本校验 metadata、固定文件、JSON 和显式资源路径；C# 路线编译后只复制本次 EntryDll。空 CP Changes 会被拒绝。校验不会解释所有 CP 条件、token 或游戏数据，也不证明游戏效果。README 要写明实际功能、游戏/前置版本、安装方式、存档影响与逐项验收步骤，不保留模板占位文案。

## 版本登记与安装

输出中的 `package/` 是本次真实交付文件集。枚举每个文件交给 `prepare_delivery`：source 为 `.delivery/<taskId>/.../package/<relative>`，target 为 `<ModFolder>/<relative>`；附上本次原始脚本成功 commandId，targetDirectoryHint 写“星露谷物语安装目录下的 Mods 文件夹”。不要交付 bin、obj、游戏参考程序集或整个 .delivery。

安装页选择 Mods 文件夹，版本管理负责预览与覆盖。不要交付或启动 SMAPI 安装器，不直接改 Steam 启动选项，不替玩家启动游戏。README 说明通过 SMAPI 启动；CP 包另需先安装 Content Patcher。验收反馈需包含对应版本、触发步骤、观察现象和 SMAPI 错误日志；敏感路径和凭据不写入可复用知识。
