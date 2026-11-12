# GenMod 开发文档

当前说明以项目整体优化整合后的源码为准。代码整理、类型检查、测试和真实运行验收分开记录；这些说明不承诺已发布安装包包含当前代码。

## 当前入口

- [项目上下文与能力边界](project-context.md)
- [当前架构与能力所有者](architecture/README.md)
- [本轮整合、完整覆盖映射与验证限制](verification/2026-11-12-project-optimization.md)
- [原始优化方案与修订历史](2026-11-11-project-optimization-plan.md)
- [优化开始时的覆盖基线](verification/2026-11-12-optimization-inventory.md)
- [领域术语](../GLOSSARY.md)

## 有效设计与保留约束

- [界面与品牌设计](design/2026-11-06-ui-and-brand.md)：品牌源文件与应用图标源保持在用。
- [会话恢复设计](design/2026-11-06-session-runtime-recovery.md)：受理确认、身份与恢复语义。
- [兼容清理方案](superpowers/plans/2026-11-11-compatibility-cleanup.md)及[审核记录](verification/2026-11-11-compatibility-audit.md)：正常空值、恢复和环境适配不能作为废代码删除。
- [游戏资源说明](../resources/games/README.md)：适配状态及资源来源。
- [协作与任务跟踪](agents/issue-tracker.md)、[标签](agents/triage-labels.md)、[领域文档约定](agents/domain.md)。

## 历史资料

[归档索引](archive/README.md)保存旧设计、实施计划、交互架构图与原日期验证证据。旧位置的 Markdown 指引只用于追踪，不保留第二份正文。历史计划中的路径、分支、尚未提交状态和验证结果不能作为当前事实。
