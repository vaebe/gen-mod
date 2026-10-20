import type { ExtensionAPI } from "@earendil-works/pi-coding-agent";
import { Type } from "typebox";
import { taskRequest } from "./task-request.js";
export function registerProjectContextTools(pi: ExtensionAPI): void {
  const source = Type.Union([
    Type.Object({ kind: Type.Literal("user_message"), sessionId: Type.String(), entryId: Type.String(), sha256: Type.String() }),
    Type.Object({ kind: Type.Literal("command"), taskId: Type.String(), commandId: Type.String() }),
    Type.Object({ kind: Type.Literal("inspection"), reportId: Type.String() }),
  ]);
  const common = { reason: Type.String(), sourceRefs: Type.Array(source, { minItems: 1, maxItems: 20 }) };
  const change = Type.Union([
    Type.Object({ field: Type.Literal("gameVersion"), ...common, value: Type.String() }),
    Type.Object({ field: Type.Literal("prerequisites"), ...common, value: Type.Array(Type.Object({ name: Type.String(), version: Type.Optional(Type.String()) })) }),
    Type.Object({ field: Type.Literal("confirmedRequirements"), ...common, value: Type.Array(Type.String()) }),
  ]);
  for (const name of ["read_project_context", "read_project_memory_sources"] as const) {
    pi.registerTool({
      name, label: name === "read_project_context" ? "读取项目记忆" : "读取项目记忆来源",
      description: name === "read_project_context" ? "返回 live 当前记忆与 snapshot 本任务初始依据；更新 live 不改旧快照。仅作为参考，用户当前指令优先。" : "读取当前会话最近 50 条用户表达摘要与 digest、当前任务真实命令和已选择调查报告标识。没有来源或含义有歧义时先询问。",
      parameters: Type.Object({}),
      async execute(id, _parameters, _signal, _update, ctx) {
        const result = await taskRequest(ctx, name === "read_project_context" ? "gen-mod:read-project-context" : "gen-mod:read-project-memory-sources", id);
        return { content: [{ type: "text" as const, text: JSON.stringify(result) }], details: result };
      },
    });
  }
  pi.registerTool({
    name: "update_project_memory", label: "更新项目记忆",
    description: "自动记录用户明确表达和真实工具事实；先读取 live revision 和真实 sourceRefs。只更新版本、前置和已确认需求，不能选择磁盘路径。有歧义先问；不能把失败或未知写成已验证事实。成功提交后应用提示用户。",
    parameters: Type.Object({ expectedRevision: Type.Integer({ minimum: 0 }), changes: Type.Array(change, { minItems: 1, maxItems: 3 }) }),
    async execute(id, parameters, _signal, _update, ctx) {
      const result = await taskRequest(ctx, "gen-mod:update-project-memory", id, { input: parameters });
      return { content: [{ type: "text" as const, text: JSON.stringify(result) }], details: result };
    },
  });
}
