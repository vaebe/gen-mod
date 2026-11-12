import type { ProjectContext } from '@/features/project-memory/types'
import { listen } from '@tauri-apps/api/event'
import { onScopeDispose, reactive, shallowRef, watch } from 'vue'
import { modService } from '@/features/projects/service'
import { projectMemoryService } from './projectMemoryService'

const invalidations = reactive(new Map<string, number>())
export function invalidateProjectMemory(modId: string): void {
  invalidations.set(modId, (invalidations.get(modId) ?? 0) + 1)
}

export function useProjectMemory(modId: () => string, active: () => boolean) {
  const memory = shallowRef<ProjectContext>()
  const archived = shallowRef(false)
  const error = shallowRef('')
  const loading = shallowRef(false)
  let generation = 0
  let disposed = false
  let unlisten: (() => void) | undefined
  async function refresh(): Promise<void> {
    const id = modId()
    const epoch = ++generation
    if (!id || !active() || disposed) {
      return
    }
    loading.value = true
    error.value = ''
    try {
      const [value, project] = await Promise.all([projectMemoryService.get(id), modService.get(id)])
      if (!disposed && epoch === generation && id === modId()) {
        memory.value = value
        archived.value = project.archived
      }
    }
    catch (cause) {
      if (!disposed && epoch === generation) {
        memory.value = undefined
        error.value = (cause as Error).message
      }
    }
    finally {
      if (!disposed && epoch === generation) {
        loading.value = false
      }
    }
  }
  watch(modId, () => {
    ++generation
    memory.value = undefined
    archived.value = false
    error.value = ''
    loading.value = false
  }, { flush: 'sync' })
  watch(() => [modId(), active(), invalidations.get(modId())], () => {
    void refresh()
  }, { immediate: true })
  void listen<{ modId: string, revision: number }>('project-memory-changed', (event) => {
    if (event.payload.modId === modId() && active()) {
      void refresh()
    }
  }).then((stop) => {
    if (disposed) {
      stop()
    }
    else { unlisten = stop }
  }).catch((cause: Error) => {
    if (!disposed) {
      error.value = cause.message
    }
  })
  onScopeDispose(() => {
    disposed = true
    ++generation
    unlisten?.()
  })
  return { memory, archived, error, loading, refresh }
}
