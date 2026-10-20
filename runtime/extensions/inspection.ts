import type { ExtensionAPI } from "@earendil-works/pi-coding-agent";
import { Type } from "typebox";
import { taskRequest } from "./task-request.js";
export function registerInspectionTools(pi: ExtensionAPI): void {
  pi.registerTool({
    name: "read_game_inspection", label: "读取环境调查报告", description: "只读取当前项目已有的调查报告，不发起目录扫描，不接受外部目录。未填写 reportId 时列出最近报告。",
    parameters: Type.Object({ reportId: Type.Optional(Type.String()) }),
    async execute(id, input, _signal, _update, ctx) {
      const data = await taskRequest(ctx, "gen-mod:read-game-inspection", id, input);
      return { content: [{ type: "text", text: JSON.stringify(data) }], details: data };
    },
  });
}
