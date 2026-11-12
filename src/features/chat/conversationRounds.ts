import type { DeepReadonly } from 'vue'
import type { ChatMessage } from '@/contracts/session'
import type { TaskRecord } from '@/contracts/task'

export interface ConversationRound {
  id: string
  taskId?: string
  user?: DeepReadonly<ChatMessage>
  supplements?: DeepReadonly<ChatMessage>[]
  process: DeepReadonly<ChatMessage>[]
  answer?: DeepReadonly<ChatMessage>
  startedAt?: number
  endedAt?: number
  running: boolean
  status?: string
}

/** 按用户需求归并一轮的模型/工具消息；终态正文放在过程区域外。 */
export function conversationRounds(messages: readonly DeepReadonly<ChatMessage>[], tasks: TaskRecord[], working: boolean, startedAt?: number, endedAt?: number): ConversationRound[] {
  const rounds: ConversationRound[] = []
  for (const message of messages) {
    if (message.role === 'user') {
      const previous = rounds[rounds.length - 1]
      if (message.inputId && message.taskId && previous && (previous.user?.taskId === message.taskId || tasks.some(task => task.context.taskId === message.taskId && previous.user?.timestamp !== undefined && previous.user.timestamp >= Date.parse(task.context.createdAt)))) {
        previous.supplements = [...(previous.supplements ?? []), message]
      }
      else { rounds.push({ id: message.id, user: message, process: [], running: false }) }
    }
    else {
      if (!rounds.length) {
        rounds.push({ id: message.id, process: [], running: false })
      }
      rounds[rounds.length - 1]!.process.push(message)
    }
  }
  if (working && !rounds.length) {
    rounds.push({ id: 'pending', process: [], running: true })
  }
  const previous = rounds[rounds.length - 1]
  if (working && startedAt !== undefined && previous?.user?.timestamp !== undefined && previous.user.timestamp < startedAt) {
    rounds.push({ id: 'pending', process: [], running: true })
  }
  const sorted = [...tasks].sort((a, b) => Date.parse(a.context.createdAt) - Date.parse(b.context.createdAt))
  for (const [index, round] of rounds.entries()) {
    const latest = index === rounds.length - 1
    round.running = latest && working
    const anchor = round.user?.timestamp ?? round.process[0]?.timestamp
    const anchorMessage = round.user ?? round.process[0]
    // 仅已确认的历史消息允许按时间兼容匹配，本地占位或失败消息不能借用上一轮交付。
    const candidates = anchor === undefined || !anchorMessage?.final || anchorMessage.error ? [] : sorted.filter(item => Date.parse(item.context.createdAt) <= anchor)
    const explicitTaskId = round.user?.taskId ?? round.process.find(message => message.taskId)?.taskId
    const task = explicitTaskId ? sorted.find(item => item.context.taskId === explicitTaskId) : candidates[candidates.length - 1]
    round.taskId = task?.context.taskId
    round.status = task?.status
    const live = latest && startedAt !== undefined && (anchor === undefined || anchor >= startedAt)
    round.startedAt = live ? startedAt : task ? Date.parse(task.context.createdAt) : undefined
    round.endedAt = live && endedAt !== undefined ? endedAt : task?.finishedAt ? Date.parse(task.finishedAt) : undefined
    const last = round.process[round.process.length - 1]
    if (!round.running && last && !last.error && last.stopReason === 'stop' && last.parts.some(part => part.type === 'text' && part.text.trim()) && !last.parts.some(part => part.type === 'tool')) {
      round.answer = last
      // 最后一条的思考仍归执行过程，最终正文单独显示。
      round.process = [...round.process.slice(0, -1), { ...last, parts: last.parts.filter(part => part.type !== 'text') }].filter(message => message.parts.length || message.error)
    }
  }
  return rounds
}
