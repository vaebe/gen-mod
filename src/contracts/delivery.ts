import type { SkillSnapshot } from './skill'

export interface ArtifactVersion {
  summary: string
  sessionId: string
  versionId: string
  taskId: string
  createdAt: string
  skills: SkillSnapshot[]
  files: { path: string, size: number, sha256: string }[]
  zipSha256: string
}

export interface InstallManifest {
  targetDirectoryHint: string
  entries: { source: string, target: string }[]
}

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
