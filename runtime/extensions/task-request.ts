import type { ExtensionContext } from "@earendil-works/pi-coding-agent";
import { context } from "./commands.js";
export async function taskRequest(ctx: ExtensionContext, title: string, toolCallId: string, payload: Record<string, unknown> = {}): Promise<Record<string, unknown>> {
  const raw = await ctx.ui.input(title, JSON.stringify({ ...context(), ...payload, toolCallId }), { timeout: 30_000 });
  if (!raw) { throw new Error("未取得原生任务响应"); }
  const result = JSON.parse(raw) as Record<string, unknown>;
  if (result.error) { throw new Error(String(result.error)); }
  return result;
}
