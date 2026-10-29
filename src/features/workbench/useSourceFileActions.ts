import type { SourceImportItem, SourceMutationItem, SourceMutationResult } from '@/contracts/source'
import { onScopeDispose, shallowRef, watch } from 'vue'
import { nativeCall } from '@/services/native'

export type SourceNativeCall = (command: string, args?: Record<string, unknown>) => Promise<unknown>

function isMutation(value: unknown): value is SourceMutationResult {
  return typeof value === 'object' && value !== null
    && 'operationId' in value && typeof value.operationId === 'string'
    && 'sourceRevision' in value && typeof value.sourceRevision === 'number' && Number.isSafeInteger(value.sourceRevision)
    && 'items' in value && Array.isArray(value.items) && value.items.every((item: unknown) => {
    return typeof item === 'object' && item !== null
      && 'source' in item && typeof item.source === 'string'
      && 'target' in item && typeof item.target === 'string'
      && 'status' in item && typeof item.status === 'string' && ['imported', 'moved', 'conflict', 'failed'].includes(item.status)
      && (!('message' in item) || item.message == null || typeof item.message === 'string')
  })
}

export function useSourceFileActions(readModId: () => string, invoke: SourceNativeCall = nativeCall) {
  const busy = shallowRef(false)
  const error = shallowRef('')
  const results = shallowRef<SourceMutationItem[]>([])
  let generation = 0

  watch(readModId, () => {
    generation++
    busy.value = false
    error.value = ''
    results.value = []
  }, { flush: 'sync' })
  onScopeDispose(() => {
    generation++
  })

  async function execute<T>(command: string, args: Record<string, unknown>, accept: (value: unknown) => T): Promise<T | undefined> {
    if (busy.value || !readModId()) {
      return
    }
    const current = ++generation
    const modId = readModId()
    busy.value = true
    error.value = ''
    results.value = []
    try {
      const value = await invoke(command, { modId, ...args })
      if (current === generation && modId === readModId()) {
        return accept(value)
      }
    }
    catch (cause) {
      if (current === generation && modId === readModId()) {
        error.value = cause instanceof Error ? cause.message : String(cause)
      }
    }
    finally {
      if (current === generation && modId === readModId()) {
        busy.value = false
      }
    }
  }

  function acceptMutation(value: unknown): SourceMutationResult {
    if (!isMutation(value)) {
      throw new Error('文件操作响应格式不正确，请刷新核对')
    }
    results.value = value.items
    return value
  }

  async function importFiles(targetDirectory: string, items: SourceImportItem[]): Promise<void> {
    if (!items.length) {
      return
    }
    await execute('import_source_files', { targetDirectory, items }, acceptMutation)
  }

  async function moveFile(source: string, targetDirectory: string, targetName: string): Promise<SourceMutationResult | undefined> {
    return execute('move_source_file', { source, targetDirectory, targetName }, acceptMutation)
  }

  async function createDirectory(parentDirectory: string, name: string) {
    return execute('create_source_directory', { parentDirectory, name }, (value) => {
      if (typeof value !== 'object' || value === null || !('path' in value) || typeof value.path !== 'string'
        || !('sourceRevision' in value) || typeof value.sourceRevision !== 'number') {
        throw new Error('创建目录响应格式不正确，请刷新核对')
      }
      return { path: value.path, sourceRevision: value.sourceRevision }
    })
  }

  return { busy, error, results, importFiles, moveFile, createDirectory }
}
