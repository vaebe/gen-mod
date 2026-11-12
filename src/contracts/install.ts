export interface InstallationRecord {
  installationId: string
  versionId: string
  targetDirectory: string
  attemptedAt: string
  status: 'succeeded' | 'failed'
  rollback: 'notNeeded' | 'notAttempted'
  error?: string
}

export interface InstalledTarget { targetDirectory: string, versionId?: string, state: 'known' | 'uncertain' }

export interface InstallHint {
  description: string
  examplePath: string
}

export interface InstallPreview {
  previewId: string
  modId: string
  versionId: string
  targetDirectory: string
  entries: { source: string, target: string, overwrite: boolean, beforeHash?: string }[]
}
