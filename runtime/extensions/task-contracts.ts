// Node 工具到原生 task_tools.rs 的线协议；随原生结构同步，不能替代接收端运行时校验。
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

export type VerificationResult = 'passed' | 'partial' | 'failed' | 'unknown'
export interface EvidenceRef { sessionId: string, entryId: string, attachmentIndex?: number | null, sha256: string }
export interface FeedbackPayload {
  versionId: string
  gameVersion?: string | null
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
export interface FeedbackSource { entryId: string, sessionId: string, text: string, sha256: string, attachments: { index: number, sha256: string, mimeType: string }[] }

export interface KnowledgeNote {
  noteId: string
  revision: number
  gameId: string
  versionScope: 'exact' | 'independent' | 'unknown'
  gameVersion?: string | null
  route: string
  title: string
  content: string
  evidenceKind: 'reference' | 'user_observation' | 'development' | 'manual'
  sourceRefs: MemorySourceRef[]
  status: 'active' | 'archived' | 'deleted'
  allowReuse: boolean
  originModId?: string | null
  originProjectName?: string | null
  userEdited: boolean
  memoryKey: string
  createdAt: string
  updatedAt: string
}
export interface KnowledgeReference { id: string, path: string, sha256: string, title: string }

export interface MemorySourceRef {
  kind: 'verification' | 'reference' | 'command' | 'user_message' | 'manual'
  id: string
  taskId?: string
  sessionId?: string
  sha256?: string
}

export interface MemorySaveResult {
  outcome: 'saved' | 'unchanged' | 'user_managed'
  noteId: string
  revision: number
  title: string
}


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

export interface FeedbackSources { sources: FeedbackSource[]; scope: string }
export interface VerificationCase { id: string; precondition: string; action: string; expected: string; evidenceHint: string }
export interface MemorySaveInput {
  noteId?: string; expectedRevision?: number; topicKey: string;
  versionScope: 'exact' | 'independent' | 'unknown'; gameVersion?: string;
  title: string; content: string; evidenceKind: 'reference' | 'user_observation' | 'development';
  sourceRefs: (Omit<MemorySourceRef, 'kind'> & { kind: 'verification' | 'reference' | 'command' | 'user_message' })[];
}
export interface TaskRequests {
  'gen-mod:check-delivery': { input: { files: { source: string; target: string }[] }; result: {
    blocked: boolean; scope: string; checkReport: { rulesVersion: string; findings: { code: string; level: string; path: string }[] };
  } };
  'gen-mod:read-project-context': { input: undefined; result: { live: ProjectContext; snapshot: ContextSnapshot } };
  'gen-mod:read-project-memory-sources': { input: undefined; result: FeedbackSources & {
    commands: { taskId: string; commandId: string; status: string; exitCode: number | null; program: string; args: string[]; outputTail: string }[];
    inspectionReportIds: string[];
  } };
  'gen-mod:update-project-memory': { input: { input: ProjectMemoryUpdateInput }; result: { projectMemoryUpdate: ProjectMemoryUpdateResult } };
  'gen-mod:read-feedback-sources': { input: undefined; result: FeedbackSources & {
    versions: { versionId: string; createdAt: string; artifactDeleted: boolean; cases: VerificationCase[] }[];
    olderVersionsAvailable: boolean;
  } };
  'gen-mod:propose-verification': { input: { payload: FeedbackPayload }; result: FeedbackDraft };
  'gen-mod:search-knowledge': { input: { terms: string[] }; result: {
    notes: { note: KnowledgeNote; needsReview: boolean; originRemoved: boolean }[]; references: KnowledgeReference[];
  } };
  'gen-mod:save-long-term-memory': { input: { input: MemorySaveInput }; result: { memorySave: MemorySaveResult } };
  'gen-mod:read-game-inspection': { input: { reportId?: string }; result: InspectionReport | InspectionReport[] };
}
