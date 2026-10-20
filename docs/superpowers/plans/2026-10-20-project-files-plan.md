# 项目文件导入与移动实现计划

> **面向 AI 代理的工作者：** 使用 executing-plans，继承[总计划](2026-10-20-gen-mod-product-correction-plan.md)的授权、并发和验证边界。

**实施记录：** [本轮实现与验证](../../verification/2026-10-20-product-correction.md)。代码接入、纯夹具检查和实际应用验收分别记录，未勾选项不可视为已验收。

**目标：** 在现有“全部文件”原样导入任意普通文件，支持单文件移动、改名和创建目标目录。

**架构：** Rust 原生流式复制与不覆盖移动；沿用 Workspaces 的项目预留及 source 指纹。UI 复用现有文件树、预览和 shadcn-vue Dialog，不经过 assets 管线。

**技术栈：** 当前 Tauri dialog、tempfile、windows-sys、Vue；不新增 crate。Windows 不覆盖移动使用现有 windows-sys 的文件系统绑定，Cargo.toml 只追加 `Win32_Storage_FileSystem` feature，不改版本。

**规格：** [设计第 4 节](../specs/2026-10-20-gen-mod-product-correction-design.md#4-全部文件增加导入和移动)。

## 全局约束与文件职责

- 已确认范围：多文件原样复制、单文件移动/改名、创建目标目录；不覆盖、无内容处理、无整目录操作。
- 新增 `src-tauri/src/source_files/{mod.rs,io.rs,tests.rs}`：command 协调、纯文件操作、隔离测试。
- 修改 `src-tauri/src/{files.rs,lib.rs,changes.rs,pi/workspace.rs,pi/bridge.rs}`：选文件、注册、变更通知、预留和任务准备。
- 修改 `src-tauri/Cargo.toml`：仅上述 Windows API feature。
- 修改 `src/contracts/source.ts`、`src/features/workbench/{SourceBrowser.vue,SourceFileTree.vue,useSourceBrowser.ts}`。
- 新增 `src/features/workbench/{SourceDirectoryDialog.vue,useSourceFileActions.ts}`：项目内目标选择与操作状态。
- 新增 `src/features/workbench/useSourceFileActions.test.ts`：迟到响应与项目切换测试，mock nativeCall，不启动 Tauri。
- 修改 `vitest.config.ts`：在既有 pi 测试包含范围中追加本计划的确切测试文件，不扩大为全仓测试。

## T1：原生文件操作与项目预留

- [x] **步骤 1：定义合同。** 在 source.ts 和 Rust DTO 固定以下字段，不返回图片配方或 assetId：

```typescript
export interface SourceImportItem { sourcePath: string, targetName: string }
export interface SourceMutationItem {
  source: string
  target: string
  status: 'imported' | 'moved' | 'conflict' | 'failed'
  message?: string
}
export interface SourceMutationResult {
  operationId: string
  sourceRevision: number
  items: SourceMutationItem[]
}
```

原生命令合同：

```text
pick_source_import_files() -> string[] | null
import_source_files(modId, targetDirectory, items: SourceImportItem[]) -> SourceMutationResult
move_source_file(modId, source, targetDirectory, targetName) -> SourceMutationResult
create_source_directory(modId, parentDirectory, name) -> {path, sourceRevision}
```

空 targetDirectory/parentDirectory 表示 source 根；source 为项目相对路径。只允许文件名作为 targetName/name，不接受路径分隔符。取文件的原生选择器不加类型过滤。路径绝不拼入 shell。

- [x] **步骤 2：先写有意义的文件测试。** io.rs 定义 `copy_to_staging(from:&Path,staged:&Path)->Result<String,String>`（返回原字节的 SHA-256）及 `move_no_replace(from:&Path,to:&Path)->Result<(),String>`；协调层先登记 staging 路径，复制完成并记录哈希后才调用不覆盖移动。tests.rs 使用已有 tempfile：

```rust
#[test]
fn import_preserves_arbitrary_bytes_and_never_overwrites() {
    let temp = tempfile::tempdir().unwrap();
    let from = temp.path().join("input.bin");
    let to = temp.path().join("target.bin");
    let staged = temp.path().join("first.tmp");
    let bytes = [0, 255, 7, 0, 128, 13, 10];
    std::fs::write(&from, bytes).unwrap();
    super::io::copy_to_staging(&from, &staged).unwrap();
    super::io::move_no_replace(&staged, &to).unwrap();
    assert_eq!(std::fs::read(&from).unwrap(), bytes);
    assert_eq!(std::fs::read(&to).unwrap(), bytes);
    std::fs::write(&from, b"changed").unwrap();
    let other_staged = temp.path().join("second.tmp");
    super::io::copy_to_staging(&from, &other_staged).unwrap();
    assert!(super::io::move_no_replace(&other_staged, &to).is_err());
    assert_eq!(std::fs::read(&to).unwrap(), bytes);
}
```

补测：移动后原路径消失、同名目标不被改写；staging 已存在时 create_new 失败且不覆盖；复制中途失败没有 source 半文件；目录输入被拒绝；`../x`、ADS、Windows 保留名、source 祖先重解析点、仅大小写改名被拒绝。测试目标都在 tempdir，不使用真实素材或项目。

- [x] **步骤 3：获得范围授权后运行红灯。** `cargo test --offline --manifest-path src-tauri/Cargo.toml --lib source_files:: -- --nocapture`。因新增模块不存在失败可作为红灯记录，编译环境失败不能记成行为红灯。未获准则只保存测试，不执行。
- [x] **步骤 4：实现可登记的原样复制和移动。** 协调层先选择并持久化 `mods/<modId>/.source-imports/<operationId>/<itemId>.tmp`，再用 create_new 创建临时文件；临时区在 source 外、同一项目卷内，不能由纯复制函数内部随机生成不可见名称。对外源文件取前后 metadata 及实际复制量，发现源变更或 I/O 错误不提交。复制、flush、sync 和哈希完成后先更新 pending 的 resultHash/phase=ready，再通过 Windows `MoveFileW` 将 staging 移到目标；目标已存在时原生失败。普通项目内移动同样使用此不覆盖 API；不能用会替换目标的 rename 模式或“先删目标再移”。不得自动改后缀、格式、时间相关内容或文件引用。

```rust
pub fn copy_to_staging(from: &std::path::Path, staged: &std::path::Path)
    -> Result<String, String> {
    use std::io::Write;
    let mut input = std::fs::File::open(from).map_err(|e| e.to_string())?;
    let before = input.metadata().map_err(|e| e.to_string())?;
    if !before.is_file() { return Err("只能导入普通文件".into()); }
    let mut output = std::fs::OpenOptions::new().write(true).create_new(true)
        .open(staged).map_err(|e| e.to_string())?;
    let copied = std::io::copy(&mut input, &mut output).map_err(|e| e.to_string())?;
    let after = input.metadata().map_err(|e| e.to_string())?;
    if copied != before.len() || before.len() != after.len()
        || before.modified().ok() != after.modified().ok() {
        return Err("导入期间源文件发生变化，请重试".into());
    }
    output.flush().map_err(|e| e.to_string())?;
    output.sync_all().map_err(|e| e.to_string())?;
    drop(output);
    crate::changes::file_hash(staged)
}
```

外部进程持续修改同一文件无法仅由 metadata 提供强一致快照；不声称抵御这类任意并发。项目内并发由下述预留处理。复制函数不负责删除未知文件，也不提交最终目标；协调层根据自己的操作记录清理失败文件。不能依赖 NamedTempFile 析构处理进程异常退出。

- [x] **步骤 5：复用已有路径校验。** 目标、来源项目路径复用 changes::relative_path、paths::reject_redirect；逐个验证祖先及最终普通文件，拒绝链接和保留内部目录。原始导入来源是用户选择的普通文件，无 MIME/解码/像素检查。创建目录只执行一层 create_dir；已存在的普通目录可返回原路径，文件冲突则拒绝。
- [x] **步骤 6：实现总计划的 NativeReservation。** Workspace State 增加 `native_operation: Option<(String, NativeOperation)>`；短锁设置和 Drop 释放。UI 操作/任务准备竞争测试使用两个线程与 Barrier，不用延时猜测：同时竞争只能一方成功，失败不持令牌，其他 modId 正常预留。原生源操作在预留后查询 preparing/running；任务准备先预留再落 preparing，释放后由数据库任务状态阻止 UI。
- [x] **步骤 7：接入源码变更和恢复。** 文件操作不伪造 TaskContext；将其记录到项目 logs/source-operations.jsonl，包含 operationId、相对目标、前后指纹和逐项结果。file_operations 记录 source_import/source_move；复制/移动成功后更新 Workspaces fingerprint/revision，即使多文件部分失败也更新已成功部分。不修改历史任务快照和旧构建回执。

pending payload 固定：`{sourceRelativePath?,targetRelativePath,temporaryRelativePath?,beforeHash?,resultHash?,phase}`。source/target 均相对 source，temporaryRelativePath 则相对项目根且只允许 `.source-imports/<本operationId>/<本itemId>.tmp`。复制的 phase 为 allocated→ready→committed：allocated 记录必须在创建临时目录和文件之前提交，ready 必须含 resultHash 并在 MoveFileW 前提交。移动无临时文件，先记录 source/target/beforeHash，再移动并标 committed。所有记录用短事务，字节复制不持事务或 StorageState。

外部原路径不作为重启自动重新导入的来源。恢复只核对该操作登记的路径、普通文件身份、祖先非重解析和已知哈希；未提交只清理确认归属的 staging，不能确认归属或内容后来变化则 requires_review，不删除猜测文件。ready 但目标未落盘时清理确认归属的 staging，目标存在且 resultHash 匹配时标完成；目标不同不覆盖。MoveFileW 后日志未提交同样按已登记哈希核对，不移回或覆盖用户后来改过的文件。临时区在 source 之外，因此异常中断的半文件不会成为源码输入；禁止扫描 source 中所有 .tmp 并清理。

在内存库/tempdir 中分别构造 allocated 且无文件、allocated 且部分 staging、ready、已移动但未 committed、目标后来变更等明确状态，再执行 production 恢复函数。断言所有自建路径在记录中、source 无半文件、外部原件不变、已完成文件不误删；无法确认身份时保留并报告，不把单元测试中的正常析构当作进程中断证据。

- [x] **步骤 8：注册与异步执行。** 在 lib.rs 注册 command，文件 I/O 放 spawn_blocking。成功及部分成功发 `project-source-changed` Tauri 事件 `{modId,sourceRevision}`；现有 pi 通道的来源通知继续保留，不能要求必须有活跃模型进程才刷新文件树。
- [x] **步骤 9：重跑获准的 T1 filter 并检查调用链。** 包括 preparing/running 均拒绝 UI，任务准备与 UI 预留竞争不会出现新任务读取半批输入，令牌出错后可重试。路径拒绝不能只测前端 disabled。

**T1 交付：** command、纯文件算法和协调可用，数据库不登记虚假 AI 工具或构建记录。

## T2：现有“全部文件”接入

- [x] **步骤 1：扩展现有选择状态。** SourceFileTree 新增 `selectDirectory:[path:string]` 事件，目录行选择和展开分别维护；useSourceBrowser 增加当前目录，点击文件时目标目录取父目录。切项目重置；不让目录选择触发 read_source_file。小视图和放大 Dialog 使用同一状态。
- [x] **步骤 2：增加最小操作合同与前端测试。** useSourceFileActions 对外返回 `busy/error/results/importFiles/moveFile/createDirectory`；每次调用保存 modId 和 generation，项目切换后丢弃旧响应。模块内定义窄调用签名，默认适配真实 nativeCall，不为测试增加通用服务层：

```typescript
export type SourceNativeCall = (command: string, args?: Record<string, unknown>) => Promise<unknown>
// useSourceFileActions 的参数：readModId: () => string、invoke: SourceNativeCall = nativeCall。
// importFiles(targetDirectory, items) 返回 Promise<void>；results 为 SourceMutationItem[] 的 shallowRef。
```

用 vitest mock nativeCall，测试下例流程：

```typescript
it('项目切换后不把导入结果显示到新项目', async () => {
  let finish!: (value: SourceMutationResult) => void
  const call = vi.fn(() => new Promise<SourceMutationResult>((resolve) => { finish = resolve }))
  const target = shallowRef('project-a')
  const scope = effectScope()
  const actions = scope.run(() => useSourceFileActions(() => target.value, call))!
  const importing = actions.importFiles('', [{ sourcePath: 'C:/fixture/a.bin', targetName: 'a.bin' }])
  target.value = 'project-b'
  await nextTick()
  finish({ operationId: 'op', sourceRevision: 1,
    items: [{ source: 'C:/fixture/a.bin', target: 'a.bin', status: 'imported' }] })
  await importing
  expect(actions.results.value).toEqual([])
  scope.stop()
})
```

测试文件显式导入 `effectScope,nextTick,shallowRef`、`vi,it,expect`、合同和本函数；原生返回值按现有 source DTO 校验或在已知 command 分支收窄类型，不断言成任意 Promise。再补测旧 A 响应不能把新 B 正在进行操作的 busy 清空。

- [x] **步骤 3：实现工具栏。** 复用 Button：“导入文件”“移动/改名”“新建文件夹”，保留刷新和放大。导入调用 pick_source_import_files 后直接提交；无冲突不弹第二张确认表。取消选择不调用 import，不显示失败。单文件移动要求当前选中普通文件；项目忙或已归档禁用并显示原因，原生仍独立校验。
- [x] **步骤 4：实现项目内目标选择。** SourceDirectoryDialog 复用 Dialog/Field/Input/Button 和现有 SourceFileTree 的目录子集，目标只能是 source 内已有目录；新建单层目录后刷新目标树。移动选择目标及可选新文件名，提交前说明引用不会自动调整；冲突只允许改名或跳过，不提供替换。
- [x] **步骤 5：处理结果与通知。** 多文件展示逐项成功/冲突/失败；不要一个失败就丢掉其余结果。成功后刷新源与目标目录、选择移动后的文件并更新预览；文件不可预览与导入失败分别显示。监听 project-source-changed，组件销毁及异步 listen 返回时清理监听；inactive 仅标记需要刷新，不无目标轮询。
- [x] **步骤 6：获准后执行新前端测试及限定静态检查。** `node node_modules/vitest/vitest.mjs run src/features/workbench/useSourceFileActions.test.ts`，无全仓测试；vue-tsc 与 ESLint 按总计划。测试不安装任何依赖，不使用假原生窗口。

**T2 检查点：** PNG、文本、任意二进制均原样导入；只在查看时走预览。两种视图操作一致，活动任务拒绝写、项目切换不串结果。真实原生文件选择器验收属于另外获准的应用检查。
