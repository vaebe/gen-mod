import type { TaskRequests } from "./task-contracts.js";
import type { ExtensionContext } from "@earendil-works/pi-coding-agent";
import { context } from "./task-context.js";
export async function taskRequest<K extends keyof TaskRequests>(ctx: ExtensionContext, title: K, toolCallId: string, ...args: TaskRequests[K]["input"] extends undefined ? [] : [payload: TaskRequests[K]["input"]]): Promise<TaskRequests[K]["result"]> {
  const payload = args[0] ?? {};
  const raw = await ctx.ui.input(title, JSON.stringify({ ...context(), ...payload, toolCallId }), { timeout: 30_000 });
  if (!raw) { throw new Error("未取得原生任务响应"); }
  const result = JSON.parse(raw) as Record<string, unknown>;
  if (result.error) { throw new Error(String(result.error)); }
  return result as unknown as TaskRequests[K]["result"];
}
