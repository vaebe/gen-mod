import type { TaskRequests } from "./task-contracts.js";
import type { ExtensionContext } from "@earendil-works/pi-coding-agent";
import type { DeliveryInput } from "./delivery.js";
import { context } from "./task-context.js";
export async function checkDelivery(ctx: ExtensionContext, id: string, input: DeliveryInput): Promise<void> {
  const raw = await ctx.ui.input("gen-mod:check-delivery", JSON.stringify({ ...context(), toolCallId: id, files: input.files }), { timeout: 30_000 });
  if (!raw) { throw new Error("未取得原生交付预检查结果，尚未创建 staging/files"); }
  const result = JSON.parse(raw) as TaskRequests["gen-mod:check-delivery"]["result"] & { error?: string };
  if (result.error) { throw new Error(result.error); }
  if (result.blocked) { throw new Error(`交付内容需修正：${JSON.stringify(result.checkReport)}`); }
}
