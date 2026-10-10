import type { ChatMessage, MessagePart, ToolRun } from "@/contracts/session";
import type { PiRecord } from "./types";

export interface ChatState {
  messages: ChatMessage[];
  tools: Record<string, ToolRun>;
  liveMessageId?: string;
  /** 尚未由 pi 用户消息确认的本地提交占位，不写入会话历史。 */
  pendingUserId?: string;
  pendingInputs?: string[];
}
export function emptyChat(): ChatState { return { messages: [], tools: {} }; }
function object(value: unknown): Record<string, unknown> {
  return value && typeof value === "object" ? value as Record<string, unknown> : {};
}

/** message_update 携带权威的累计消息；替换内容而非重复追加累计正文。 */
function normalize(state: ChatState, raw: unknown, id: string, final: boolean): ChatMessage | undefined {
  const message = object(raw);
  if (message.role !== "user" && message.role !== "assistant") { return; }
  const parts: MessagePart[] = [];
  const content = typeof message.content === "string" ? [{ type: "text", text: message.content }] : Array.isArray(message.content) ? message.content : [];
  for (const value of content) {
    const part = object(value);
    if (part.type === "text" && typeof part.text === "string") { parts.push({ type: "text", text: part.text }); }
    else if (part.type === "image" && typeof part.data === "string" && ["image/png", "image/jpeg", "image/webp", "image/gif"].includes(String(part.mimeType))) { parts.push({ type: "image", data: part.data, mimeType: String(part.mimeType) }); }
    else if (part.type === "thinking" && typeof part.thinking === "string") { parts.push({ type: "thinking", text: part.thinking }); }
    else if (part.type === "toolCall" && typeof part.id === "string") {
      parts.push({ type: "tool", toolCallId: part.id });
      const previous = state.tools[part.id];
      state.tools[part.id] = previous ?? { toolCallId: part.id, name: String(part.name ?? "工具"), args: part.arguments, status: "waiting" };
    }
  }
  return { id, entryId: typeof message.appEntryId === "string" ? message.appEntryId : undefined, role: message.role, parts, final, error: typeof message.errorMessage === "string" ? message.errorMessage : undefined,
    timestamp: typeof message.timestamp === "number" ? message.timestamp : undefined,
    stopReason: typeof message.stopReason === "string" ? message.stopReason : undefined };
}

export function reduceRecord(state: ChatState, record: PiRecord): ChatState {
  const next: ChatState = { ...state, messages: [...state.messages], tools: { ...state.tools } };
  if (record.type === "message_start" || record.type === "message_update" || record.type === "message_end") {
    const message = object(record.message);
    if (message.role === "toolResult") {
      const id = String(message.toolCallId ?? "");
      const previous = next.tools[id];
      if (previous) { next.tools[id] = { ...previous, output: message.content, status: message.isError ? "failed" : previous.status === "failed" ? "failed" : "succeeded" }; }
      return next;
    }
    if (message.role !== "user" && message.role !== "assistant") { return next; }
    const confirmedId = typeof record.inputId === "string" ? record.inputId : undefined;
    const id = message.role === "user" && confirmedId ? confirmedId : message.role === "user" && next.pendingUserId ? next.pendingUserId : record.type === "message_start" || !next.liveMessageId ? crypto.randomUUID() : next.liveMessageId;
    const normalized = normalize(next, message, id, record.type === "message_end");
    if (normalized) {
      normalized.inputId = confirmedId;
      normalized.taskId = typeof record.taskId === "string" ? record.taskId : undefined;
      const index = next.messages.findIndex((item) => item.id === id);
      if (index >= 0) { next.messages[index] = normalized; } else { next.messages.push(normalized); }
      if (message.role === "user") { if (confirmedId) { next.pendingInputs = next.pendingInputs?.filter(value => value !== confirmedId); } else { next.pendingUserId = undefined; } }
      next.liveMessageId = record.type === "message_end" ? undefined : id;
    }
  } else if (record.type.startsWith("tool_execution_")) {
    const id = String(record.toolCallId ?? "");
    if (!id) { return next; }
    const previous = next.tools[id];
    next.tools[id] = {
      toolCallId: id, name: String(record.toolName ?? previous?.name ?? "工具"),
      args: record.args ?? previous?.args,
      output: record.result ?? record.partialResult ?? previous?.output,
      status: record.type === "tool_execution_end" ? record.isError || object(object(record.result).details).status === "failed" ? "failed" : object(object(record.result).details).status === "cancelled" ? "cancelled" : "succeeded" : "running",
    };
  } else if (record.type === "agent_settled") {
    next.liveMessageId = undefined;
    for (const [id, tool] of Object.entries(next.tools)) {
      if (tool.status === "running" || tool.status === "waiting") {
        next.tools[id] = { ...tool, status: record.aborted ? "cancelled" : "failed" };
      }
    }
  }
  return next;
}

export function restoreChat(messages: unknown[]): ChatState {
  const state = emptyChat();
  messages.forEach((raw, index) => {
    const message = object(raw);
    if (message.role === "toolResult") {
      const id = String(message.toolCallId ?? "");
      state.tools[id] = { toolCallId: id, name: String(message.toolName ?? "工具"), args: state.tools[id]?.args, output: message.content, status: message.isError ? "failed" : "succeeded" };
    } else {
      const item = normalize(state, raw, `history-${index}-${String(message.timestamp ?? 0)}`, true);
      if (item) { state.messages.push(item); }
    }
  });
  return state;
}
