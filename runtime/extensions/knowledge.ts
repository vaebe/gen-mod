import type { ExtensionAPI } from "@earendil-works/pi-coding-agent";
import { Type } from "typebox";
import { taskRequest } from "./task-request.js";
export function registerKnowledgeTools(pi: ExtensionAPI): void {
  pi.registerTool({
    name: "search_game_knowledge", label: "查找长期记忆", description: "按当前游戏、版本和本轮关键词检索长期记忆及资料索引；返回实际使用的条目版本。记忆只作参考，未知版本或过期资料必须核对，不读取其他项目私人上下文。",
    parameters: Type.Object({ terms: Type.Array(Type.String(), { maxItems: 10 }) }),
    async execute(id, input, _signal, _update, ctx) {
      const data = await taskRequest(ctx, "gen-mod:search-knowledge", id, input);
      return { content: [{ type: "text", text: JSON.stringify(data) }], details: data };
    },
  });
  pi.registerTool({
    name: "save_long_term_memory", label: "保存长期记忆", description: "自动保存当前游戏中值得复用且有真实来源的短知识，成功回执才可提示已记住。topicKey 是跨任务稳定主题，不随来源改变；更新需 noteId/expectedRevision。user_managed 表示用户已维护或撤回，不得换主题或改写旧内容重存。无版本依据使用 unknown，不修改 Skills。",
    parameters: Type.Object({
      noteId: Type.Optional(Type.String()), expectedRevision: Type.Optional(Type.Integer({ minimum: 1 })),
      topicKey: Type.String({ minLength: 1, maxLength: 200 }),
      versionScope: Type.Union([Type.Literal("exact"), Type.Literal("independent"), Type.Literal("unknown")]), gameVersion: Type.Optional(Type.String()),
      title: Type.String({ minLength: 1, maxLength: 120 }), content: Type.String({ minLength: 1, maxLength: 2000 }),
      evidenceKind: Type.Union([Type.Literal("reference"), Type.Literal("user_observation"), Type.Literal("development")]),
      sourceRefs: Type.Array(Type.Object({ kind: Type.Union([Type.Literal("verification"), Type.Literal("reference"), Type.Literal("command"), Type.Literal("user_message")]),
        id: Type.String(), taskId: Type.Optional(Type.String()), sessionId: Type.Optional(Type.String()), sha256: Type.Optional(Type.String()) }), { minItems: 1, maxItems: 20 }),
    }),
    async execute(id, input, _signal, _update, ctx) {
      const data = await taskRequest(ctx, "gen-mod:save-long-term-memory", id, { input });
      return { content: [{ type: "text", text: JSON.stringify(data) }], details: data };
    },
  });
}
