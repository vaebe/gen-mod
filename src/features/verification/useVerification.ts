import type { VerificationList } from '@/contracts/verification'
import { shallowRef, watch } from 'vue'
import { nativeCall } from '@/services/native'

export function useVerification(projectId: () => string | undefined, revision: () => number, versionId: () => string | undefined = () => undefined) {
  const data = shallowRef<VerificationList>({ drafts: [], records: [] })
  const error = shallowRef('')
  let generation = 0
  async function refresh(): Promise<void> {
    const current = ++generation
    const modId = projectId()
    if (!modId) {
      data.value = { drafts: [], records: [] }
      return
    }
    try {
      const value = await nativeCall<VerificationList>('list_verification', { modId, versionId: versionId() })
      if (current === generation) {
        data.value = value
        error.value = ''
      }
    }
    catch (cause) {
      if (current === generation) {
        error.value = (cause as Error).message
      }
    }
  }
  watch(() => [projectId(), revision(), versionId()] as const, (identity, previous) => {
    if (!previous || identity[0] !== previous[0] || identity[2] !== previous[2]) {
      data.value = { drafts: [], records: [] }
      error.value = ''
    }
    void refresh()
  }, { immediate: true })
  return { data, error, refresh }
}
