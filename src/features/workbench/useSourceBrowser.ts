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
export function useSourceBrowser(props: { modId: string, revision: number, active: boolean }, rememberExpansion = false) {
  const directories = shallowRef<Record<string, SourceEntry[]>>({})
  const expanded = shallowRef(new Set<string>())
  const loading = shallowRef<Record<string, boolean>>({})
  const errors = shallowRef<Record<string, string>>({})
  const selected = shallowRef('')
  const focused = shallowRef('')
  const directory = shallowRef('')
  const preview = shallowRef<SourcePreview>()
  const previewLoading = shallowRef(false)
  const previewError = shallowRef('')
  let generation = 0
  let selectionGeneration = 0

  function readExpanded(modId: string): Set<string> {
    if (!rememberExpansion || !modId) {
      return new Set()
    }
    try {
      const paths: unknown = JSON.parse(localStorage.getItem(`source-expanded:${modId}`) ?? '[]')
      return new Set(Array.isArray(paths) ? paths.filter((path): path is string => typeof path === 'string') : [])
    }
    catch {
      return new Set()
    }
  }

  watch(expanded, (paths) => {
    if (rememberExpansion && props.modId) {
      try {
        localStorage.setItem(`source-expanded:${props.modId}`, JSON.stringify([...paths]))
      }
      catch {
        // 偏好无法保存时仍可浏览，本次展开状态保留在内存中。
      }
    }
  }, { flush: 'sync' })

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
    focused.value = path
    directory.value = path.includes('/') ? path.slice(0, path.lastIndexOf('/')) : ''
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

  function selectDirectory(path: string): void {
    directory.value = path
    focused.value = path
  }

  function focusEntry(entry: SourceEntry): void {
    focused.value = entry.path
    directory.value = entry.kind === 'directory' ? entry.path : entry.path.split('/').slice(0, -1).join('/')
  }

  async function expandDirectory(path: string): Promise<void> {
    const current = generation
    const parts = path ? path.split('/') : []
    expanded.value = new Set([...expanded.value, ...parts.map((_, index) => parts.slice(0, index + 1).join('/'))])
    await loadDirectory('')
    for (let index = 0; index < parts.length; index++) {
      if (current !== generation) {
        return
      }
      await loadDirectory(parts.slice(0, index + 1).join('/'))
    }
  }

  function remapPaths(source: string, target: string): void {
    const remap = (path: string) => path === source || path.startsWith(`${source}/`) ? target + path.slice(source.length) : path
    expanded.value = new Set([...expanded.value].map(remap))
    focused.value = remap(focused.value)
    directory.value = remap(directory.value)
    selected.value = remap(selected.value)
    selectionGeneration++
  }

  async function reveal(path: string): Promise<void> {
    const current = generation
    const parts = path.split('/').slice(0, -1)
    for (let index = 0; index < parts.length; index++) {
      const parent = parts.slice(0, index + 1).join('/')
      expanded.value = new Set([...expanded.value, parent])
      await loadDirectory(parent)
      if (current !== generation) {
        return
      }
    }
    await select(path)
  }

  /** 展开时更新当前目录，收起后保留子分支的展开偏好。 */
  function toggle(path: string): void {
    const next = new Set(expanded.value)
    if (next.has(path)) {
      next.delete(path)
    }
    else {
      next.add(path)
    }
    expanded.value = next
    if (next.has(path)) {
      const current = generation
      void loadDirectory(path).then(() => reloadExpanded(path, current))
    }
  }

  async function reloadExpanded(parent: string, current: number): Promise<void> {
    for (const entry of directories.value[parent] ?? []) {
      if (current !== generation) {
        return
      }
      if (entry.kind === 'directory' && expanded.value.has(entry.path)) {
        await loadDirectory(entry.path)
        if (current === generation) {
          await reloadExpanded(entry.path, current)
        }
      }
    }
  }

  /** 刷新只重新读取已展开分支，保留当前选择并更新磁盘预览。 */
  async function refresh(): Promise<void> {
    const current = ++generation
    loading.value = {}
    errors.value = {}
    if (!props.modId || !props.active) {
      return
    }
    if (selected.value) {
      const previousFocus = focused.value
      const previousDirectory = directory.value
      void select(selected.value)
      focused.value = previousFocus
      directory.value = previousDirectory
    }
    await loadDirectory('')
    if (current === generation) {
      await reloadExpanded('', current)
    }
  }

  watch(() => [props.modId, props.revision, props.active] as const, ([modId, , active], previous) => {
    if (modId !== previous?.[0]) {
      directories.value = {}
      expanded.value = readExpanded(modId)
      focused.value = ''
      selected.value = ''
      directory.value = ''
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
  }, { immediate: true, flush: 'sync' })
  onScopeDispose(() => {
    generation += 1
    selectionGeneration += 1
  })

  return { rows, selected, focused, directory, preview, previewLoading, previewError, rootLoading: computed(() => !!loading.value['']), rootError: computed(() => errors.value[''] ?? ''), toggle, select, selectDirectory, focusEntry, expandDirectory, remapPaths, reveal, refresh, loadDirectory }
}
