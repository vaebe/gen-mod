# 产品纠偏实施与验证

本轮已接入 T1–T12 的实现，并执行 T13 中已授权的限定检查；完整故障注入、独立应用与真实 SDK/模型联调尚未验收。用户已明确项目仍在开发阶段，无需考虑旧内容，旧库升级、历史数据兼容和旧素材恢复退出剩余验收范围；已有实现与检查结果保留，不将范围取消记为验证通过。不能据此宣称整份方案已完成全部验收。工作分支为 `codex/gen-mod-improvements`，保留已有工作区改动，没有提交、推送、安装依赖或启动应用/游戏/外部模型，没有操作真实项目数据。

## T1–T2：全部文件

接入原字节多文件导入、项目内单文件移动/改名、目录创建、冲突改名/重试/跳过，两种浏览视图共用选择状态。导入不解码、不转换、不分析内容。复制使用 source 外的已登记 staging、哈希和不覆盖移动；外部原件保留。原生预留和活动任务/生命周期校验阻止 UI 与 AI 写入竞争，成功及部分成功发源码变更事件。

文件夹具覆盖任意字节、不覆盖、路径边界、明确中断状态恢复及用户后来修改的目标保留；首次未实现复制时有实际失败，随后实现通过。预留 API 初次缺失导致的编译失败不作为行为红灯。真实 native picker、窗口交互及任务准备对半批输入的完整应用联调未运行。

## T3–T5：跨项目长期记忆

009 保留旧身份/状态/正文，解除来源项目外键；旧条目全部标记用户维护，同一迁移事务回填正文指纹。自动保存以游戏/主题/版本建立稳定身份，正文指纹防止换来源或主题重建用户改过、停用或删除的内容。用户编辑使用 CAS，删除清除正文，保留最小防恢复关联。来源项目移除不会删除长期记忆。

接入真实消息 digest、命令 receipt、资料和已确认验收来源核对；按游戏、版本、关键词有界检索，实际使用副本独立保存。设置页提供搜索/筛选/分页、新增/编辑、启用/停用/删除；切换 Tab 保留编辑组件。聊天直达对应条目，已删除条目不误开其他记录。

reducer 实时和历史都保留 details，校验 memorySave/projectMemoryUpdate；助手正文、部分输出、错误结果不能生成保存成功提示。普通命令回执继续可查看。先运行新增 reducer 用例观察到两项实际失败，再实现并复测。

纯仓储与来源夹具不等同于真实模型自然语言总结准确性。确定性身份/正文保护不识别任意语义改写，Skills 明确禁止换键或改写撤回内容规避用户决定。设置页的真实编辑/冲突/删除交互、聊天直达尚未在窗口中验收。

## T9–T11：项目生命周期

010 增加 archived 和不依赖项目行的删除墓碑；get_mod 改为只读 DTO 刷新，revision 不再因界面读取增加。归档/恢复使用 CAS；统一守卫在原生实际写入路径检查归档、删除及 pending/requires_review，保留中断恢复未完成时的写入阻断。

ProcessConnect 覆盖检查至进程注册，ProjectLifecycle 在预留后关闭空闲连接，关闭期间不持 StorageState，再以短存储临界区变更状态或 rename。迟到源码租约和反馈草稿也在实际提交前复核项目/任务状态。预留的 Barrier 夹具证明互斥与跨项目独立，不把它描述为真实进程关闭/重连已经验收。

永久删除只处理受控项目目录和项目私有表，保留长期记忆及指纹关联；逻辑提交后清理目录失败返回 removed=true/cleanupPending=true。启动先恢复删除，再执行旧 JSON 导入，墓碑阻止重导入。恢复冲突保留两份目录、持续阻写并记录原因。

临时目录/内存表夹具核对所有私有子表清理、外键一致性、另一项目/外部安装文件/长期记忆保留、CAS 和归档守卫、事务失败回滚及目录恢复、已提交/未提交/冲突恢复。Windows junction 故障夹具证明清理停止并保留外部原字节。未运行完整应用重启、真实关闭失败或真实项目永久删除。

项目页与侧边栏共用移除选择框，默认归档；归档列表可恢复/只读查看/永久删除。晚响应不能重新插入已失效 ID，当前项目移除后清理其会话缓存并离开页面。归档界面禁用安装/导出/删除，原生校验独立保留；这些 UI 行为尚未做窗口验收。

## T6–T8：项目记忆与任务输入

项目创建同事务初始化空记忆，旧项目在导入后回填，不提升已有 revision。工具只允许更新版本、前置和明确需求，核对真实来源、任务及 CAS；live 更新不改历史 snapshot。资料目录仍由用户选择，调查事实直接应用，无旧技术表单前置。

快照与 preparing 记录同事务建立。bootstrap 先写游戏/版本和玩家约束，再写前置与资料状态，摘要最多 6000 字符、历史最多 20 项，超限标记并指示读取完整记忆。扩展初始化核对字节指纹及身份、缓存参考消息、注册 handler 后才发 ready；自由文本不进入 systemPrompt。

stderr 的握手先投递专用通道，不等待会话槽锁；普通初始化诊断缓冲避免挡住 ready。原生最多等 10 秒，只有匹配 ready 才写 prompt。准备取消注册先于进程事件，握手等待可直接收到取消；受理取消后拒绝迟到 ready，并与最终 prompt 发送串行核对。

项目上下文新增“记忆”Tab，展示事实、来源数量、未知状态和按需目录调查；切项目/卸载丢弃迟到响应，归档只读。内存通道夹具核对握手拒绝、超时/退出/取消及槽锁不阻塞投递；没有启动实际 SDK，不能据此声明扩展初始化、模型注入或歧义处理已通过真实联调。真实 Job 停止及零次 prompt 的完整应用故障注入仍未验收。

## T12：退役兼容与 Skills

旧环境弹窗、候选经验入口、独立素材 UI 和图片处理链路退出注册、白名单与资源分发；保留原迁移、素材元数据和旧 pending 核对。旧素材恢复使用独立 `{assetId,status,sourcePath}` 合同，不给通用文件结果添加素材状态。只有未恢复记录时显示恢复入口；选择后原样复制到 source/assets/legacy，同哈希幂等，冲突不覆盖，不复活已中断任务的处理结果。

原图/派生元数据、缺文件、坏哈希、中断任务及不解码的字节恢复使用临时文件和内存表验证。移除旧图片测试，保留 ResourceManifest 的实际资源哈希与越界拒绝回归。两份 Skills/registry 更新到 2.1.0，说明新记忆协议、用户维护优先、普通文件路径及运行时 Skills 只读；没有新增 API 或游戏资源加载结论。本轮没有重新运行子代理/真实模型 Skills 行为评估。

prepare-runtime 只清理 portable 中两个确切退役文件，再复制、更新哈希清单。同步前核对了本地 node.zip、node.exe、两份部署标记和已安装依赖，未进入下载/部署安装分支。实际分发中 assets.ts 和 process-image.ps1 均不存在。

## T13：实际命令与结果

限定 Rust 命令前缀均为：

```powershell
cargo test --offline --manifest-path src-tauri/Cargo.toml --lib <filter> -- --nocapture
```

本轮对应 filter 与结果：source_files:: 8 项、pi::workspace::tests:: 4 项、pi::bridge::tests:: 3 项、knowledge:: 10 项、project_context:: 5 项、project_lifecycle:: 6 项、storage::database::tests:: 1 项、assets:: 2 项；共 39 项通过。没有执行整个 Rust 测试套件。

前端及资源：

- `node node_modules/vitest/vitest.mjs run src/services/pi/reducer.test.ts src/features/workbench/useSourceFileActions.test.ts`：两文件共 7 项通过。
- `node node_modules/vue-tsc/bin/vue-tsc.js --noEmit`：通过。
- 本地 ESLint 使用总计划明确文件数组，含本轮直接影响的版本只读菜单和旧素材恢复组件；无 --fix，限定检查通过。分步修正过格式错误与警告，不把最初失败隐藏为一直通过。
- 便携扩展文件数组执行 `node node_modules/typescript/bin/tsc --ignoreConfig --noEmit --target ES2022 --module NodeNext --moduleResolution NodeNext --strict --skipLibCheck --esModuleInterop --types node`：通过。
- `node scripts/prepare-runtime.mjs`：本地资源同步通过，清单为 11280 个运行时文件、19 个 Skills 文件。
- `powershell.exe -NoProfile -NonInteractive -ExecutionPolicy Bypass -File scripts/tests/verify-local-assets.ps1`：通过；输出中的“资源清单包含无效路径段”是故意提供 ../escape.png 的拒绝断言，不是被忽略的失败。夹具保留在 work/asset-evaluation/9e69fb11-8c5f-48de-9341-ad8cb02f9c39。
- `codegraph sync`、`codegraph status`：通过，索引为 up to date（326 个文件）。`git diff --check` 通过；53 个相关未跟踪代码文件无行末空白。设计 Markdown 日期行的两个空格是既有强制换行，保留原样。索引维护不替代功能验证。

Cargo 初次在受限环境被 Tauri build script 的访问权限阻止；随后以正常权限离线执行限定测试。vue-tsc 在受限环境曾整体误报 Vue/VueUse 导出缺失，正常权限检查通过，没有为该环境问题修改无关源码。

没有执行完整构建、安装包生成、桌面 UI 验收、实际 SDK/模型联调、真实用户库迁移、Duckov 编译或游戏效果验证。其中旧库升级及历史兼容已由用户取消，不再补测；当前版本新建项目的初始化和持久化仍在验收范围。实施合同中的其余复合验收项未完全覆盖时保留未勾选；已完成实现和已运行检查不代表这些剩余验收已通过。
