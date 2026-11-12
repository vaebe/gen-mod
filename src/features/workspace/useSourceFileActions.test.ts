import type { SourceMutationResult } from '@/contracts/source'
import { expect, it } from 'vitest'
import { effectScope, shallowRef } from 'vue'
import { useSourceFileActions } from './useSourceFileActions'

function fileService(importFiles: (args: { modId: string }) => Promise<unknown>) {
  return { importFiles, moveFile: importFiles, createDirectory: importFiles }
}

function deferred<T>() {
  let resolve!: (value: T) => void
  let reject!: (reason: Error) => void
  const promise = new Promise<T>((done, fail) => {
    resolve = done
    reject = fail
  })
  return { promise, resolve, reject }
}

const saved: SourceMutationResult = { operationId: 'op', sourceRevision: 1, items: [{ source: 'C:/input.bin', target: 'input.bin', status: 'imported' }] }

it('切项目后丢弃非空的旧导入结果，且不清除新项目忙状态', async () => {
  const pendingA = deferred<SourceMutationResult>()
  const pendingB = deferred<SourceMutationResult>()
  const modId = shallowRef('a')
  const scope = effectScope()
  const actions = scope.run(() => useSourceFileActions(() => modId.value, fileService(args => args.modId === 'a' ? pendingA.promise : pendingB.promise)))!
  try {
    const a = actions.importFiles('', [{ sourcePath: 'C:/input.bin', targetName: 'input.bin' }])
    modId.value = 'b'
    const b = actions.importFiles('', [{ sourcePath: 'C:/input.bin', targetName: 'input.bin' }])
    pendingA.resolve(saved)
    await a
    expect(actions.results.value).toEqual([])
    expect(actions.busy.value).toBe(true)
    pendingB.resolve(saved)
    await b
    expect(actions.results.value).toEqual(saved.items)
    expect(actions.busy.value).toBe(false)
  }
  finally { scope.stop() }
})

it('部分失败保留成功项和冲突项，错误后可重试', async () => {
  const scope = effectScope()
  let fail = true
  const partial: SourceMutationResult = { ...saved, items: [...saved.items, { source: 'C:/other.bin', target: 'input.bin', status: 'conflict', message: '已存在' }] }
  const actions = scope.run(() => useSourceFileActions(() => 'a', fileService(async () => {
    if (fail) {
      throw new Error('项目正忙')
    }
    return partial
  })))!
  try {
    await actions.importFiles('', [{ sourcePath: 'C:/input.bin', targetName: 'input.bin' }])
    expect(actions.error.value).toBe('项目正忙')
    expect(actions.busy.value).toBe(false)
    fail = false
    await actions.importFiles('', [{ sourcePath: 'C:/input.bin', targetName: 'input.bin' }])
    expect(actions.error.value).toBe('')
    expect(actions.results.value).toEqual(partial.items)
  }
  finally { scope.stop() }
})

it('卸载后不接纳迟到结果，也不将无效原生响应视为成功', async () => {
  const pending = deferred<SourceMutationResult>()
  const scope = effectScope()
  const actions = scope.run(() => useSourceFileActions(() => 'a', fileService(() => pending.promise)))!
  const operation = actions.importFiles('', [{ sourcePath: 'C:/input.bin', targetName: 'input.bin' }])
  scope.stop()
  pending.resolve(saved)
  await operation
  expect(actions.results.value).toEqual([])
  const other = effectScope()
  const invalid = other.run(() => useSourceFileActions(() => 'a', fileService(async () => ({ items: [] }))))!
  try {
    await invalid.importFiles('', [{ sourcePath: 'C:/input.bin', targetName: 'input.bin' }])
    expect(invalid.error.value).toBeTruthy()
    expect(invalid.results.value).toEqual([])
  }
  finally { other.stop() }
})
