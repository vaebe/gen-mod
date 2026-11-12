# GenMod 当前架构

界面提出业务操作，Rust 管理持久状态和本机资源，内置 pi 通过显式工具扩展调用模型并操作工作文件。任务受理、终态、身份、租约、路径和交付证据校验仍以原生侧为准；类型契约不替代运行时校验。

## 能力与实现位置

| 能力 | 当前所有者与职责 | 详细说明 |
| --- | --- | --- |
| 项目 | src/features/projects；src-tauri/src/projects/repository.rs；project_lifecycle 管理归档/删除 | [聚合读取与局部写入](project-write-ownership.md) |
| 会话 | features/sessions 管理存续、输入、恢复、队列与只读投影；services/pi/client.ts 只负责传输 | [会话接口迁移闭环](session-interface-migration.md) |
| 聊天 | features/chat 管理消息投影、展示与会话日志，工作台仅组合 | [会话与聊天边界](session-interface-migration.md) |
| 工作文件 | features/workspace 管理编辑、文件操作、引用、保护与变更视图；Rust source_files 保持文件安全 | [工作文件](workspace.md) |
| 交付/安装 | features/delivery 复用版本操作与安装界面；artifacts/install 仓储局部写入 | [交付和安装写入](project-write-ownership.md) |
| 项目记忆 | features/project-memory；原生 project_context 保持快照、保护与冲突校验 | [项目记忆](project-memory.md) |
| 长期记忆 | features/knowledge；原生 knowledge 保持来源与删除边界 | [跨项目长期记忆](knowledge.md) |
| 玩家反馈 | features/verification；原生 verification | [反馈契约](player-feedback.md) |
| 游戏/Skills | features/games、features/skills；resources/games/catalog.json 提供显示提示；Rust 保持校验 | [游戏与 Skills](games-skills.md) |
| 设置/认证 | features/settings、features/auth 的类型与服务；原生 config/cloud 保持凭据规则 | [设置与认证](settings-auth.md) |
| 工作台 | features/workbench 组合布局、导航与能力面板；不拥有各能力数据 | [项目上下文](../project-context.md) |

## 任务链路和跨能力协调

会话操作经过 PiClient/IPC 到原生 bridge。pi/process.rs 负责进程与 JSONL 适配，subscription 负责订阅；适配层不读取业务仓储或持有会话锁。

pi/bridge/preparation.rs 集中模型、历史、Skills、不可变快照、preparing 与提交绑定、bootstrap 和准备失败补偿。preparing、记忆快照与提交绑定保持事务；bootstrap 成功且取消检查通过后才发送需求。失败、超时或受理的取消不能降级启动。

pi/bridge/termination.rs 集中停止、异常退出、终态、租约及失败收尾。重复和迟到事件仍经身份与 generation 保护。会话槽、工作区预留、存储锁及释放顺序沿既有流程保留，未公开内部可写状态。

前端应用协调入口按事件原项目与会话转交工作文件、交付、项目记忆和反馈更新，各能力持有自己的快照和失效状态。异步代次保护避免旧结果覆盖新视图；没有新增全局事件总线。

## 数据与启动恢复

storage/repository 保留序列化与项目聚合读取；旧 save_on/save/create/update 通用业务写入口已经移除。项目资料、会话、交付、安装由所属仓储通过固定业务操作写入，跨模块原子事务不拆散。pi/tasks.rs、pi/logs.rs 与 pi/session_repository.rs 分别负责任务、日志和会话/steering 持久化。

application/startup.rs 依次初始化数据库、恢复项目删除、核对旧任务及 steering、拒绝未启动的 pending 提交、核对文件操作。application/file_operations.rs 保存跨能力恢复日志；安装恢复只修正元数据，不重放安装。首会话草稿接管、归档可写限制及删除边界见[写入说明](project-write-ownership.md)。

## 工具与分发

[Node 工具边界](node-tools.md)将任务上下文、请求契约与诊断从命令执行中分离；mod-tools.ts 仍是原生显式加载入口。CLI、扩展、历史 helper 及其内置 Node 由同一 runtime lock 解析，[运行时与构建生命周期](runtime-build.md)解释声明检查、资源复制、受控退役及 CI 宿主和内置 Node 的区别。

租约是应用内部协作机制，Windows Job 管理进程树；这些机制不构成操作系统级文件沙箱。

## 历史图与验证限制

[旧交互架构图](../archive/architecture/gen-mod.html)、[图源](../archive/architecture/gen-mod.architecture.json)及[原验证记录](../archive/architecture/verification.md)属于历史快照，不表示上述最终能力布局。保留原生成路径、失败的视觉检查与未完成复核，不为本轮重绘或改写证据。

[本轮整合记录](../verification/2026-11-12-project-optimization.md)区分实际检查、阶段证据与未验收范围；源码架构不等于安装包或游戏运行结果。
