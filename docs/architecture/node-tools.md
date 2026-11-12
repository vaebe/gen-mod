# Node 工具边界

`runtime/extensions/mod-tools.ts` 仍是原生唯一显式加载入口，按原有职责注册文件、项目记忆、玩家反馈、长期记忆、环境调查、命令和交付工具。

- `task-context.ts` 持有原生任务上下文类型与读取；各工具直接使用，不再通过命令执行模块读取。
- `diagnostics.ts` 持有 stderr 诊断和共同脱敏规则；工作区协调与命令日志共用。`GEN_MOD_DIAGNOSTIC` 前缀及 `command_diagnostic` 类型保持，原生 `pi/process.rs` 继续消费。
- `commands.ts` 负责程序探测、命令执行与回执，保留路径限制、子进程环境过滤、取消回收和输出跨分片脱敏。
- `task-contracts.ts` 定义 Node 与 `pi/task_tools.rs` 的请求名称、参数和结果对应关系。`task-request.ts` 只按请求名称推导结果，业务消费者不能指定任意返回类型。交付预检查复用该契约，保留自己的错误说明。

这些类型属于 Node 分发边界，不依赖未打包的前端文件。跨进程接收端仍执行身份、工具参数、租约、路径及证据校验；TypeScript 类型不代替这些检查。

## 分发

`runtime/package.json` 包含整个 `extensions` 目录；`scripts/prepare-runtime.mjs` 覆盖复制该目录到 portable 运行时，原生准备流程显式加载其中的 `mod-tools.ts`。本轮增加三个模块并覆盖原有模块内容，没有退役文件路径；重复准备通过覆盖 `commands.ts` 去除其旧实现，无需增加删除列表或清空生成目录。已有受控退役资源清理保持。

扩展校验必须使用 Node/pi SDK 的加载与工具边界；前端类型检查不覆盖该目录。应用、模型任务和游戏验收另行进行。
