import type { RuntimeLogEntry } from '@/contracts/session'
import { useIntervalFn } from '@vueuse/core'
import { readonly, shallowRef, watch } from 'vue'
import { sessionService } from '@/features/sessions/service'
import { useSessionStore } from '@/features/sessions/useSessionStore'

/** 日志属于会话能力；工作台只决定是否可见。 */
export function useSessionLogs(visible: () => boolean) {
  const session = useSessionStore()
  const entries = shallowRef<RuntimeLogEntry[]>([])
  const error = shallowRef('')
  let generation = 0
  let reading = false
  async function refresh(): Promise<void> {
    if (reading || !visible() || !session.modId || !session.sessionId) {
      return
    }
    reading = true
    const epoch = generation
    const target = { modId: session.modId, sessionId: session.sessionId }
    try {
      const result = await sessionService.runtimeLogs(target)
      if (epoch === generation && session.isSelected(target)) {
        entries.value = result
        error.value = ''
      }
    }
    catch (cause) {
      if (epoch === generation && session.isSelected(target)) {
        error.value = (cause as Error).message
      }
    }
    finally {
      if (epoch === generation) {
        reading = false
      }
    }
  }
  async function openTask(): Promise<void> {
    const target = { modId: session.modId, sessionId: session.sessionId }
    const taskId = session.task?.taskId
    if (!taskId) {
      return
    }
    try {
      await sessionService.openTaskLogs({ modId: target.modId, taskId })
    }
    catch (cause) {
      if (session.isSelected(target)) {
        error.value = (cause as Error).message
      }
    }
  }
  watch(() => [session.modId, session.sessionId], () => {
    generation += 1
    reading = false
    entries.value = []
    error.value = ''
    void refresh()
  })
  watch(visible, () => {
    void refresh()
  })
  useIntervalFn(() => {
    void refresh()
  }, 3000)
  return { entries: readonly(entries), error: readonly(error), refresh, openTask }
}
