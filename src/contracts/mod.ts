import type { SessionReference } from './session'
import type { SkillSnapshot } from './skill'

export interface ArtifactVersion {
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

export interface InstallationRecord {
  installationId: string
  versionId: string
  targetDirectory: string
  attemptedAt: string
  status: 'succeeded' | 'failed'
  rollback: 'notNeeded' | 'notAttempted' | 'succeeded' | 'failed'
  backupDirectory?: string
  error?: string
}

/** 项目名可修改，modId 及所在目录不随名称变化。 */
export interface ModProject {
  revision: number
  archived: boolean
  modId: string
  name: string
  gameId: string
  gameSkillName: string
  description: string
  draftRequirement: string
  updatedAt: string
  developmentStatus: 'draft' | 'generated' | 'running' | 'failed' | 'ready'
  latestVersionId?: string
  /** 通过目录校验的最近部署位置；旧项目可从安装记录回填显示。 */
  lastDeploymentDirectory?: string
  versions: ArtifactVersion[]
  installations: InstallationRecord[]
  installedTargets: { targetDirectory: string, versionId?: string, state: 'known' | 'uncertain' }[]
  sessions: SessionReference[]
}

export interface DeleteProjectResult { modId: string, removed: boolean, cleanupPending: boolean, message?: string | null }
