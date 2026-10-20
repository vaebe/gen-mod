import type { ExtensionAPI } from "@earendil-works/pi-coding-agent";
import { Type } from "typebox";
import { taskRequest } from "./task-request.js";
const result = Type.Union(["passed", "partial", "failed", "unknown"].map(value => Type.Literal(value)));
export const verificationCaseSchema = Type.Object({ id: Type.String(), precondition: Type.String(), action: Type.String(), expected: Type.String(), evidenceHint: Type.String() });
export function registerVerificationTools(pi: ExtensionAPI): void {
  pi.registerTool({
    name: "read_feedback_sources", label: "读取玩家反馈来源", description: "读取当前会话最近玩家消息的摘要与原生指纹，并列出本项目真实交付版本。版本不明时询问玩家，不默认取最新版。",
    parameters: Type.Object({}),
    async execute(id, _input, _signal, _update, ctx) {
      const data = await taskRequest(ctx, "gen-mod:read-feedback-sources", id);
      return { content: [{ type: "text", text: JSON.stringify(data) }], details: data };
    },
  });
  pi.registerTool({
    name: "propose_verification", label: "整理验收草稿", description: "将玩家反馈整理为指定交付版本的候选记录，sourceRefs 必须来自读取工具。不能确认或伪造玩家结果，正式记录由玩家在界面确认。",
    parameters: Type.Object({
      versionId: Type.String(), gameVersion: Type.Optional(Type.String()), observations: Type.String(), result,
      caseResults: Type.Array(Type.Object({ caseId: Type.String(), result, observed: Type.String() })),
      sourceRefs: Type.Array(Type.Object({ sessionId: Type.String(), entryId: Type.String(), attachmentIndex: Type.Optional(Type.Integer({ minimum: 0 })), sha256: Type.String() })),
    }),
    async execute(id, input, _signal, _update, ctx) {
      const data = await taskRequest(ctx, "gen-mod:propose-verification", id, { payload: input });
      return { content: [{ type: "text", text: JSON.stringify(data) }], details: data };
    },
  });
}
