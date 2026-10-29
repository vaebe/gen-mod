import type { useSourceBrowser } from './useSourceBrowser'
import type { useSourceFileActions } from './useSourceFileActions'
import type { SourceEntry } from '@/contracts/source'
import { onScopeDispose, shallowRef, watch } from 'vue'

export interface SourceTreeEdit {
  mode: 'create' | 'rename'
  path: string
  initialName: string
  name: string
  directory: boolean
}

export function useSourceTreeActions(
  props: { modId: string, active: boolean },
  browser: ReturnType<typeof useSourceBrowser>,
  actions: ReturnType<typeof useSourceFileActions>,
  readDisabled: () => boolean,
) {
  const editing = shallowRef<SourceTreeEdit>()
  const editError = shallowRef('')
  const working = shallowRef(false)
  let generation = 0

  function reset(): void {
    generation++
    editing.value = undefined
    editError.value = ''
    working.value = false
  }
  watch(() => [props.modId, props.active], reset, { flush: 'sync' })
  onScopeDispose(reset)

  function cancel(): void {
    if (!working.value) {
      editing.value = undefined
      editError.value = ''
    }
  }

  async function beginCreate(path: string): Promise<void> {
    if (readDisabled() || working.value) {
      return
    }
    const current = generation
    working.value = true
    try {
      await browser.expandDirectory(path)
      if (current !== generation || readDisabled()) {
        return
      }
      browser.selectDirectory(path)
      editing.value = { mode: 'create', path, initialName: '', name: '', directory: true }
      editError.value = ''
    }
    finally {
      if (current === generation) {
        working.value = false
      }
    }
  }

  function beginRename(entry: SourceEntry): void {
    if (readDisabled() || working.value || entry.kind === 'blocked' || !entry.path) {
      return
    }
    browser.focusEntry(entry)
    editing.value = { mode: 'rename', path: entry.path, initialName: entry.name, name: entry.name, directory: entry.kind === 'directory' }
    editError.value = ''
  }

  async function move(source: string, targetDirectory: string, targetName: string): Promise<boolean> {
    const current = generation
    const result = await actions.moveFile(source, targetDirectory, targetName)
    if (current !== generation) {
      return false
    }
    const item = result?.items.find(item => item.source === source)
    if (item?.status !== 'moved') {
      actions.error.value ||= item?.message || '未完成移动，请刷新核对后重试。'
      await browser.refresh()
      return false
    }
    browser.remapPaths(source, item.target)
    actions.error.value = item.message ?? ''
    await browser.expandDirectory(targetDirectory)
    if (current === generation) {
      await browser.refresh()
    }
    return current === generation
  }

  async function submit(name: string): Promise<void> {
    const edit = editing.value
    if (!edit || readDisabled() || working.value) {
      return
    }
    if (edit.mode === 'rename' && name === edit.initialName) {
      cancel()
      return
    }
    if (!name.trim() || /[/\\]/u.test(name)) {
      editError.value = '请输入单层名称，不要包含目录分隔符。'
      return
    }
    const current = generation
    working.value = true
    editError.value = ''
    let success = false
    try {
      if (edit.mode === 'create') {
        const result = await actions.createDirectory(edit.path, name)
        if (current !== generation) {
          return
        }
        await browser.refresh()
        if (result && current === generation) {
          browser.selectDirectory(result.path)
          success = true
        }
      }
      else {
        success = await move(edit.path, edit.path.split('/').slice(0, -1).join('/'), name)
      }
      if (current === generation) {
        if (success) {
          editing.value = undefined
        }
        else {
          editError.value = actions.error.value || '操作未完成，请重试。'
        }
      }
    }
    finally {
      if (current === generation) {
        working.value = false
      }
    }
  }

  async function drop(entry: SourceEntry, targetDirectory: string): Promise<void> {
    if (readDisabled() || working.value || editing.value || entry.kind === 'blocked' || !entry.path) {
      return
    }
    const current = generation
    working.value = true
    try {
      await move(entry.path, targetDirectory, entry.name)
    }
    finally {
      if (current === generation) {
        working.value = false
      }
    }
  }

  function updateName(name: string): void {
    if (editing.value && !working.value) {
      editing.value = { ...editing.value, name }
    }
  }

  return { editing, editError, working, beginCreate, beginRename, cancel, submit, drop, updateName }
}
