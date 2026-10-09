# 交付协议

`run_command` 参数：`program`、`args`（字符串数组）、`cwd`（相对源码目录，默认 `.`）。返回真实 commandId、toolCallId、exitCode、status、logPath，只有 status=succeeded 且 exitCode=0 可作为交付依据。

`prepare_delivery` 参数：`files: [{source, target}]`、`commandIds: string[]`、`targetDirectoryHint: string`。source 相对当前源码目录；target 相对用户以后所选安装目录。例：source 为 `.delivery/<taskId>/ExampleMod.dll`，target 为 `ExampleMod/ExampleMod.dll`。路径统一 `/`；禁止绝对路径、`..`、符号链接和重复安装目标。

工具从原生注入的上下文取得 taskId/stagingId，不接受模型自行填写这两个字段。工具将文件复制至本任务 staging/files，生成 install.json、文件清单和 mod.zip。打包使用通用 PowerShell Compress-Archive 命令，其真实退出码也会登记。原生仅在成功工具结束事件登记版本，失败或旧任务申请不生成版本。

未变化的静态资源允许复用；必要编译输出应放入本任务的新目录，并包含本次编译成功 commandId。源码已写入但依赖缺失时只报告源码状态，不声称存在可安装 DLL。
