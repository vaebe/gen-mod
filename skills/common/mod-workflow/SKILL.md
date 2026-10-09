---
name: mod-workflow
description: 开发游戏 MOD 时使用的通用命令、文件交付与版本规范
---

# 通用 MOD 开发

先确认用户需求和交付范围，读取所选游戏 skill 的资料与模板，然后在当前 source 工作目录实现。不要要求安装游戏或声称已在游戏中验证；运行效果由用户验证。缺少编译依赖时保留源码，列出具体缺项，不伪造程序集和编译结果。

文件读取与修改使用文件工具，执行命令只使用 `run_command`。程序名与参数数组分开；PowerShell 脚本以 `powershell.exe` 的 `-NoProfile -NonInteractive -ExecutionPolicy Bypass -File` 参数调用，路径作为独立参数。记录真实成功命令的 commandId。必要编译由游戏 skill 规定，不提供手动构建步骤。

在本次任务内使用全新的输出目录，如 `.delivery/<taskId>/`；读取 `GEN_MOD_TASK_CONTEXT` 可取得当前任务 ID。清除或复用以前编译的 DLL 不能证明当前任务编译成功。成功编译或整理交付文件后调用 `prepare_delivery`，提交相对 source 的文件列表、相对安装目标的映射、成功 commandIds 和目录提示。ZIP 由交付工具通过真实命令创建，原生负责核对与登记。

参考[交付协议](references/delivery.md)。不要自行写入 artifacts、改历史版本或执行安装；安装由用户在版本管理页点击确认。最终回复概述源码变化、版本登记结果、必要依赖和未执行的验证。
