> 历史档案：保留原日期、计划与验证限制，不代表当前实现或本轮验收。当前入口见[文档首页](../../../README.md)。

# 本地图片素材分项执行计划

> **面向 AI 代理的执行者：** 使用 executing-plans，继承[总计划](2026-10-19-gen-mod-improvement-plan.md)门槛。本分项已批准，仅覆盖 PNG/JPEG 图片。

**目标：** 导入并保留玩家原图，生成可追溯的处理结果，将实际支持的资源正确送入源码与交付包。

**架构：** Rust 管理 assetId、路径、指纹和恢复；现有 Windows 图像能力处理 PNG/JPEG；进入 source 的副本使用原生写租约。游戏加载方式由对应 skill 的已核对方法决定。

**技术栈：** 现有 Rust/SQLite、Windows PowerShell/System.Drawing、Vue；不新增图像 crate、ffmpeg 或 Blender。若目标环境现有图像能力不可用，报告阻塞并评估替代，不自动安装依赖。

**规格：** [整体设计第 7.7、9–11 节](../specs/2026-10-19-gen-mod-improvement-design.md#77-本地素材导入与处理)。依赖 P2、P3；与 P4 的交付元数据一致。

**执行状态：** 已按完整范围实现并完成限定检查；此文保留原始任务合同。逐项结果、实现调整及未运行的应用/游戏验收见 [执行记录](2026-10-19-execution-progress.md)。

## 文件与合同

- 创建：`src/contracts/asset.ts`、`src-tauri/src/assets/{mod.rs,models.rs,repository.rs,images.rs,tests.rs}`、`src-tauri/src/storage/migrations/008_assets.sql`、`runtime/helpers/process-image.ps1`、`runtime/extensions/assets.ts`、`src/features/assets/ProjectAssetsDialog.vue`。
- 修改：`lib.rs`、`storage/database.rs`、`storage/repository.rs`、`pi/workspace.rs`、`pi/bridge.rs`、`runtime/extensions/mod-tools.ts`、`src/services/native.ts`、`src/features/mods/ProjectPage.vue`、Duckov `scripts/package.ps1` 与资源专题。

```typescript
export interface ImageRecipe {
  width: number, height: number
  fit: 'contain' | 'cover'
  crop?: { x: number, y: number, width: number, height: number }
  sampling: 'nearest' | 'smooth'
}
export interface ProjectAsset {
  assetId: string, modId: string, originalName: string
  relativePath: string, sha256: string, width: number, height: number
  mediaType: 'image/png' | 'image/jpeg'
  inputAssetId?: string, recipe?: ImageRecipe
}
```

## 任务 1：导入与恢复

- [ ] project_assets(asset_id 主键、mod_id、payload) 保存源图和派生图；路径固定为 `assets/originals/<id>.<ext>` 或 `assets/derived/<id>.png`，不从模型输入拼任意路径。
- [ ] 玩家选择文件后原生检验实际签名、文件大小和 dimensions，再复制与流式计算哈希。上限为输入 10 MiB、总像素 16,777,216、单边 8192；PNG/JPEG 头先按有界读取解析尺寸，再交给解码器，不能先无界解码再检查。
- [ ] 导入记录 `asset_import` pending，复制到自己生成的临时目录，原子转到最终路径，再短事务保存。恢复仅检查已记录路径与哈希，不重读玩家原目录或删除其他文件。
- [ ] UI 取消导入不产生记录；已导入源图之后不被处理命令覆盖。

## 任务 2：确定性的图片处理

- [ ] Rust `validate_recipe(input_width,input_height,recipe)` 检查正整数尺寸、输出像素上限、crop 位于输入范围；指定参数外不自动猜裁剪或改变方向。
- [ ] process-image.ps1 参数为 Input、Output、RecipeFile，路径均原生提供；输出固定 PNG。contain 保留比例并用透明画布补齐，cover 按中心裁剪，crop 先裁后缩放；nearest/smooth 明确对应插值方式。释放所有图像/GDI对象。
- [ ] 调用 helper 使用受控 PowerShell、参数数组和隐藏窗口，清除模型密钥环境；不把任意用户文本拼成 shell 命令。处理只写 assets/derived，不持源码租约；配方、输入指纹、输出指纹在提交前完整保存。
- [ ] 用 `asset_process` 恢复记录处理跨文件边界；取消/失败不登记成功结果，原图保持不变。
- [ ] 先定义如下纯验证测试，获准后执行：

```rust
#[test]
fn rejects_crop_outside_original_image() {
    let recipe = ImageRecipe { width: 64, height: 64, fit: "contain".into(), sampling: "nearest".into(), crop: Some(Crop { x: 90, y: 0, width: 20, height: 20 }) };
    assert!(validate_recipe(100, 100, &recipe).is_err());
}
```

ImageRecipe/Crop Rust 字段与合同一致，字符串枚举值在 serde/校验中受限。处理行为用自行生成的 2×2 RGBA 图验证透明像素、输出尺寸和原图哈希；不使用真实游戏素材。

## 任务 3：进入源码与并发

- [ ] `process_project_asset(assetId,recipe)` 由当前任务绑定项目，只能读该项目已导入资源；`materialize_project_asset(assetId)` 将选定结果复制到 `source/assets/<assetId>.png`。
- [ ] 原生租约 operation 固定为 `materialize_asset`，加入 acquire 写白名单、path 校验、release 的 source fingerprint/diff 处理以及 source_changed 通知。复制须持相同项目的写租约。
- [ ] 目标不存在时原子创建；已存在且哈希相同可幂等返回，哈希不同拒绝覆盖并要求重新查看，不绕过先读后写规则。
- [ ] 素材复制发生后构建回执必须对应新 revision，旧构建不能用于交付；仅编辑素材元数据不改变 source revision。
- [ ] 两个新模型工具同时注册并加入 --tools；UI 的导入入口不允许模型提供外部文件路径。

## 任务 4：明确资源清单与实际交付

- [ ] Duckov package 新增可选 `-ResourceManifest`，JSON 数组 `{source,target}` 的 source 相对 sourceRoot，target 相对 MOD 子目录。逐项校验文件存在、相对路径、普通文件、重复目标与内容检查，不复制整个目录或通配符 DLL。
- [ ] ResourceManifest 写入当前 `.delivery/<taskId>/`，避免构建后修改交付说明等真实源码；package 仍复制固定四类文件，再复制清单内额外资源。未提供清单时保持旧行为。
- [ ] `prepare_delivery` 接收的文件集必须与 package 实际文件及安装映射一致，P3 对最终文件复核。README 列出实际素材，不将预览成功写成游戏可加载。
- [ ] 在新增游戏资源加载代码前，核对对应版本 MOD 安装目录获取方式与图片加载 API。未取得依据时仅完成独立图片处理能力，不编造路径属性或承诺业务 MOD 已生效；如需缩减已选交付范围，先报告并确认。

## 任务 5：界面与核对

- [ ] ProjectAssetsDialog 展示原图/处理图、尺寸、用途与配方；玩家可预览后选择加入任务，原始路径不作为分发路径显示。
- [ ] 获准后运行 `cargo test --lib assets:: -- --nocapture`；PowerShell 图像测试和资源打包测试明确在隔离临时目录执行，经许可才启动。
- [ ] 检查恢复分派包含 asset_import/asset_process，工作区操作白名单与通知已覆盖 materialize_asset，资源准备脚本确实复制新 helper。
- [ ] 限定静态检查、资源同步与 CodeGraph sync/status。未运行应用与游戏时不写“资源已在游戏中验证”，不自动提交。
