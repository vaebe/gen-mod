import type { KnowledgeNote, MemoryEntryInput, MemoryPage } from '@/contracts/knowledge'
import { onScopeDispose, shallowRef } from 'vue'
import { nativeCall } from '@/services/native'

export function useLongTermMemory() {
  const notes = shallowRef<KnowledgeNote[]>([])
  const total = shallowRef(0)
  const page = shallowRef(1)
  const text = shallowRef('')
  const game = shallowRef('all')
  const status = shallowRef('all')
  const selected = shallowRef<KnowledgeNote>()
  const editing = shallowRef(false)
  const loading = shallowRef(false)
  const saving = shallowRef(false)
  const error = shallowRef('')
  const entryError = shallowRef('')
  let listGeneration = 0
  let entryGeneration = 0
  let disposed = false
  onScopeDispose(() => {
    disposed = true
  })

  async function refresh(nextPage = page.value): Promise<void> {
    const generation = ++listGeneration
    loading.value = true
    error.value = ''
    try {
      const result = await nativeCall<MemoryPage>('list_long_term_memories', { query: { text: text.value, gameId: game.value === 'all' ? null : game.value }, status: status.value === 'all' ? null : status.value, page: nextPage, pageSize: 20 })
      if (disposed || generation !== listGeneration) {
        return
      }
      notes.value = result.notes
      total.value = result.total
      page.value = nextPage
    }
    catch (cause) {
      if (!disposed && generation === listGeneration) {
        error.value = (cause as Error).message
      }
    }
    finally {
      if (!disposed && generation === listGeneration) {
        loading.value = false
      }
    }
  }
  async function select(id: string): Promise<void> {
    if (saving.value) {
      return
    }
    const generation = ++entryGeneration
    selected.value = undefined
    editing.value = false
    entryError.value = ''
    try {
      const note = await nativeCall<KnowledgeNote>('get_long_term_memory', { noteId: id })
      if (!disposed && generation === entryGeneration) {
        selected.value = note
        editing.value = true
      }
    }
    catch (cause) {
      if (!disposed && generation === entryGeneration) {
        entryError.value = `无法打开这条记忆，可能已被移除。${(cause as Error).message}`
      }
    }
  }
  function create(): void {
    if (saving.value) {
      return
    }
    ++entryGeneration
    selected.value = undefined
    entryError.value = ''
    editing.value = true
  }
  async function save(input: MemoryEntryInput): Promise<boolean> {
    if (saving.value) {
      return false
    }
    saving.value = true
    entryError.value = ''
    try {
      const note = await nativeCall<KnowledgeNote>('save_long_term_memory_entry', { noteId: selected.value?.noteId ?? null, expectedRevision: selected.value?.revision ?? null, input })
      if (!disposed) {
        selected.value = note
        await refresh()
      }
      return true
    }
    catch (cause) {
      if (!disposed) {
        entryError.value = (cause as Error).message
      }
      return false
    }
    finally { saving.value = false }
  }
  async function changeStatus(value: 'active' | 'archived' | 'deleted'): Promise<void> {
    const note = selected.value
    if (!note || saving.value) {
      return
    }
    saving.value = true
    entryError.value = ''
    try {
      await nativeCall('set_long_term_memory_status', { noteId: note.noteId, expectedRevision: note.revision, status: value })
      if (disposed) {
        return
      }
      if (value === 'deleted') {
        selected.value = undefined
        editing.value = false
      }
      else {
        selected.value = await nativeCall<KnowledgeNote>('get_long_term_memory', { noteId: note.noteId })
      }
      await refresh()
    }
    catch (cause) {
      if (!disposed) {
        entryError.value = (cause as Error).message
      }
    }
    finally { saving.value = false }
  }
  return { notes, total, page, text, game, status, selected, editing, loading, saving, error, entryError, refresh, select, create, save, changeStatus }
}
