export interface InspectionReport {
  reportId: string
  modId: string
  gameId: string
  kind: 'install' | 'references'
  inspectedAt: string
  findings: { relativePath: string, fact: string, sha256?: string }[]
  unknowns: string[]
  truncated: boolean
  errors: { relativePath: string, code: 'permission' | 'format' | 'limit' }[]
}
