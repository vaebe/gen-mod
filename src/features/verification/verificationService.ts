import type { FeedbackDraft, FeedbackPayload, VerificationList, VerificationRecord } from './types'
import { nativeCall } from '@/services/native'

interface DraftIdentity {
  modId: string
  draftId: string
  expectedRevision: number
}

/** 对应 Rust verification 命令；身份、revision 与证据仍由原生层校验。 */
export const verificationService = {
  list(modId: string, versionId?: string): Promise<VerificationList> {
    return nativeCall('list_verification', { modId, versionId })
  },
  edit(args: DraftIdentity & { payload: FeedbackPayload }): Promise<FeedbackDraft> {
    return nativeCall('edit_feedback_draft', { ...args })
  },
  confirm(args: DraftIdentity & { supersedes?: string }): Promise<VerificationRecord> {
    return nativeCall('confirm_feedback_draft', { ...args })
  },
  reject(args: DraftIdentity): Promise<void> {
    return nativeCall('reject_feedback_draft', { ...args })
  },
}
