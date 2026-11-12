import type { ExtensionContext, ExtensionAPI, ToolDefinition } from "@earendil-works/pi-coding-agent";
import { createReadToolDefinition, createWriteToolDefinition, createEditToolDefinition, createGrepToolDefinition, createFindToolDefinition, createLsToolDefinition } from "@earendil-works/pi-coding-agent";
import { randomUUID } from "node:crypto";
import { relative, resolve, isAbsolute, sep } from "node:path";
import { context } from "./task-context.js";
import { runtimeDiagnostic } from "./diagnostics.js";

export interface Lease { leaseId: string; sourceRevision: number; toolCallId: string; descendantsUnconfirmed?: boolean }
async function request(ctx: ExtensionContext, payload: Record<string, unknown>): Promise<Record<string, unknown>> {
  const started = performance.now();
  runtimeDiagnostic("workspace_request", payload);
  const reply = await ctx.ui.input("gen-mod:workspace", JSON.stringify({ ...context(), ...payload }), { timeout: 30_000 });
  if (!reply) {
    runtimeDiagnostic("workspace_timeout", { ...payload, elapsedMs: Math.round(performance.now() - started) });
    const phase = payload.action === "acquire" ? "未取得租约，工具尚未开始" : payload.action === "release" ? "工具已开始，命令可能已结束；不能确认租约释放，请查看命令日志" : "未确认租约清理";
    throw new Error(`工作区协调响应超时（${payload.action}，leaseId ${payload.leaseId}，toolCallId ${payload.toolCallId}）：${phase}`);
  }
  const result = JSON.parse(reply) as Record<string, unknown>;
  runtimeDiagnostic("workspace_response", { ...payload, elapsedMs: Math.round(performance.now() - started), result });
  if (result.error) { throw new Error(String(result.error)); }
  return result;
}
/** 租约覆盖整个工具、后代命令和登记；prepare_delivery 内部命令复用传入租约。 */
export async function withLease<T>(ctx: ExtensionContext, toolCallId: string, operation: string, path: string | undefined, signal: AbortSignal | undefined, action: (lease: Lease) => Promise<T>): Promise<T> {
  const leaseId = randomUUID();
  const payload = { leaseId, toolCallId, operation, path };
  let success = false;
  let acquired = false;
  let lease: Lease | undefined;
  let value: T | undefined;
  let actionError: unknown;
  try {
    if (signal?.aborted) { throw new Error("工具已取消"); }
    const result = await request(ctx, { ...payload, action: "acquire" });
    acquired = true;
    if (signal?.aborted) { throw new Error("工具已取消"); }
    lease = { leaseId, sourceRevision: Number(result.sourceRevision), toolCallId };
    value = await action(lease);
    success = true;
    return value;
  } catch (error) {
    actionError = error;
    throw error;
  } finally {
    try {
      const released = await request(ctx, { ...payload, action: lease?.descendantsUnconfirmed ? "terminate" : acquired ? "release" : "cancel", success });
      if (operation === "run_command" && value && typeof value === "object" && "sourceRevision" in value) { value.sourceRevision = Number(released.sourceRevision); }
    } catch (error) {
      runtimeDiagnostic("workspace_cleanup_failed", { ...payload, error: String(error), actionError: actionError === undefined ? undefined : String(actionError), result: value });
      throw new Error(`${actionError === undefined ? "" : `工具执行错误：${String(actionError)}；`}${String(error)}${value === undefined ? "" : `；已产生执行结果（协调未确认）：${JSON.stringify(value)}`}`);
    }
  }
}
function sourcePath(value: string, writing: boolean): string | undefined {
  const root = context().sourceDirectory;
  const path = relative(root, resolve(root,value));
  if (isAbsolute(path) || path === ".." || path.startsWith(".." + sep)) { if (writing) { throw new Error("文件工具只能修改共享源码目录"); } return undefined; }
  return path.replaceAll("\\", "/") || ".";
}
/** 使用锁定 SDK 的原始工具实现并包装 execute，保留截断、diff 及路径校验语义。 */
export function registerFileTools(pi: ExtensionAPI): void {
  const cwd = context().sourceDirectory;
  const definitions = [createReadToolDefinition(cwd),createWriteToolDefinition(cwd),createEditToolDefinition(cwd),createGrepToolDefinition(cwd),createFindToolDefinition(cwd),createLsToolDefinition(cwd)];
  function wrap<T extends import("typebox").TSchema, D, R>(tool: ToolDefinition<T,D,R>): void {
    const original = tool.execute;
    pi.registerTool({ ...tool, async execute(id, parameters, signal, update, ctx) {
      const input = parameters as { path?: string };
      const path = sourcePath(input.path ?? ".",tool.name === "write" || tool.name === "edit");
      return withLease(ctx,id,tool.name,path,signal,async () => original(id,parameters,signal,update,ctx));
    } });
  }
  for (const tool of definitions) { wrap(tool as ToolDefinition); }
}
