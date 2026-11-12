import type { VerificationList } from '@/features/verification/types'
import { onScopeDispose, reactive, shallowRef, watch } from 'vue'
import { verificationService } from './verificationService'

const invalidations = reactive(new Map<string, number>())

export function invalidateVerification(modId: string): void {
  invalidations.set(modId, (invalidations.get(modId) ?? 0) + 1)
}

export function useVerification(projectId: () => string | undefined, revision: () => number = () => 0, versionId: () => string | undefined = () => undefined) {
  const data = shallowRef<VerificationList>({ drafts: [], records: [] })
  const error = shallowRef('')
  let generation = 0
  async function refresh(): Promise<void> {
    const current = ++generation
    const modId = projectId()
    const version = versionId()
    if (!modId) {
      data.value = { drafts: [], records: [] }
      return
    }
    try {
      const value = await verificationService.list(modId, version)
      if (current === generation && modId === projectId() && version === versionId()) {
        data.value = value
        error.value = ''
      }
    }
    catch (cause) {
      if (current === generation && modId === projectId() && version === versionId()) {
        error.value = (cause as Error).message
      }
    }
  }
  onScopeDispose(() => {
    ++generation
  })
  watch(() => [projectId(), revision(), versionId(), invalidations.get(projectId() ?? '')] as const, (identity, previous) => {
    if (!previous || identity[0] !== previous[0] || identity[2] !== previous[2]) {
      data.value = { drafts: [], records: [] }
      error.value = ''
    }
    void refresh()
  }, { immediate: true, flush: 'sync' })
  return { data, error, refresh }
}
