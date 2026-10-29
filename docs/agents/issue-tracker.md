# 任务追踪：本地 Markdown

本仓库的需求和任务以 Markdown 文件存放于 `.scratch/`。

## 文件约定

- 每个功能使用独立目录：`.scratch/<feature-slug>/`。
- 需求规格：`.scratch/<feature-slug>/spec.md`。
- 实现任务：`.scratch/<feature-slug>/issues/<NN>-<slug>.md`，
  从 `01` 开始编号，每个任务独立成文件。
- 任务文件顶部附近使用 `Status:` 记录 triage 状态，
  状态名称见 `triage-labels.md`。
- 评论和讨论追加至文件底部的 `## Comments` 下。

## 技能操作约定

“发布到任务追踪器”表示在上述对应路径创建 Markdown 文件，
必要时创建目录。

“获取相关任务”表示读取用户提供路径或编号对应的任务文件。

## Wayfinder 操作约定

- 工作地图：`.scratch/<effort>/map.md`，
  包含 Notes、Decisions-so-far 和 Fog。
- 子任务：`.scratch/<effort>/issues/NN-<slug>.md`，
  从 `01` 开始编号，在正文记录问题。
- 使用 `Type:` 记录类型：
  `research`、`prototype`、`grilling` 或 `task`。
- 使用 `Blocked by: NN, NN` 记录依赖；
  所有依赖均为 `resolved` 时才解除阻塞。
- 按编号优先选择尚未解决、未阻塞且未被领取的任务。
- 开始处理前写入 `Status: claimed` 并保存。
- 完成后在 `## Answer` 下追加答案，
  写入 `Status: resolved`，
  并在地图的 Decisions-so-far 中追加结论摘要和链接。

Wayfinder 的 `claimed` / `resolved` 表示其任务执行状态，
与普通任务的五个 triage 状态分别使用。
