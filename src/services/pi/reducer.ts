import type { PiRecord } from './types'
import type { MemorySaveResult } from '@/contracts/knowledge'
import type { ProjectMemoryUpdateResult } from '@/contracts/project-context'
import type { ChatMessage, MessagePart, ToolRun } from '@/contracts/session'

export interface ChatState {
  messages: ChatMessage[]
  tools: Record<string, ToolRun>
  liveMessageId?: string
  /** 尚未由 pi 用户消息确认的本地提交占位，不写入会话历史。 */
  pendingUserId?: string
  pendingInputs?: string[]
}
export function emptyChat(): ChatState {
  return { messages: [], tools: {} }
}
function object(value: unknown): Record<string, unknown> {
  return value && typeof value === 'object' ? value as Record<string, unknown> : {}
}

// 只有原生工具的完成回执能生成管理入口，助手正文和流式片段不参与。
function memoryReceipt(value: unknown): MemorySaveResult | undefined {
  const item = object(value)
  if (!['saved', 'unchanged', 'user_managed'].includes(String(item.outcome)) || typeof item.noteId !== 'string' || !item.noteId || !Number.isSafeInteger(item.revision) || Number(item.revision) < 1 || typeof item.title !== 'string') {
    return
  }
  return { outcome: item.outcome as MemorySaveResult['outcome'], noteId: item.noteId, revision: Number(item.revision), title: item.title }
}
function projectMemoryReceipt(value: unknown): ProjectMemoryUpdateResult | undefined {
  const item = object(value)
  if (!['saved', 'unchanged'].includes(String(item.outcome)) || typeof item.modId !== 'string' || !item.modId || !Number.isSafeInteger(item.revision) || Number(item.revision) < 0 || !Array.isArray(item.changedFields) || !item.changedFields.every(field => ['gameVersion', 'prerequisites', 'confirmedRequirements'].includes(field))) {
    return
  }
  return { outcome: item.outcome as ProjectMemoryUpdateResult['outcome'], modId: item.modId, revision: Number(item.revision), changedFields: item.changedFields }
}
function toolResult(output: unknown, isError: unknown): Pick<ToolRun, 'output' | 'status' | 'memorySave' | 'projectMemoryUpdate'> {
  const details = object(object(output).details)
  const status = isError || details.status === 'failed' ? 'failed' : details.status === 'cancelled' ? 'cancelled' : 'succeeded'
  return { output, status, memorySave: status === 'succeeded' ? memoryReceipt(details.memorySave) : undefined, projectMemoryUpdate: status === 'succeeded' ? projectMemoryReceipt(details.projectMemoryUpdate) : undefined }
}
function historyOutput(message: Record<string, unknown>): unknown {
  return message.details && typeof message.details === 'object' ? { content: message.content, details: message.details } : message.content
}

/** message_update 携带权威的累计消息；替换内容而非重复追加累计正文。 */
function normalize(state: ChatState, raw: unknown, id: string, final: boolean): ChatMessage | undefined {
  const message = object(raw)
  if (message.role !== 'user' && message.role !== 'assistant') {
    return
  }
  const parts: MessagePart[] = []
  const content = typeof message.content === 'string' ? [{ type: 'text', text: message.content }] : Array.isArray(message.content) ? message.content : []
  for (const value of content) {
    const part = object(value)
    if (part.type === 'text' && typeof part.text === 'string') {
      parts.push({ type: 'text', text: part.text })
    }
    else if (part.type === 'image' && typeof part.data === 'string' && ['image/png', 'image/jpeg', 'image/webp', 'image/gif'].includes(String(part.mimeType))) {
      parts.push({ type: 'image', data: part.data, mimeType: String(part.mimeType) })
    }
    else if (part.type === 'thinking' && typeof part.thinking === 'string') {
      parts.push({ type: 'thinking', text: part.thinking })
    }
    else if (part.type === 'toolCall' && typeof part.id === 'string') {
      parts.push({ type: 'tool', toolCallId: part.id })
      const previous = state.tools[part.id]
      state.tools[part.id] = previous ?? { toolCallId: part.id, name: String(part.name ?? '工具'), args: part.arguments, status: 'waiting' }
    }
  }
  return { id, inputId: typeof message.appInputId === 'string' ? message.appInputId : undefined, taskId: typeof message.appTaskId === 'string' ? message.appTaskId : undefined, entryId: typeof message.appEntryId === 'string' ? message.appEntryId : undefined, role: message.role, parts, final, error: typeof message.errorMessage === 'string' ? message.errorMessage : undefined, timestamp: typeof message.timestamp === 'number' ? message.timestamp : undefined, stopReason: typeof message.stopReason === 'string' ? message.stopReason : undefined }
}

export function reduceRecord(state: ChatState, record: PiRecord): ChatState {
  const next: ChatState = { ...state, messages: [...state.messages], tools: { ...state.tools } }
  if (record.type === 'message_start' || record.type === 'message_update' || record.type === 'message_end') {
    const message = object(record.message)
    if (message.role === 'toolResult') {
      const id = String(message.toolCallId ?? '')
      const previous = next.tools[id]
      if (previous) {
        next.tools[id] = { ...previous, ...toolResult(historyOutput(message), message.isError || previous.status === 'failed' || previous.status === 'cancelled') }
      }
      return next
    }
    if (message.role !== 'user' && message.role !== 'assistant') {
      return next
    }
    const confirmedId = typeof record.inputId === 'string' ? record.inputId : undefined
    const stableId = typeof message.timestamp === 'number' ? `${message.role}-${message.timestamp}` : undefined
    const restored = stableId ? next.messages.find(item => item.role === message.role && item.timestamp === message.timestamp) : undefined
    const id = restored?.id ?? (message.role === 'user' && confirmedId ? confirmedId : message.role === 'user' && next.pendingUserId ? next.pendingUserId : stableId ?? (record.type === 'message_start' || !next.liveMessageId ? crypto.randomUUID() : next.liveMessageId))
    const normalized = normalize(next, message, id, record.type === 'message_end')
    if (normalized) {
      normalized.inputId = confirmedId
      normalized.taskId = typeof record.taskId === 'string' ? record.taskId : undefined
      const update = object(record.assistantMessageEvent)
      if (message.role === 'assistant' && record.type === 'message_update' && (update.type === 'thinking_start' || update.type === 'thinking_delta') && typeof update.contentIndex === 'number' && Array.isArray(message.content)) {
        // 使用原始内容索引，只展示当前段，不重复展示之前已经结束的思考。
        const part = object(message.content[update.contentIndex])
        if (part.type === 'thinking' && typeof part.thinking === 'string') {
          normalized.activeThinking = part.thinking
        }
      }
      const index = next.messages.findIndex(item => item.id === id)
      if (index >= 0) {
        next.messages[index] = normalized
      }
      else { next.messages.push(normalized) }
      if (message.role === 'user') {
        if (confirmedId) {
          next.pendingInputs = next.pendingInputs?.filter(value => value !== confirmedId)
        }
        else { next.pendingUserId = undefined }
      }
      next.liveMessageId = record.type === 'message_end' ? undefined : id
    }
  }
  else if (record.type.startsWith('tool_execution_')) {
    if (record.type === 'tool_execution_start') {
      next.messages = next.messages.map(message => message.activeThinking !== undefined ? { ...message, activeThinking: undefined } : message)
    }
    const id = String(record.toolCallId ?? '')
    if (!id) {
      return next
    }
    const previous = next.tools[id]
    next.tools[id] = {
      toolCallId: id,
      name: String(record.toolName ?? previous?.name ?? '工具'),
      args: record.args ?? previous?.args,
      ...(record.type === 'tool_execution_end' ? toolResult(record.result, record.isError) : { output: record.partialResult ?? previous?.output, status: 'running' }),
    }
  }
  else if (record.type === 'agent_settled') {
    next.liveMessageId = undefined
    next.messages = next.messages.map(message => message.activeThinking !== undefined ? { ...message, activeThinking: undefined } : message)
    for (const [id, tool] of Object.entries(next.tools)) {
      if (tool.status === 'running' || tool.status === 'waiting') {
        next.tools[id] = { ...tool, status: record.aborted ? 'cancelled' : 'failed' }
      }
    }
  }
  return next
}

export function restoreChat(messages: unknown[]): ChatState {
  const state = emptyChat()
  messages.forEach((raw, index) => {
    const message = object(raw)
    if (message.role === 'toolResult') {
      const id = String(message.toolCallId ?? '')
      state.tools[id] = { toolCallId: id, name: String(message.toolName ?? '工具'), args: state.tools[id]?.args, ...toolResult(historyOutput(message), message.isError) }
    }
    else {
      const id = typeof message.appInputId === 'string' ? message.appInputId : typeof message.timestamp === 'number' ? `${message.role}-${message.timestamp}` : `history-${index}`
      const item = normalize(state, raw, id, true)
      if (item) {
        state.messages.push(item)
      }
    }
  })
  return state
}
