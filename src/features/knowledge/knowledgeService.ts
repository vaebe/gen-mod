import type { KnowledgeNote, MemoryEntryInput, MemoryListOptions, MemoryPage } from './types'
import { nativeCall } from '@/services/native'

/** 跨项目长期记忆命令；原生 knowledge 负责范围、revision 和用户编辑保护校验。 */
export const knowledgeService = {
  list(options: MemoryListOptions): Promise<MemoryPage> {
    return nativeCall('list_long_term_memories', { ...options })
  },
  get(noteId: string): Promise<KnowledgeNote> {
    return nativeCall('get_long_term_memory', { noteId })
  },
  save(noteId: string | null, expectedRevision: number | null, input: MemoryEntryInput): Promise<KnowledgeNote> {
    return nativeCall('save_long_term_memory_entry', { noteId, expectedRevision, input })
  },
  setStatus(noteId: string, expectedRevision: number, status: KnowledgeNote['status']): Promise<void> {
    return nativeCall('set_long_term_memory_status', { noteId, expectedRevision, status })
  },
}
