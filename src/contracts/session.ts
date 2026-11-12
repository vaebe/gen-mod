import type { SkillSnapshot } from './skill'
import type { MemorySaveResult } from '@/features/knowledge/types'
import type { ProjectMemoryUpdateResult } from '@/features/project-memory/types'

/** pi 会话文件是历史权威；此处只保存关联信息。 */
export interface SessionReference {
  sessionId: string
  sessionFile: string
  skills: SkillSnapshot[]
  readOnly: boolean
  /** 空名称显示“新会话”；首轮完成后生成标题。 */
  name: string
}

export type MessagePart
  = | { type: 'text', text: string }
    | ImageInput
    | { type: 'thinking', text: string }
    | { type: 'tool', toolCallId: string }

/** pi RPC 图片内容；data 为无 data URL 前缀的 Base64。 */
export interface ImageInput { type: 'image', data: string, mimeType: string }

/** 尚未发送的本地附件；文本文件只读取 UTF-8 正文。 */
export type PromptAttachment = { id: string, name: string, size: number } & (
  | { type: 'image', data: string, mimeType: string }
  | { type: 'text', text: string }
)

export interface ChatMessage {
  taskId?: string
  inputId?: string
  inputStatus?: string
  entryId?: string
  id: string
  role: 'user' | 'assistant'
  parts: MessagePart[]
  /** 仅用于实时展示当前思考段；结束事件和历史恢复不保留此状态。 */
  activeThinking?: string
  final: boolean
  error?: string
  timestamp?: number
  stopReason?: string
}

export type ThinkingLevel = 'off' | 'minimal' | 'low' | 'medium' | 'high' | 'xhigh' | 'max'

/** pi 当前窗口估算，不是整个会话累计计费 token。 */
export interface ContextUsage {
  sampledAt?: string
  estimated?: boolean
  generation?: number
  historical?: boolean
  tokens: number | null
  contextWindow: number
  percent: number | null
}

export interface ToolRun {
  toolCallId: string
  name: string
  args: unknown
  output?: unknown
  memorySave?: MemorySaveResult
  projectMemoryUpdate?: ProjectMemoryUpdateResult
  status: 'waiting' | 'running' | 'succeeded' | 'failed' | 'cancelled'
}

/** 仅应用引导关联；正式历史仍来自 pi JSONL。 */
export interface SteeringInput { inputId: string, taskId: string, status: string, message: string, submittedAt: string, messageTimestamp?: number, entryId?: string }

/** 操作发起时捕获的身份，异步结果始终归此会话。 */
export interface SessionIdentity { modId: string, sessionId: string }

export interface SessionSummary {
  readonly initialized: boolean
  readonly busy: boolean
  readonly running: boolean
  readonly activity: string
}

export interface RuntimeLogEntry { timestamp: string, generation: number, taskId?: string, message: string }
