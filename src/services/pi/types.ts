import type { TaskRecord } from '@/contracts/task'

/** 锁定 pi 的 RPC JSONL。内部桥事件与 pi 记录分开。 */
export interface PiRecord {
  type: string
  id?: string
  command?: string
  success?: boolean
  error?: string
  data?: Record<string, unknown>
  [key: string]: unknown
}
export interface PiEnvelope {
  sequence: number
  subscription: number
  modId: string
  sessionId: string
  taskId?: string
  generation: number
  kind: 'record' | 'diagnostic' | 'exit' | 'task' | 'artifact'
  record: PiRecord
}
export interface RuntimeSnapshot {
  modId: string
  sessionId: string
  generation: number
  sequence: number
  subscription: number
  connected: boolean
  active: boolean
  running: boolean
  stopping: boolean
  compacting: boolean
  task: TaskRecord | null
  messages: unknown[]
  tools: PiEnvelope[]
}
export interface SubmissionResult {
  status: 'unknown' | 'pending' | 'accepted' | 'rejected'
  task?: TaskRecord | null
  error?: string
}
export interface PiSessionState {
  sessionId: string
  sessionFile?: string
  isStreaming: boolean
  isCompacting: boolean
  pendingMessageCount: number
}
