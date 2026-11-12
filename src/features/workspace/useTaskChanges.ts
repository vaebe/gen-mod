import type { FileChange, FileDiff as FileDiffData, TaskFileOperation } from '@/contracts/changes'
import { onScopeDispose, reactive, shallowRef, watch } from 'vue'
import { workspaceService } from './workspaceService'

const invalidations = reactive(new Map<string, number>())
export const sourceRevision = (modId: string) => invalidations.get(modId) ?? 0
export function invalidateSource(modId: string): void {
  invalidations.set(modId, sourceRevision(modId) + 1)
}

/** 工作文件能力持有任务差异及读取代次，布局只选择和展示。 */
export function useTaskChanges(modId: () => string, taskId: () => string | undefined) {
  const changes = shallowRef<FileChange[]>([])
  const operations = shallowRef<TaskFileOperation[]>([])
  const diff = shallowRef<FileDiffData>()
  const selected = shallowRef('')
  const error = shallowRef('')
  let generation = 0
  async function select(path: string): Promise<void> {
    const current = ++generation
    const id = modId()
    const task = taskId()
    diff.value = undefined
    selected.value = path
    error.value = ''
    if (!task) {
      return
    }
    try {
      const result = await workspaceService.getTaskDiff({ modId: id, taskId: task, relativePath: path })
      if (current === generation && id === modId() && task === taskId()) {
        diff.value = result
      }
    }
    catch (cause) {
      if (current === generation && id === modId() && task === taskId()) {
        error.value = (cause as Error).message
      }
    }
  }
  watch(() => [modId(), taskId(), sourceRevision(modId())], async (_value, _old, onCleanup) => {
    let valid = true
    onCleanup(() => {
      valid = false
    })
    changes.value = []
    operations.value = []
    diff.value = undefined
    error.value = ''
    generation += 1
    const task = taskId()
    if (!task) {
      return
    }
    try {
      const recorded = await workspaceService.listTaskOperations({ modId: modId(), taskId: task })
      if (!valid) {
        return
      }
      operations.value = recorded
      const result = await workspaceService.listTaskChanges({ modId: modId(), taskId: task })
      if (!valid) {
        return
      }
      changes.value = result
      if (selected.value && result.some(item => item.path === selected.value)) {
        await select(selected.value)
      }
    }
    catch (cause) {
      if (valid) {
        error.value = (cause as Error).message
      }
    }
  }, { immediate: true, flush: 'sync' })
  onScopeDispose(() => {
    ++generation
  })
  return { changes, operations, diff, selected, error, select }
}
