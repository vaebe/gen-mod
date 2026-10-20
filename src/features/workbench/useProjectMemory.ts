import type { ModProject } from '@/contracts/mod'
import type { ProjectContext } from '@/contracts/project-context'
import { listen } from '@tauri-apps/api/event'
import { onScopeDispose, shallowRef, watch } from 'vue'
import { nativeCall } from '@/services/native'

export function useProjectMemory(modId: () => string, active: () => boolean, revision: () => number) {
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
      const [value, project] = await Promise.all([nativeCall<ProjectContext>('get_project_context', { modId: id }), nativeCall<ModProject>('get_mod', { modId: id })])
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
  watch(() => [modId(), active(), revision()], () => {
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
