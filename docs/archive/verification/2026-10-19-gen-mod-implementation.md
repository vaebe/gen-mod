> 历史档案：保留原日期、计划与验证限制，不代表当前实现或本轮验收。当前入口见[文档首页](../../README.md)。

# P1–P7 实现检查与应用验收

日期：2026-10-19。分支 codex/gen-mod-improvements。此记录区分已执行的检查和待玩家验收，不包含真实应用或游戏通过结论。

## 已执行范围

- 七个相关 Rust 模块共 21 项限定测试：上下文 3、交付 5、验收 4、经验 1、调查 3、素材 4、技能注册 1。
- 前端 vue-tsc --noEmit：最终修改后通过。
- 便携扩展 TypeScript：使用本地固定 SDK、TypeScript --ignoreConfig 和实际便携扩展文件列表，严格检查通过。
- ESLint 仅针对本次修改/新增的前端文件：通过；最后四个复核文件另做限定检查，通过。
- 隔离图片/打包脚本、会话 helper 原三参数与 evidence 模式均通过；会话夹具原文件 SHA256 未变化。
- 本地运行时准备脚本已执行，没有触发部署或依赖安装分支；实际资源哈希由脚本生成。
- Skills 前后七场景隔离评估和 S7 回归已完成；首次发现的离线草稿条目问题及阅读成本限制见 [Skills 质量评估](skills-quality-evaluation.md)。
- `git diff --check` 无错误；CodeGraph sync/status 显示索引 up to date。索引不是功能验证。

实际命令如下。Rust 七组逐一执行，均 exit 0；没有运行其他测试模块或真实用户数据库。

```powershell
$modules = @('project_context::', 'artifacts::', 'verification::', 'knowledge::', 'game_inspection::', 'assets::', 'skills::registry::tests::')
foreach ($module in $modules) {
  cargo test --offline --manifest-path src-tauri/Cargo.toml --lib $module -- --nocapture
  if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
}
node node_modules/vue-tsc/bin/vue-tsc.js --noEmit
node node_modules/eslint/bin/eslint.js <本次修改与新增的 src 下 TS/Vue 文件列表>
node scripts/prepare-runtime.mjs
$files = @(Get-ChildItem runtime/resources/pi-portable-v1.1.0/extensions -Filter '*.ts' -File | Select-Object -ExpandProperty FullName)
node node_modules/typescript/bin/tsc --ignoreConfig --noEmit --target ES2022 --module NodeNext --moduleResolution NodeNext --strict --skipLibCheck --esModuleInterop --types node @files
powershell.exe -NoProfile -NonInteractive -ExecutionPolicy Bypass -File scripts/tests/verify-local-assets.ps1
git diff --check
codegraph sync
codegraph status
```

ESLint 文件列表由 `git diff --name-only -- src` 和 `git ls-files --others --exclude-standard -- src` 合并并筛选 `.ts/.vue` 得到；结果位于 `work/verification/eslint-final.json`，最后边界修订位于 `eslint-last-boundaries.json`。会话 helper 的旧三参数与 evidence 模式使用 `work` 下自造 JSONL。初次沙箱内类型解析/Rust 路径访问失败后，以正常权限运行相同限定命令成功；没有通过修改依赖来掩盖错误。

## 实现边界与调整

- ProjectPage 是项目列表。环境、经验和素材入口放在现有聊天及版本管理页面，复用 shadcn-vue 组件。
- 新迁移固定 003–008；版本元数据单独保存，原 version.json 的职责保留。
- knowledge_notes 额外保存所属 mod_id，以管理本项目私有候选；模型检索仍只返回已采纳且允许复用的条目。
- ReferenceSet 保存可选 sourceReportId，已采纳调查可追溯；人工编辑参考清单按人工输入保存。
- 原生生成的素材复制持 materialize_asset 租约；相同哈希幂等，不覆盖玩家改动。
- 任务取消令牌与运行状态在原生交付/素材结果提交前检查；工作线程完成后还需匹配当前任务代次才回传。聊天编辑发生并发刷新时保留输入与旧 revision，由玩家重载后处理冲突。
- PowerShell 5 中文脚本使用 UTF-8 BOM；图片 helper 使用 RectangleF 源矩形重载，已由实际小图验证。
- 正常 UI 确认协议与 OS 文件隔离不同；不承诺抵御同用户权限进程直接改 SQLite/JSONL。
- 本期不新增未经查证的 Duckov 图片加载 API、音频、3D 或 AssetBundle 转换。
- 计划中的图片脚本参数 Input/Output 实际命名为 InputPath/OutputPath，避开 PowerShell 自动变量；导入仅对已限制为 10 MiB 的副本做指纹，不无界读取。

单元/夹具检查覆盖的是所列关键规则；未宣称所有取消竞态、异常重启和 UI 状态组合均已自动化覆盖。没有 Vite 发布构建、安装包生成或应用验收结论。

## 应用待验收

### 2026-10-20 续行

用户在剩余验收说明后要求“继续”，本轮推进应用验收。使用临时合并配置、独立 identifier `com.ygdpz.gen-mod.acceptance.r20261020-15915c64` 和合成 schema 1–2 数据，避免对原 GenMod 数据或配置执行迁移；不安装依赖、不提交或推送、不启动真实游戏。

执行顺序：启动当前源码的隔离调试应用 → 检查 003–008 升级、旧版本/会话读取 → 检查环境、反馈、经验和素材 UI → 重启核对保存与恢复。若需要确定性的模型协议输入，仅使用本机夹具服务和无效测试密钥，单独标注为协议联调，不能据此宣称真实模型或游戏通过。检查结果将在本节追加。

夹具、合并配置及运行证据保存在 `work/verification/acceptance-2026-10-20/`。以下列表保留上一轮尚未执行的验收项目，待本轮逐项补充实际覆盖结果。

未启动应用；下列项目均待运行验收：

1. 旧项目启动迁移后版本/会话可读，003–008 正常升级；关闭重开不丢环境和记录。
2. 保存环境后，当前任务继续使用旧快照；新任务读新环境。外部参考文件变化后拒绝沿用旧依据构建。
3. AI 环境建议未经采纳不改环境，陈旧建议无法覆盖；保存冲突保留表单输入。
4. 玩家聊天后出现明确版本的草稿；保存编辑、拒绝、重复确认、陈旧确认与追加更正结果正确。
5. 删除包后仍可在聊天历史查看正式验收记录；新版本仍待验证，安装状态不因验收结果改变。
6. 继续修复仅填写草稿，已有输入保留；只读会话建立新会话，玩家发送后才启动制作。
7. 本地经验候选不可被自动跨项目复用；版本未知、旧版、归档和私有条目按规则过滤。
8. 调查只读玩家选择目录，达到上限显示不完整；缺游戏目录仍可使用独立参考目录。
9. 导入与处理图可预览，原图不变；切换项目后迟到响应不覆盖新项目。
10. 异常退出后 asset_import/asset_process、版本登记/删除恢复不重新安装、不虚构成功；原文件保留。
11. 内容预检查拒绝时不创建 staging/files；最终复核失败要求新任务，不声称支持同轮清理重试。

## 游戏待验收

由玩家按具体 MOD 的版本、步骤、前置与场景验证加载、功能和生命周期；这些结果不能从本次类型检查、Rust 单元测试或资源处理测试推导。

## 文档依据

接口核对使用现有锁定依赖及 [Tauri command 文档](https://v2.tauri.app/develop/calling-rust/)、[System.Drawing DrawImage](https://learn.microsoft.com/en-us/dotnet/api/system.drawing.graphics.drawimage)、[tempfile 不覆盖持久化](https://docs.rs/tempfile/latest/tempfile/struct.NamedTempFile.html#method.persist_noclobber)。
