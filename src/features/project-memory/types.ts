export interface ContextField {
  value: string
  source: 'user' | 'inspection'
  sourceId?: string
  observedAt: string
}
export interface ProjectContext {
  modId: string
  revision: number
  gameVersion?: ContextField
  referenceSets: { id: string, directory: string, sourceReportId: string, files: { path: string, sha256: string }[] }[]
  prerequisites: { name: string, version?: string }[]
  confirmedRequirements: string[]
  fieldSources: Partial<Record<'gameVersion' | 'prerequisites' | 'confirmedRequirements', ProjectMemorySourceRef[]>>
}
export interface ContextSnapshot {
  snapshotId: string
  modId: string
  taskId: string
  createdAt: string
  context: ProjectContext
}
export type ContextChange
  = | { field: 'gameVersion', reason: string, value: string }
    | { field: 'prerequisites', reason: string, value: ProjectContext['prerequisites'] }
    | { field: 'confirmedRequirements', reason: string, value: string[] }

export type ProjectMemorySourceRef
  = | { kind: 'user_message', sessionId: string, entryId: string, sha256: string }
    | { kind: 'command', taskId: string, commandId: string }
    | { kind: 'inspection', reportId: string }
export interface ProjectMemoryUpdateInput { expectedRevision: number, changes: (ContextChange & { sourceRefs: ProjectMemorySourceRef[] })[] }
export interface ProjectMemoryUpdateResult { outcome: 'saved' | 'unchanged', modId: string, revision: number, changedFields: ('gameVersion' | 'prerequisites' | 'confirmedRequirements')[] }
