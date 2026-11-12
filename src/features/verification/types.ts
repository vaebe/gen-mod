export type VerificationResult = 'passed' | 'partial' | 'failed' | 'unknown'
export interface EvidenceRef { sessionId: string, entryId: string, attachmentIndex?: number, sha256: string }
export interface FeedbackPayload {
  versionId: string
  gameVersion?: string
  observations: string
  result: VerificationResult
  caseResults: { caseId: string, result: VerificationResult, observed: string }[]
  sourceRefs: EvidenceRef[]
}
export interface StoredEvidence { ref: EvidenceRef, summary: string }
export interface FeedbackDraft {
  draftId: string
  modId: string
  revision: number
  status: 'pending' | 'confirmed' | 'rejected'
  payload: FeedbackPayload
  evidence: StoredEvidence[]
}
export interface VerificationRecord {
  recordId: string
  modId: string
  versionId: string
  sourceDraftId: string
  confirmedAt: string
  payload: FeedbackPayload
  evidence: StoredEvidence[]
  supersedes?: string
}
export interface VerificationList { drafts: FeedbackDraft[], records: VerificationRecord[] }
export interface FeedbackSource { entryId: string, sessionId: string, text: string, sha256: string, attachments: { index: number, sha256: string, mimeType: string }[] }
