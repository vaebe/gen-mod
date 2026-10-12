import type { ChatMessage } from '@/contracts/session'
import type { TaskRecord } from '@/contracts/task'

export interface ConversationRound {
  id: string
  user?: ChatMessage
  supplements?: ChatMessage[]
  process: ChatMessage[]
  answer?: ChatMessage
  startedAt?: number
  endedAt?: number
  running: boolean
  status?: string
}

/** 按用户需求归并一轮的模型/工具消息；终态正文放在过程区域外。 */
export function conversationRounds(messages: ChatMessage[], tasks: TaskRecord[], working: boolean, startedAt?: number, endedAt?: number): ConversationRound[] {
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
    const candidates = anchor === undefined ? [] : sorted.filter(item => Date.parse(item.context.createdAt) <= anchor)
    const task = candidates[candidates.length - 1]
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
