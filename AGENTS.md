# 项目协作规范

- UI 开发优先使用 shadcn-vue 已有组件。修改前读取 `.agents/skills/shadcn-vue/SKILL.md`，先检查 `src/components/ui` 和官方组件文档；已有组件满足需求时直接复用，不用原生控件或自定义标记重复实现。仅在现有组件无法满足明确需求时使用自定义实现；新增组件或依赖遵循授权要求。

## Agent skills

### Issue tracker

使用本地 Markdown 跟踪任务，存放于 `.scratch/<feature>/`。详见 `docs/agents/issue-tracker.md`。

### Triage labels

使用五个默认 triage 标签。详见 `docs/agents/triage-labels.md`。

### Domain docs

采用 single-context 布局：根目录 `GLOSSARY.md` 和 `docs/adr/`。详见 `docs/agents/domain.md`。
