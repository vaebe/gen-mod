import type { ArtifactVersion } from './delivery'
import type { InstallationRecord, InstalledTarget } from './install'
import type { SessionReference } from './session'

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
  /** 通过目录校验的最近部署位置；新项目尚未选择时为空。 */
  lastDeploymentDirectory?: string
  versions: ArtifactVersion[]
  installations: InstallationRecord[]
  installedTargets: InstalledTarget[]
  sessions: SessionReference[]
}

export interface DeleteProjectResult { modId: string, removed: boolean, cleanupPending: boolean, message?: string | null }
