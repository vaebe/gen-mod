import { shallowRef, watch } from 'vue'
import { modService } from '@/services/native'
import { useProjectsStore } from '@/stores/projects'

/** 保存串行化，离开前 flush；失败时保留输入并阻止切换。 */
export function useDraft() {
  const requirement = shallowRef('')
  const error = shallowRef('')
  const saving = shallowRef(false)
  let projectId = ''
  let saved = ''
  let sessionId: string | undefined
  let pending: Promise<boolean> | undefined
  let submissionPending: Promise<void> | undefined
  const revisions = new Map<string, number>()
  let loading = false
  watch(requirement, () => {
    if (!loading) {
      const key = `${projectId}/${sessionId}`
      revisions.set(key, (revisions.get(key) ?? 0) + 1)
    }
  }, { flush: 'sync' })
  const projects = useProjectsStore()
  function load(id: string, value: string, session?: string): void {
    projectId = id
    sessionId = session
    saved = value
    loading = true
    requirement.value = value
    loading = false
    error.value = ''
  }
  async function flush(): Promise<boolean> {
    if (submissionPending) {
      await submissionPending
      return flush()
    }
    if (pending) {
      if (!await pending) {
        return false
      }
      return flush()
    }
    if (!projectId || saved === requirement.value) {
      return true
    }
    const text = requirement.value
    const id = projectId
    const targetSession = sessionId
    saving.value = true
    const operation = (async () => {
      try {
        projects.upsert(await modService.saveDraft(id, text, targetSession))
        if (id !== projectId || targetSession !== sessionId) {
          return true
        }
        saved = text
        error.value = ''
        return true
      }
      catch (cause) {
        error.value = (cause as Error).message
        return false
      }
      finally { saving.value = false }
    })()
    pending = operation
    const result = await operation
    if (pending === operation) {
      pending = undefined
    }
    return result && (saved === requirement.value || await flush())
  }
  /** 输入立即清空，提交快照独立保存；自动保存等待提交结束后再写入新草稿。 */
  function captureSubmitted() {
    const id = projectId
    const targetSession = sessionId
    const text = requirement.value
    const key = `${id}/${targetSession}`
    requirement.value = ''
    const revision = revisions.get(key) ?? 0
    let release!: () => void
    submissionPending = new Promise<void>((resolve) => {
      release = resolve
    })
    async function save(): Promise<boolean> {
      while (pending) {
        const operation = pending
        await operation
        if (pending === operation) {
          pending = undefined
        }
      }
      saving.value = true
      const operation = (async () => {
        try {
          projects.upsert(await modService.saveDraft(id, text, targetSession))
          if (id === projectId && targetSession === sessionId) {
            saved = text
            error.value = ''
          }
          return true
        }
        catch (cause) {
          error.value = (cause as Error).message
          return false
        }
        finally { saving.value = false }
      })()
      pending = operation
      const result = await operation
      if (pending === operation) {
        pending = undefined
      }
      return result
    }
    async function finish(restore: boolean, preserve = false): Promise<boolean> {
      try {
        const unchanged = id === projectId && targetSession === sessionId && (revisions.get(key) ?? 0) === revision
        if (restore && unchanged) {
          requirement.value = text
        }
        // 清理只匹配本次快照，不覆盖其他来源已写入的草稿。
        if (!restore && !preserve && unchanged && targetSession) {
          saving.value = true
          try {
            projects.upsert(await modService.saveDraft(id, '', targetSession, text))
            saved = ''
            error.value = ''
          }
          catch (cause) {
            error.value = (cause as Error).message
            return false
          }
          finally { saving.value = false }
        }
      }
      finally {
        submissionPending = undefined
        release()
      }
      return preserve ? true : flush()
    }
    return { save, finish }
  }
  return { requirement, error, saving, load, flush, captureSubmitted }
}
