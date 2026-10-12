export interface InstallPreview {
  previewId: string
  modId: string
  versionId: string
  targetDirectory: string
  entries: { source: string, target: string, overwrite: boolean, beforeHash?: string }[]
}
