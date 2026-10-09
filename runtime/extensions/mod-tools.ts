import type { ExtensionAPI } from "@earendil-works/pi-coding-agent";
import { Type } from "typebox";
import { runCommand, context, commandEnvironment } from "./commands.js";
import { prepareDelivery } from "./delivery.js";
import { lstat } from "node:fs/promises";
import { dirname, resolve } from "node:path";
import { contained } from "./commands.js";

/** 应用唯一显式加载的 extension：命令执行与文件交付，无游戏分支。 */
export default function modTools(pi: ExtensionAPI): void {
  // taskId 只隔离本轮命令与交付；同一个 source 的开发成果必须持续复用。
  pi.on("before_agent_start", async (event) => {
    const available = await commandEnvironment();
    const shellInstructions = process.platform === "win32" ? `\n命令环境是 Windows，run_command 直接启动程序（shell:false），不会解释 shell 字符串。${available.shell ? `需要 shell 时统一使用已探测的 ${available.shell}，参数使用 -NoProfile -NonInteractive -ExecutionPolicy Bypass -Command 或 -File；不要反复尝试其他 shell。` : '未找到 PowerShell；不要猜测可执行程序路径。'} dotnet：${available.dotnet ?? '未检测到'}。读取文件、列目录、搜索优先使用 read/ls/grep/find，不把 pwd/ls 当成独立可执行程序。调用 skill 构建脚本时使用原始脚本绝对路径，避免包装脚本导致编译记录不能登记。命令子进程 PATH 已补充找到的 shell 与 dotnet 目录。` : "";
    return {
    systemPrompt: `${event.systemPrompt}\n\n当前项目是持续开发的工作区：sourceDirectory 在所有轮次中保持不变。先检查已有源码、工程文件和历史进度，在其基础上增量修改。只有项目尚无源码时才初始化游戏 skill 模板，禁止每轮重新复制模板覆盖已有实现。用户询问或补充环境信息时不要重新生成文件；“继续”表示接着尚未完成的步骤执行。新 taskId/stagingId 仅隔离本轮日志和编译交付，不代表新项目。保留既有源码及成功版本；重新交付仍需本轮真实编译记录。当前 sourceDirectory：${context().sourceDirectory}${shellInstructions}`,
    };
  });
  pi.on("tool_call", async (event) => {
    if (event.toolName !== "write" && event.toolName !== "edit") { return; }
    try {
      const task = context();
      const input = event.input as Record<string, unknown>;
      if (typeof input.path !== "string") { throw new Error("文件工具缺少路径"); }
      let path = resolve(task.sourceDirectory, input.path);
      while (true) {
        try { await lstat(path); break; }
        catch (error) { if ((error as NodeJS.ErrnoException).code !== "ENOENT") { throw error; } const parent = dirname(path); if (parent === path) { throw error; } path = parent; }
      }
      await contained(task.sourceDirectory, path);
      if ((await lstat(path)).isSymbolicLink()) { throw new Error("不能写入链接文件"); }
    } catch (error) { return { block: true, reason: String(error) }; }
  });
  pi.registerTool({
    name: "run_command", label: "执行命令", description: "在当前 MOD source 内直接执行程序和参数数组（shell:false），返回真实退出码与完整日志。Windows shell 命令统一使用环境说明中的 PowerShell 路径及 -Command/-File 参数。禁止通过命令安装 MOD。", executionMode: "sequential",
    parameters: Type.Object({ program: Type.String(), args: Type.Array(Type.String()), cwd: Type.Optional(Type.String()) }),
    async execute(id, parameters, signal) {
      const receipt = await runCommand(id, parameters.program, parameters.args, parameters.cwd ?? ".", signal);
      return { content: [{ type: "text", text: JSON.stringify(receipt) }], details: receipt };
    },
  });
  pi.registerTool({
    name: "prepare_delivery", label: "准备交付", description: "复制当前任务文件并打包 ZIP；原生校验成功后才登记不可变版本。taskId/stagingId 由应用绑定。", executionMode: "sequential",
    parameters: Type.Object({ files: Type.Array(Type.Object({ source: Type.String(), target: Type.String() }), { minItems: 1 }), commandIds: Type.Array(Type.String(), { minItems: 1 }), targetDirectoryHint: Type.String() }),
    async execute(id, parameters, signal, _onUpdate, ctx) {
      context();
      const delivery = await prepareDelivery(id, parameters, signal);
      const reply = await ctx.ui.input("gen-mod:register-delivery", JSON.stringify({ toolCallId: id, delivery }), { timeout: 30_000 });
      if (!reply) { throw new Error("未取得原生版本登记结果；不能声称交付成功，请查看运行日志。ZIP 暂存在本轮 staging 目录。"); }
      const registered = JSON.parse(reply) as { versionId?: string; error?: string };
      if (registered.error || !registered.versionId) { throw new Error(`ZIP 已生成，但版本登记未成功：${registered.error || '缺少版本 ID'}。不能声称已交付。`); }
      const result = { ...delivery, versionId: registered.versionId };
      return { content: [{ type: "text", text: JSON.stringify(result) }], details: result };
    },
  });
}
