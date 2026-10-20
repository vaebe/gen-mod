export interface ContentFinding {
  code: 'env_file' | 'private_key' | 'known_secret' | 'reference_copy' | 'review_origin'
  level: 'block' | 'review'
  path: string
}
export interface ContentCheckReport { rulesVersion: '1', findings: ContentFinding[] }
export interface VerificationCase { id: string, precondition: string, action: string, expected: string, evidenceHint: string }
export interface DeliveryMetadata {
  modId: string
  versionId: string
  taskId: string
  contextSnapshotId?: string
  createdAt: string
  artifactDeleted: boolean
  cases: VerificationCase[]
  checkReport?: ContentCheckReport
}
