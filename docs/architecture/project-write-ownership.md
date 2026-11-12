# 项目聚合读取与局部写入

任务 11 将项目资料写入迁移至 src-tauri/src/projects/repository.rs。ModProject 聚合读取继续使用 storage::repository::read_on/read，IPC 参数与聚合返回值不变，不迁移数据库结构。

## 项目资料

| IPC 消费者 | 所属操作 | 写入范围与保护 |
| --- | --- | --- |
| storage/mods.rs:create_mod | projects::repository::create | 新项目行与初始项目环境在同一事务创建；拒绝已删除身份，不写会话表 |
| storage/mods.rs:rename_mod | DetailChange::Name | 名称、updatedAt、revision；命令保留名称校验 |
| storage/mods.rs:update_mod_description | DetailChange::Description | 描述、updatedAt、revision；保留 expectedDescription 比较与 trim |
| storage/mods.rs:save_mod_draft（无 sessionId） | DetailChange::InitialDraft | 初始草稿、updatedAt、revision；事务内确认没有未删除会话，草稿原样保存 |

资料更新使用 Immediate 事务、expected revision 条件更新和事务内 ensure_writable_on；只允许上述固定业务字段，不接受任意字段名或完整 DTO 覆盖。创建入口接受创建命令构造的新项目，不能用于更新。资料操作不接管初始草稿，不同步会话、版本、安装记录或当前安装状态。

首会话草稿接管属于任务 12：必须在创建首会话的同一事务完成，不能调用资料操作拆成第二个事务。

## 已完成的分批迁移记录

此清单记录任务 11 完成时的剩余消费者，后续任务完成时更新。路径或函数若被任务 09 拆分，应沿原函数职责追踪到新位置，不按旧行号机械修改。

### 任务 12：会话和任务写入（已迁移）

会话业务写入归 `pi/session_repository.rs`，通过 `SessionChange` 的 Create、Rename、Delete、History、RefreshSkills、Runtime 固定操作写入。Immediate 事务内检查可写性、按 revision 更新项目；只更新指定会话及必要项目字段，不重写版本、安装和无关会话。内部事件在事务内重读 revision，删除操作使用调用方已读取的 revision 以保留文件补偿边界。

- `bridge` 的 session_selected 原子创建会话、接管初始草稿及保存初始模型；仅在没有未删除会话时转移草稿，事务失败整体回滚。
- `bridge` 的终态和异常退出通过 Runtime 更新状态、时间及空白会话标题；同事务检查其他活动任务，保留原项目归属。任务 09 的 `bridge/preparation.rs` 使用同入口写 Running。
- `bridge` 保存历史文件关联、模型、思考强度及统计使用会话仓储；模型切换仍原子清除不适用的窗口统计。
- `pi/sessions.rs` 重命名和 `bridge` 删除使用局部操作；历史文件 tombstone、恢复记录与失败补偿保持。
- `storage/mods.rs` 会话草稿保存迁入 `save_draft_on`，同事务可写检查；expectedRequirement 不匹配时保留新输入。无 sessionId 的初始草稿仍由项目资料操作负责。
- `cloud/catalog.rs` 和会话选择通过 `refresh_skills` 持久化会话失效状态。读取时 `list_mods/get_mod` 的 refresh_sessions 仍只改返回投影。

会话偏好、模型和统计沿用原有局部列及校验语义，聚合读取保持。任务 12 阶段曾暂留通用仓储的任务、日志和恢复职责；任务 13/14 已完成后续迁移，最终归属见下文。

### 任务 13：交付和安装写入（已迁移）

| 消费者 | 所属局部操作与约束 |
| --- | --- |
| artifacts/registry.rs:register | artifacts::repository::register_on 只追加 versions、设置 latestVersionId、Ready 和 updatedAt；加入既有 Immediate 事务，与交付锚点及任务登记一起回退 |
| artifacts/registry.rs:delete_version | artifacts::repository::delete_on 只更新版本集合、最新版本、必要的 Generated 和 updatedAt；保持锚点删除标记同事务、目录恢复及部分完成报错 |
| install/installer.rs:preview_install | install::repository::remember_directory 只保存通过校验的 lastDeploymentDirectory 与 updatedAt；保留预览令牌失效规则 |
| install/installer.rs:install_version | install::repository::record_attempt 原子追加历史与更新当前安装目标；未触碰文件的失败保留旧目标，部分复制失败标为 uncertain，成功标为 known |

全部操作保留 expected revision、事务内可写检查及 revision 递增；不写 sessions、初始草稿、项目标题或描述。完整 ModProject 仍由 storage::repository 聚合读取。登记的取消、活动任务、租约、命令证据、内容检查、路径保护及安装的预览比较仍归原命令；文件恢复与 pending/done 日志顺序保持。

交付模型归 artifacts/models.rs，安装模型归 install/models.rs，storage/models.rs 只组合聚合 DTO。前端版本页、列表/详情/确认、安装界面、版本操作及 06 的交付快照归 features/delivery；contracts/delivery 与 contracts/install 拥有版本和安装类型，contracts/mod 仅组合项目读取。features/delivery/service.ts 绑定各命令的参数与结果并复用 nativeCall 传输；项目读取复用 modService。没有新增平行 store 或事件总线。

### 任务 14：项目生命周期与通用存储收缩

应用启动编排归 application/startup.rs：数据库初始化、项目删除恢复、旧任务中断/引导核对、未启动提交拒绝、文件操作核对，顺序及事务保持。database 只负责连接、当前结构初始化及 WAL。

旧 storage::repository::{save_on,save,create,update} 已移除；聚合读取保留。安装恢复归 install::repository::recover_attempt_on：Immediate 事务内检查可写性，按安装身份幂等补 failed/notAttempted 和 uncertain，保留 updatedAt、会话及草稿，不重放文件安装。既有记录保持，只推进 revision 与恢复日志。

项目页面、新建、归档/删除、描述、侧栏归 features/projects；项目 typed service 同目录，native.ts 仅传输。会话弹窗和运行态仍由会话能力所有。

## 验证边界

已在项目仓储公共操作边界执行四项 Rust 定向测试：资料更新不触碰会话且保留版本/安装聚合信息；陈旧 revision、描述冲突、归档拒绝无部分写入；初始草稿原样保存及会话存在时拒绝；创建与初始环境原子提交、删除身份拒绝及环境失败回退。

命令：cargo test --manifest-path src-tauri/Cargo.toml projects::repository::tests --lib --no-default-features。TDD 每个新增接口先确认编译缺失的红阶段，再实现并通过；未声称旧实现发生运行时故障。未启动应用、模型或游戏，不代表 IPC 真机或打包验收。

独立工作树测试使用 runtime/resources 到主仓同名目录的 junction，并复用主仓 src-tauri/target。归档工作树前仅移除 junction 本身，不递归删除其目标资源。

任务 12 在原生会话仓储边界新增并执行四项 TDD 测试：首会话草稿原样接管与故障回滚、后续创建不重复接管；局部会话操作隔离及陈旧 revision/归档拒绝；运行状态/历史/Skills 保留已有草稿和用户标题；条件草稿清理保留发送期间新输入。命令：`cargo test --manifest-path src-tauri/Cargo.toml pi::session_repository::tests --lib --no-default-features`。每个新增操作先确认缺失接口的红阶段，再实现通过。未启动应用、模型或游戏。

任务 13 在交付/安装仓储公共操作边界逐个执行新增接口缺失的 TDD 红阶段，再实现绿阶段；覆盖会话表不可写时的局部更新、初始/会话草稿和安装历史保留、陈旧 revision/归档拒绝、登记事务回滚及安装目标 known/uncertain/未触碰三种语义。`cargo test --manifest-path src-tauri/Cargo.toml repository::tests --lib --no-default-features`；前端复用版本操作和 sessionRuntime 中跨能力刷新测试，迁移 vitest include 路径。类型检查使用已安装 vue-tsc 的 `--noEmit`。没有运行真实文件复制、应用、模型、游戏或打包；删除补偿及安装校验按调用链静态核对，不能视为真机验收。

13 工作树另有 node_modules junction 指向主仓现有依赖。归档时只移除链接本身，不能递归删除目标依赖及 runtime/resources。

13 本次实际结果：Rust仓储定向10项通过（包括11/12回归）；版本操作与跨能力刷新25项通过；vue-tsc --noEmit与限定变更前端文件ESLint通过。首次Lint发现5项本轮导入排序/重复导入，已修正后通过。

14 独立树复用主仓 node_modules 与 runtime/resources junction；清理只删除链接本身。生命周期定向七项通过，vue-tsc --noEmit 通过；新增安装恢复断言属于重构后回归，未声称 TDD 红阶段。未启动应用、模型、游戏或打包。

14 最终收缩：任务保存/读取和活动任务判断归 pi/tasks.rs（TaskRecord 原本已归此处）；运行日志读写归 pi/logs.rs（RuntimeLogEntry 原本已归此处）；会话偏好、元数据及 steering 持久化归现有 pi/session_repository.rs。跨能力文件操作日志和恢复问题查询归 application/file_operations.rs。所有调用者与 IPC 注册同步，storage/repository 只保留序列化和项目聚合读取。合并07/16后的前端26项、vue-tsc和限定ESLint通过。
