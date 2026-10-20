# 项目文档入口

后续接续开发，先阅读 [当前项目上下文](./project-context.md)，再按任务需要查阅设计和计划，并核对当前源码。不要把旧实施记录当作最新状态或运行验收结论。

2026-10-20 已接入文件操作、两类记忆、归档/永久删除及旧入口退役。实际检查和未执行验收以[本轮实施记录](./verification/2026-10-20-product-correction.md)为准；当前工作区未提交。

2026-10-12 已同步 shadcn-vue 组件统一调整与使用约定，覆盖标准控件、菜单、折叠区、表单、卡片和状态展示；检查结果与未验收范围见 [阶段实施记录](./verification/implementation-progress.md#2026-10-12-shadcn-vue-组件统一调整)。本次文档同步保留已有打包说明，未运行应用或测试。

- [当前项目上下文](./project-context.md)：已确认需求、当前实现、数据保存、关键文件、已知限制及检查方式。后续相关改动完成时同步更新。
- [文件管理、只读 Skills 与记忆调整方案](./superpowers/specs/2026-10-20-gen-mod-product-correction-design.md)：2026-10-20 已确认并开始实施，覆盖文件操作、两类记忆、只读 Skills 及项目归档/永久删除。
- [产品纠偏执行计划](./superpowers/plans/2026-10-20-gen-mod-product-correction-plan.md)：四份分项计划、T1–T13 任务、迁移及工具合同、兼容与验证边界；[本轮验证记录](./verification/2026-10-20-product-correction.md)区分已实现、检查结果和未验收范围。
- [能力改进与 Skills 优化整体方案](./superpowers/specs/2026-10-19-gen-mod-improvement-design.md)：2026-10-19 已批准的 P1–P7 方案，包含已确认需求、七项能力、Skills 优化及收益与成本比较。
- [能力改进执行计划总入口](./superpowers/plans/2026-10-19-gen-mod-improvement-plan.md)：经子代理审核的七份分项计划、共享合同和授权边界；代码已实现，限定检查与待验收项目见执行记录。
- [项目架构](./architecture/README.md)：当前工作区的总体架构、模块职责、生成交付链路、多会话、存储与安装边界，附交互式架构图。
- [设计文档](./2026-10-09-gen-mod-design.md)：初始架构、pi RPC 与 SDK 取舍、skills、MOD 版本和安装流程。
- [执行计划](./2026-10-09-gen-mod-implementation-plan.md)：首期实施任务及验收范围。
- [多会话并发、引导与 SQLite 计划](./2026-10-09-multi-session-steering-plan.md)：同项目共享源码并发、运行中补充要求、会话圆环与数据迁移；源码任务 1–7 已实施，运行验收 Gate 待执行。
- [阶段实施记录](./verification/implementation-progress.md)：按阶段保留的历史记录；后续交互调整以当前项目上下文为准。
- [Windows 发布记录](./verification/windows-release.md)：历史安装包、资源检查与 2026-10-12 打包优化记录；区分已构建的 zlib 安装包与尚未重新构建的当前 LZMA 配置，安装包不自动包含后续源码改动。

本目录保存开发决策与接续信息。用户会话、附件、API Key 和 MOD 文件由应用自己的数据目录管理，不复制到文档。

- [P1–P7 实现检查与应用验收](verification/2026-10-19-gen-mod-implementation.md)
- [Skills 质量评估](verification/skills-quality-evaluation.md)
