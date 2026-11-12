import type { SourceEntry, SourceMutationItem, SourceTransferItem } from '@/contracts/source'
import { computed, onScopeDispose, shallowRef, watch } from 'vue'
import { addSourceReferences, remapSourceReferences, sourceLeaveGuards } from './sourceWorkspace'
import { useSourceBrowser } from './useSourceBrowser'
import { workspaceService } from './workspaceService'

export type SourceAction = 'file' | 'folder' | 'rename' | 'delete' | 'copy' | 'cut' | 'paste' | 'import' | 'importFolder' | 'export' | 'reference' | 'relative' | 'absolute' | 'open' | 'reveal' | 'refresh'
const parent = (path: string) => path.split('/').slice(0, -1).join('/')
const basename = (path: string) => path.split(/[\\/]/u).at(-1) ?? ''
const join = (directory: string, name: string) => directory ? `${directory}/${name}` : name
const displayText = (value: string) => value.replace(/\r\n|\r/gu, '\n')

export function useSourceManager(props: { modId: string, active: boolean, revision: number }, blocked: () => string) {
  const browser = useSourceBrowser(props)
  const selection = shallowRef<string[]>([])
  const busy = shallowRef(false)
  const error = shallowRef('')
  const results = shallowRef<SourceMutationItem[]>([])
  const clipboard = shallowRef<{ modId: string, operation: 'copy' | 'move', paths: string[] }>()
  const editing = shallowRef<{ mode: 'file' | 'folder' | 'rename', directory: string, path: string, name: string, isDirectory: boolean }>()
  const editError = shallowRef('')
  const text = shallowRef('')
  const original = shallowRef('')
  const editPath = shallowRef('')
  const dirty = computed(() => !!editPath.value && text.value !== displayText(original.value))
  const retryOperation = shallowRef<'copy' | 'move' | 'import' | 'export'>()
  const question = shallowRef<{ title: string, detail: string, choices: { value: string, label: string }[] }>()
  let resolveQuestion: ((answer: string) => void) | undefined
  let generation = 0
  let disposed = false
  function answer(value: string): void {
    const resolve = resolveQuestion
    resolveQuestion = undefined
    question.value = undefined
    resolve?.(value)
  }
  async function ask(title: string, detail: string, choices: { value: string, label: string }[]): Promise<string> {
    answer('cancel')
    question.value = { title, detail, choices }
    return new Promise((resolve) => {
      resolveQuestion = resolve
    })
  }

  watch(() => browser.preview.value, (preview) => {
    if (dirty.value && editPath.value === browser.selected.value) {
      return
    }
    editPath.value = preview?.text != null ? browser.selected.value : ''
    original.value = preview?.text ?? ''
    text.value = displayText(original.value)
  })

  async function save(): Promise<boolean> {
    if (blocked() || busy.value || !editPath.value) {
      error.value = blocked() || '文件操作进行中'
      return false
    }
    const current = generation
    const newline = original.value.includes('\r\n') ? '\r\n' : original.value.includes('\r') ? '\r' : '\n'
    const content = displayText(text.value).replaceAll('\n', newline)
    busy.value = true
    error.value = ''
    try {
      await workspaceService.saveText({ modId: props.modId, path: editPath.value, expected: original.value, text: content })
      if (current !== generation) {
        return false
      }
      original.value = content
      return true
    }
    catch (cause) {
      if (current === generation) {
        error.value = String(cause)
      }
      return false
    }
    finally {
      if (current === generation) {
        busy.value = false
      }
    }
  }

  async function leave(): Promise<boolean> {
    if (busy.value || question.value) {
      return false
    }
    if (!dirty.value) {
      return true
    }
    const current = generation
    const decision = await ask('文件尚未保存', editPath.value, [{ value: 'save', label: '保存' }, { value: 'discard', label: '放弃修改' }])
    if (current !== generation) {
      return false
    }
    if (decision === 'save') {
      return save()
    }
    if (decision === 'discard') {
      text.value = displayText(original.value)
      return true
    }
    return false
  }

  watch(() => props.modId, (id, previous) => {
    if (previous) {
      sourceLeaveGuards.delete(previous)
    }
    generation++
    answer('cancel')
    busy.value = false
    editing.value = undefined
    selection.value = []
    clipboard.value = undefined
    error.value = ''
    results.value = []
    retryOperation.value = undefined
    text.value = ''
    original.value = ''
    editPath.value = ''
    sourceLeaveGuards.set(id, leave)
  }, { immediate: true, flush: 'sync' })
  onScopeDispose(() => {
    disposed = true
    generation++
    answer('cancel')
    sourceLeaveGuards.delete(props.modId)
  })

  async function open(path: string): Promise<void> {
    const current = generation
    if (path !== browser.selected.value && !await leave()) {
      return
    }
    if (current !== generation) {
      return
    }
    await browser.select(path)
  }

  function selectedEntries(): SourceEntry[] {
    return browser.rows.value.filter(row => selection.value.includes(row.path) && row.kind !== 'blocked')
  }

  function topEntries(entries: SourceEntry[]): SourceEntry[] {
    return entries.filter(entry => !entries.some(other => other !== entry && other.kind === 'directory' && entry.path.startsWith(`${other.path}/`)))
  }

  function applyResults(items: SourceMutationItem[]): void {
    for (const item of [...items].sort((a, b) => b.source.length - a.source.length)) {
      if (item.status === 'moved') {
        browser.remapPaths(item.source, item.target)
        remapSourceReferences(props.modId, item.source, item.target)
      }
      if (item.status === 'deleted') {
        remapSourceReferences(props.modId, item.source)
        if (browser.selected.value === item.source || browser.selected.value.startsWith(`${item.source}/`)) {
          void browser.select('')
        }
      }
    }
  }

  async function transfer(operation: 'copy' | 'move' | 'import' | 'export', items: SourceTransferItem[]): Promise<void> {
    const current = generation
    const id = props.modId
    if (!items.length || busy.value || blocked() || !await leave() || current !== generation || blocked()) {
      return
    }
    busy.value = true
    error.value = ''
    results.value = []
    retryOperation.value = operation
    try {
      let pending = items
      while (pending.length) {
        if (current !== generation || disposed) {
          return
        }
        const result = await workspaceService.transferEntries({ modId: id, operation, items: pending })
        if (current !== generation) {
          return
        }
        applyResults(result.items)
        results.value = [...results.value, ...result.items.filter(item => item.status !== 'conflict')]
        pending = []
        for (const conflict of result.items.filter(item => item.status === 'conflict')) {
          const decision = await ask('同名冲突', `${conflict.source}\n→ ${conflict.target}\n${conflict.message}`, [
            ...(conflict.message === '同名文件已存在' ? [{ value: 'replace', label: '替换' }] : []),
            { value: 'keep', label: '保留两份' },
            { value: 'skip', label: '跳过' },
          ])
          if (current !== generation) {
            return
          }
          if (decision === 'cancel') {
            results.value = [...results.value, ...result.items.filter(item => item.status === 'conflict').map(item => ({ ...item, message: '已取消，尚未处理' }))]
            return
          }
          pending.push({ source: conflict.source, target: conflict.target, decision: decision as SourceTransferItem['decision'] })
        }
      }
      if (operation === 'move' && current === generation) {
        const result = await workspaceService.transferEntries({ modId: id, operation, items: [], finalize: items })
        if (current === generation) {
          applyResults(result.items)
          results.value = [...results.value, ...result.items]
        }
      }
    }
    catch (cause) {
      if (current === generation) {
        error.value = String(cause)
      }
    }
    finally {
      if (current === generation) {
        busy.value = false
        selection.value = []
        await browser.refresh()
      }
    }
  }

  async function submitName(name: string): Promise<void> {
    const edit = editing.value
    const current = generation
    if (!edit || busy.value || blocked() || !await leave() || current !== generation) {
      return
    }
    if (!name.trim() || /[/\\]/u.test(name)) {
      editError.value = '请输入单层文件或文件夹名称'
      return
    }
    busy.value = true
    editError.value = ''
    try {
      let path = join(edit.directory, name)
      if (edit.mode === 'rename') {
        if (name !== basename(edit.path)) {
          const result = await workspaceService.moveFile({ modId: props.modId, source: edit.path, targetDirectory: edit.directory, targetName: name })
          const moved = result.items.find(item => item.status === 'moved')
          if (!moved) {
            throw new Error(result.items[0]?.message || '重命名失败')
          }
          if (current === generation) {
            applyResults(result.items)
          }
        }
      }
      else {
        const created = await (edit.mode === 'file' ? workspaceService.createFile : workspaceService.createDirectory)({ modId: props.modId, parentDirectory: edit.directory, name })
        path = created.path
      }
      if (current !== generation) {
        return
      }
      editing.value = undefined
      await browser.expandDirectory(edit.directory)
      selection.value = [path]
      if (!edit.isDirectory) {
        await browser.select(path)
      }
      else if (edit.mode === 'rename' && browser.selected.value) {
        await browser.select(browser.selected.value)
      }
    }
    catch (cause) {
      if (current === generation) {
        editError.value = String(cause)
      }
    }
    finally {
      if (current === generation) {
        busy.value = false
      }
    }
  }

  function updateName(name: string): void {
    if (editing.value) {
      editing.value = { ...editing.value, name }
    }
  }

  async function importPaths(paths: string[], directory: string): Promise<void> {
    const current = generation
    if (busy.value || blocked() || await ask('导入到工作目录', `目标：${directory || '顶层'}\n${paths.join('\n')}\n外部原件保留。`, [{ value: 'import', label: '复制导入' }]) !== 'import' || current !== generation) {
      return
    }
    await transfer('import', paths.map(source => ({ source, target: join(directory, basename(source)) })))
  }

  async function retryFailed(): Promise<void> {
    if (!retryOperation.value) {
      return
    }
    const items = results.value.filter(item => item.status === 'failed' || item.status === 'conflict').map(item => ({ source: item.source, target: item.target }))
    await transfer(retryOperation.value, items)
  }

  async function reload(): Promise<void> {
    if (await leave()) {
      await browser.select(browser.selected.value)
    }
  }

  async function action(kind: SourceAction, target?: SourceEntry): Promise<void> {
    const current = generation
    const id = props.modId
    error.value = ''
    const entries = target ? (selection.value.includes(target.path) ? selectedEntries() : [target]) : selectedEntries()
    const directory = target ? (target.kind === 'directory' ? target.path : parent(target.path)) : (entries.length === 1 ? (entries[0]!.kind === 'directory' ? entries[0]!.path : parent(entries[0]!.path)) : '')
    try {
      if (kind === 'refresh') {
        await browser.refresh()
        return
      }
      if (kind === 'reference') {
        addSourceReferences(id, entries)
        return
      }
      if (kind === 'relative' || kind === 'absolute' || kind === 'open' || kind === 'reveal') {
        const path = entries[0]?.path ?? ''
        const absolute = await workspaceService.entryAction({ modId: id, path, action: kind === 'open' || kind === 'reveal' ? kind : 'path' })
        if (current === generation && (kind === 'relative' || kind === 'absolute')) {
          await navigator.clipboard.writeText(kind === 'relative' ? path : absolute)
        }
        return
      }
      if (busy.value || blocked()) {
        error.value = blocked() || '文件操作进行中'
        return
      }
      if (kind === 'copy' || kind === 'cut') {
        clipboard.value = { modId: id, operation: kind === 'copy' ? 'copy' : 'move', paths: topEntries(entries).map(entry => entry.path) }
      }
      else if (kind === 'file' || kind === 'folder' || kind === 'rename') {
        if (kind === 'rename' && entries.length !== 1) {
          return
        }
        await browser.expandDirectory(directory)
        if (current === generation) {
          editing.value = { mode: kind, directory: kind === 'rename' ? parent(entries[0]!.path) : directory, path: kind === 'rename' ? entries[0]!.path : '', name: kind === 'rename' ? entries[0]!.name : '', isDirectory: kind === 'folder' || (kind === 'rename' && entries[0]!.kind === 'directory') }
          editError.value = ''
        }
      }
      else if (kind === 'paste' && clipboard.value?.modId === id) {
        await transfer(clipboard.value.operation, clipboard.value.paths.map(source => ({ source, target: join(directory, basename(source)) })))
        if (current === generation && clipboard.value?.operation === 'move') {
          clipboard.value = undefined
        }
      }
      else if (kind === 'import' || kind === 'importFolder') {
        const picked = await (kind === 'import' ? workspaceService.pickImportFiles() : workspaceService.pickDirectory())
        if (picked && current === generation) {
          await importPaths(Array.isArray(picked) ? picked : [picked], directory)
        }
      }
      else if (kind === 'export' && entries.length) {
        const picked = await workspaceService.pickDirectory()
        if (picked && current === generation) {
          await transfer('export', topEntries(entries).map(entry => ({ source: entry.path, target: join(picked.replaceAll('\\', '/'), entry.name) })))
        }
      }
      else if (kind === 'delete' && entries.length) {
        if (!await leave() || current !== generation) {
          return
        }
        if ((entries.length > 1 || entries.some(entry => entry.kind === 'directory')) && await ask('移入系统回收站', entries.map(entry => entry.path).join('\n'), [{ value: 'delete', label: '移入回收站' }]) !== 'delete') {
          return
        }
        if (current !== generation || blocked()) {
          return
        }
        busy.value = true
        retryOperation.value = undefined
        try {
          const result = await workspaceService.recycleEntries({ modId: id, entries: topEntries(entries).map(entry => entry.path) })
          if (current === generation) {
            results.value = result.items
            applyResults(result.items)
            selection.value = []
            await browser.refresh()
          }
        }
        finally {
          if (current === generation) {
            busy.value = false
          }
        }
      }
    }
    catch (cause) {
      if (current === generation) {
        error.value = String(cause)
      }
    }
  }

  async function move(entries: SourceEntry[], directory: string): Promise<void> {
    await transfer('move', topEntries(entries).map(entry => ({ source: entry.path, target: join(directory, entry.name) })))
  }

  return { browser, selection, busy, error, results, clipboard, editing, editError, text, original, editPath, dirty, question, retryOperation, answer, save, reload, retryFailed, leave, open, action, submitName, updateName, move, importPaths }
}
