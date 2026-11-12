export interface KnowledgeNote {
  noteId: string
  revision: number
  gameId: string
  versionScope: 'exact' | 'independent' | 'unknown'
  gameVersion?: string
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
export type KnowledgeInput = Pick<KnowledgeNote, 'versionScope' | 'gameVersion' | 'route' | 'title' | 'content' | 'evidenceKind' | 'sourceRefs'>
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

export interface MemoryEntryInput {
  title: string
  content: string
  gameId: string
  versionScope: KnowledgeNote['versionScope']
  gameVersion?: string
}

export interface MemoryPage { notes: KnowledgeNote[], total: number }

export interface MemoryListOptions {
  query: { text: string, gameId: string | null } | null
  status: string | null
  page: number
  pageSize: number
}
