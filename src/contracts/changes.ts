export interface FileChange {
  path: string
  kind: 'added' | 'modified' | 'deleted'
  added: number
  removed: number
  beforeSize: number | null
  afterSize: number | null
  textAvailable: boolean
}
export interface FileDiff {
  change: FileChange
  before: string | null
  after: string | null
  lines: { kind: 'equal' | 'insert' | 'delete', text: string }[]
}

/** 持久化任务文件操作记录中用于变更视图的字段。 */
export interface TaskFileOperation {
  toolCallId: string
  operation: string
  path?: string
  sourceRevision: number
  fileChanges?: { path: string }[]
}
