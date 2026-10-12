import type { SourceEntry, SourcePreview } from '@/contracts/source'
import { computed, onScopeDispose, shallowRef, watch } from 'vue'
import { nativeCall } from '@/services/native'

export interface SourceTreeRow extends SourceEntry {
  depth: number
  expanded: boolean
  loading: boolean
  error: string
  empty: boolean
}

/** 维护按目录加载的文件树；项目切换、刷新和卸载后丢弃旧请求。 */
export function useSourceBrowser(props: { modId: string, revision: number, active: boolean }) {
  const directories = shallowRef<Record<string, SourceEntry[]>>({})
  const expanded = shallowRef(new Set<string>())
  const loading = shallowRef<Record<string, boolean>>({})
  const errors = shallowRef<Record<string, string>>({})
  const selected = shallowRef('')
  const preview = shallowRef<SourcePreview>()
  const previewLoading = shallowRef(false)
  const previewError = shallowRef('')
  let generation = 0
  let selectionGeneration = 0

  const rows = computed(() => {
    const result: SourceTreeRow[] = []
    function append(directory: string, depth: number): void {
      for (const entry of directories.value[directory] ?? []) {
        const open = expanded.value.has(entry.path)
        result.push({ ...entry, depth, expanded: open, loading: !!loading.value[entry.path], error: errors.value[entry.path] ?? '', empty: directories.value[entry.path]?.length === 0 })
        if (entry.kind === 'directory' && open) {
          append(entry.path, depth + 1)
        }
      }
    }
    append('', 0)
    return result
  })

  /** 只请求单层目录，目录失败时允许在原位置重试。 */
  async function loadDirectory(path: string): Promise<void> {
    if (!props.modId || loading.value[path]) {
      return
    }
    const current = generation
    const modId = props.modId
    loading.value = { ...loading.value, [path]: true }
    errors.value = { ...errors.value, [path]: '' }
    try {
      const entries = await nativeCall<SourceEntry[]>('list_source_directory', { modId, relativePath: path })
      if (current === generation && modId === props.modId) {
        directories.value = { ...directories.value, [path]: entries }
      }
    }
    catch (cause) {
      if (current === generation && modId === props.modId) {
        errors.value = { ...errors.value, [path]: (cause as Error).message }
      }
    }
    finally {
      if (current === generation && modId === props.modId) {
        loading.value = { ...loading.value, [path]: false }
      }
    }
  }

  /** 先清除旧内容，快速连续选择时仅显示最后一次读取结果。 */
  async function select(path: string): Promise<void> {
    const current = ++selectionGeneration
    const modId = props.modId
    selected.value = path
    preview.value = undefined
    previewError.value = ''
    previewLoading.value = !!modId && !!path
    if (!modId || !path) {
      return
    }
    try {
      const result = await nativeCall<SourcePreview>('read_source_file', { modId, relativePath: path })
      if (current === selectionGeneration && modId === props.modId) {
        preview.value = result
      }
    }
    catch (cause) {
      if (current === selectionGeneration && modId === props.modId) {
        previewError.value = (cause as Error).message
      }
    }
    finally {
      if (current === selectionGeneration && modId === props.modId) {
        previewLoading.value = false
      }
    }
  }

  /** 文件夹首次展开时读取子项；收起后保留本次浏览缓存。 */
  function toggle(path: string): void {
    const next = new Set(expanded.value)
    if (next.has(path)) {
      // 收起时同时关闭子分支，避免刷新后保留尚未加载的展开状态。
      for (const item of next) {
        if (item === path || item.startsWith(`${path}/`)) {
          next.delete(item)
        }
      }
    }
    else {
      next.add(path)
      if (!directories.value[path]) {
        void loadDirectory(path)
      }
    }
    expanded.value = next
  }

  /** 刷新只重新读取已展开分支，保留当前选择并更新磁盘预览。 */
  async function refresh(): Promise<void> {
    const current = ++generation
    directories.value = {}
    loading.value = {}
    errors.value = {}
    if (!props.modId || !props.active) {
      return
    }
    if (selected.value) {
      void select(selected.value)
    }
    await loadDirectory('')
    async function reloadExpanded(parent: string): Promise<void> {
      for (const entry of directories.value[parent] ?? []) {
        if (current !== generation) {
          return
        }
        if (entry.kind === 'directory' && expanded.value.has(entry.path)) {
          await loadDirectory(entry.path)
          if (current !== generation) {
            return
          }
          await reloadExpanded(entry.path)
        }
      }
    }
    if (current === generation) {
      await reloadExpanded('')
    }
  }

  watch(() => [props.modId, props.revision, props.active] as const, ([modId, , active], previous) => {
    if (modId !== previous?.[0]) {
      expanded.value = new Set()
      selected.value = ''
      preview.value = undefined
      previewError.value = ''
      previewLoading.value = false
      selectionGeneration += 1
    }
    if (active) {
      void refresh()
    }
    else {
      generation += 1
      selectionGeneration += 1
      previewLoading.value = false
    }
  }, { immediate: true })
  onScopeDispose(() => {
    generation += 1
    selectionGeneration += 1
  })

  return { rows, selected, preview, previewLoading, previewError, rootLoading: computed(() => !!loading.value['']), rootError: computed(() => errors.value[''] ?? ''), toggle, select, refresh, loadDirectory }
}
