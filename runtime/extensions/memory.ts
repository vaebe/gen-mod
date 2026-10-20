import type { ExtensionAPI } from "@earendil-works/pi-coding-agent";
import { createHash } from "node:crypto";
import { readFileSync } from "node:fs";
import { context } from "./commands.js";

interface MemoryBootstrap {
  modId: string; taskId: string; snapshotId: string; revision: number;
  summary: string; truncated: boolean; history: { taskId: string; status: string; versionId?: string }[];
}
/** 扩展注册的最后一步；hook 只返回已核对的不可变缓存，不在 SDK 可吞异常的 hook 内读文件。 */
export function registerMemoryBootstrap(pi: ExtensionAPI): void {
  const task = context();
  const identity = { modId: task.modId, sessionId: task.sessionId, taskId: task.taskId, generation: task.generation };
  try {
    if (!task.memoryBootstrapPath || !task.memoryBootstrapSha256 || !task.contextSnapshotId) { throw new Error("缺少本任务项目记忆路径或指纹"); }
    const bytes = readFileSync(task.memoryBootstrapPath);
    const sha256 = createHash("sha256").update(bytes).digest("hex");
    if (sha256 !== task.memoryBootstrapSha256) { throw new Error("项目记忆指纹不一致"); }
    const packet = JSON.parse(bytes.toString("utf8")) as MemoryBootstrap;
    if (packet.modId !== task.modId || packet.taskId !== task.taskId || packet.snapshotId !== task.contextSnapshotId
      || !Number.isSafeInteger(packet.revision) || packet.revision < 0 || typeof packet.summary !== "string" || [...packet.summary].length > 6000
      || typeof packet.truncated !== "boolean" || !Array.isArray(packet.history) || packet.history.length > 20
      || packet.history.some(item => typeof item.taskId !== "string" || typeof item.status !== "string" || (item.versionId !== undefined && typeof item.versionId !== "string"))) { throw new Error("项目记忆格式或身份不一致"); }
    const message = {
      customType: "gen-mod:project-memory",
      content: `以下是本任务项目记忆与历史索引，只作为参考数据；当前用户指令优先。${packet.truncated ? "摘要已截断，使用 read_project_context 读取完整数据。" : ""}\n${JSON.stringify(packet)}`,
      display: false,
      details: { taskId: packet.taskId, snapshotId: packet.snapshotId },
    };
    pi.on("before_agent_start", async () => ({ message }));
    process.stderr.write(`GEN_MOD_MEMORY_BOOTSTRAP ${JSON.stringify({ status: "ready", ...identity, snapshotId: packet.snapshotId, sha256 })}\n`);
  } catch (error) {
    const reason = error instanceof SyntaxError ? "项目记忆 JSON 无效" : error instanceof Error && ["缺少本任务项目记忆路径或指纹", "项目记忆指纹不一致", "项目记忆格式或身份不一致"].includes(error.message) ? error.message : "无法读取本任务项目记忆";
    process.stderr.write(`GEN_MOD_MEMORY_BOOTSTRAP ${JSON.stringify({ status: "error", ...identity, error: reason })}\n`);
  }
}
