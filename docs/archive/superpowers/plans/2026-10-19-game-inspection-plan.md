> 历史档案：保留原日期、计划与验证限制，不代表当前实现或本轮验收。当前入口见[文档首页](../../../README.md)。

# 支持游戏调查分项执行计划

> **面向 AI 代理的执行者：** 使用 executing-plans，遵循[总计划](2026-10-19-gen-mod-improvement-plan.md)。只调查支持列表内的游戏。

**目标：** 在玩家选择的目录中只读识别环境，减少手工填写，不自动适配或安装。

**架构：** 游戏 skill 提供受限规则，原生通用解释器执行；报告独立保存，玩家采纳后才更新项目上下文。

**技术栈：** Rust std/serde/sha2、现有目录选择与 SQLite；无 Python、全盘发现器或联网研究服务。

**规格：** [整体设计第 7.5–7.6 节](../specs/2026-10-19-gen-mod-improvement-design.md#76-已支持游戏调查)。依赖 P2。

**执行状态：** 已按完整范围实现并完成限定检查；此文保留原始任务合同。逐项结果、实现调整及未运行的应用/游戏验收见 [执行记录](2026-10-19-execution-progress.md)。

## 文件与数据

- 创建：`src/contracts/inspection.ts`、`src-tauri/src/game_inspection/{mod.rs,models.rs,scanner.rs,repository.rs,tests.rs}`、`src-tauri/src/storage/migrations/007_inspection.sql`、`skills/games/duckov/duckov-mod/references/inspection.json`。
- 创建：`runtime/extensions/inspection.ts`，只提供读取已有报告的模型工具。
- 修改：`lib.rs`、`storage/database.rs`、`skills/models.rs`、`skills/registry.rs`、`skills/registry.json`、`pi/bridge.rs`、`runtime/extensions/mod-tools.ts`、`src/contracts/skill.ts`、`src/features/mods/ProjectContextDialog.vue`、`src/services/native.ts`。
- 游戏能力元数据增加可选 `inspectionFile`，文件位于其资源目录且参与原有哈希。未声明的导入能力显示“没有调查规则”，不回退到猜测引擎。

```typescript
export interface InspectionRule {
  schemaVersion: 1
  profiles: { kind: 'install' | 'references', paths: { relative: string, fact: 'engineMarker' | 'referenceAssembly' | 'loaderMarker' }[] }[]
}
export interface InspectionReport {
  reportId: string, modId: string, gameId: string
  kind: 'install' | 'references', inspectedAt: string
  findings: { relativePath: string, fact: string, sha256?: string }[]
  unknowns: string[]
  truncated: boolean
  errors: { relativePath: string, code: 'permission' | 'format' | 'limit' }[]
}
```

## 任务 1：规则与有限读取

- [ ] scanner 仅解释相对路径和文件名末段的 `*` 匹配，禁止 `..`、绝对路径、脚本、任意正则和规则中的命令。目录、文件与每层父目录拒绝重解析点。
- [ ] 固定上限：目录深度 4、枚举项 5000、单文件哈希 64 MiB、总哈希 256 MiB；超出时停止该部分并记录 limit/truncated，不返回“未发现”冒充完整调查。
- [ ] Duckov install 规则仅检查已知 UnityPlayer.dll 与 Duckov_Data/Managed 的必要文件；references 规则检查现有 prepare 要求的 UnityEngine.CoreModule.dll、IMGUIModule.dll、TeamSoda.*.dll。文件存在只证明标记，不证明游戏 API 完整或运行可用。
- [ ] 没有可靠游戏构建标识时 unknowns 保留 gameVersion；不以程序集文件版本推导游戏版本，不扫描存档或注册表。

## 任务 2：仓储与采纳

- [ ] 建 inspection_reports(report_id 主键、mod_id、payload)。`inspect_supported_game(modId, kind, directory)` 只能使用玩家目录选择的结果，并核对项目所选 skill 的有效规则。
- [ ] 扫描放后台阻塞线程，文件 IO 不持 SQLite 事务或全局项目存储锁；报告完成后用短事务保存。
- [ ] `apply_inspection(modId,reportId,expectedContextRevision)` 验证报告归属；明确选择的字段以 source=inspection/sourceId 写 P2。环境修改不影响已开始任务快照。
- [ ] AI 如需读取报告，通过 P2 已采纳的上下文或单独只读 `read_game_inspection` 工具；该工具无目录参数，不能由模型发起新目录扫描。加入原生分派与任务工具白名单。

## 任务 3：静态夹具行为测试

- [ ] 在 scanner tests 以 tempfile 创建安装与参考两种目录夹具，获准后运行红绿循环：

```rust
#[test]
fn assembly_markers_do_not_claim_game_version() {
    let root = reference_fixture(); // 临时目录只含规则要求的三个占位二进制文件，不含真实游戏 DLL
    let report = scan_directory(root.path(), &duckov_reference_rules()).unwrap();
    assert!(!report.findings.is_empty());
    assert!(report.unknowns.iter().any(|value| value == "gameVersion"));
}
```

`scan_directory(root,rules)` 返回扫描部分结果；fixture 只用于文件标记路径测试，不用于伪造可编译程序集。另测权限错误、路径越界、数量截断与规则缺失。

## 任务 4：玩家展示与交付

- [ ] ProjectContextDialog 提供“检查所选目录”，展示命中文件与未知项，玩家可采纳；不显示概率百分比、不将残缺报告改成通过。
- [ ] 未支持游戏仍拒绝创建，缺安装目录仍可选择独立参考目录。规则JSON纳入技能静态校验、内容哈希和资源同步。
- [ ] 获准后运行 `cargo test --lib game_inspection:: -- --nocapture`；限定 Lint、CodeGraph sync/status。真实游戏目录读取需独立授权，不从临时文件结果推导真实识别率。
