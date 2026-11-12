import type { ModProject } from '@/contracts/mod'
import { expect, it, vi } from 'vitest'
import { effectScope, shallowRef } from 'vue'
import { useVersionOperations } from './useVersionOperations'

vi.mock('@/services/native', () => {
  const nativeCall = vi.fn()
  return { nativeCall, modService: { get: (modId: string) => nativeCall('get_mod', { modId }) } }
})
const { nativeCall } = await import('@/services/native')
const call = vi.mocked(nativeCall)
function project(modId = 'a'): ModProject {
  return { modId, revision: 1, archived: false, name: modId, gameId: '', gameSkillName: '', description: '', draftRequirement: '', updatedAt: '', developmentStatus: 'ready', latestVersionId: 'v2', versions: ['v1', 'v2'].map(versionId => ({ versionId, summary: '', sessionId: '', taskId: '', createdAt: '', skills: [], files: [], zipSha256: '' })), installations: [], installedTargets: [], sessions: [] }
}
function setup(afterDelete: 'latest' | 'clear' = 'latest') {
  call.mockReset()
  const current = shallowRef(project())
  const updated: ModProject[] = []
  const scope = effectScope()
  const actions = scope.run(() => useVersionOperations({ project: () => current.value, afterDelete, updateProject: value => updated.push(value) }))!
  return { current, updated, actions, stop: () => scope.stop() }
}
it('取消导出不显示成功提示', async () => {
  const test = setup()
  call.mockResolvedValue(null)
  await test.actions.act('export', 'v1')
  expect(call).toHaveBeenCalledWith('export_version', { modId: 'a', versionId: 'v1' })
  expect(test.actions.notice.value).toBe('')
  expect(test.actions.busy.value).toBe(false)
  test.stop()
})
it.each(['latest', 'clear'] as const)('删除成功后按 %s 策略选择，更新原项目', async (strategy) => {
  const test = setup(strategy)
  const result = { ...project(), versions: [project().versions[1]!], revision: 2 }
  call.mockResolvedValue(result)
  await test.actions.act('delete', 'v1')
  expect(call).not.toHaveBeenCalled()
  await test.actions.confirmDelete()
  expect(call).toHaveBeenCalledWith('delete_version', { modId: 'a', versionId: 'v1' })
  expect(test.updated).toEqual([result])
  expect(test.actions.selectedId.value).toBe(strategy === 'latest' ? 'v2' : '')
  expect(test.actions.deleting.value).toBe(false)
  test.stop()
})
it('删除失败但记录已变化时重新读取原项目，保留原始错误', async () => {
  const test = setup()
  const actual = { ...project(), versions: [], latestVersionId: undefined, revision: 2 }
  call.mockRejectedValueOnce(new Error('残留目录清理失败')).mockResolvedValueOnce(actual)
  await test.actions.act('delete', 'v1')
  await test.actions.confirmDelete()
  expect(call).toHaveBeenLastCalledWith('get_mod', { modId: 'a' })
  expect(test.updated).toEqual([actual])
  expect(test.actions.error.value).toBe('残留目录清理失败')
  expect(test.actions.notice.value).toBe('')
  expect(test.actions.selectedId.value).toBe('')
  test.stop()
})
it.each([false, true])('切换项目后迟到的删除结果仍更新原项目且不改新视图（失败=%s）', async (fail) => {
  const test = setup()
  let resolve!: (value: ModProject) => void
  let reject!: (error: Error) => void
  call.mockImplementationOnce(() => new Promise<ModProject>((done, error) => {
    resolve = done
    reject = error
  }))
  await test.actions.act('delete', 'v1')
  const pending = test.actions.confirmDelete()
  test.current.value = project('b')
  await test.actions.act('details', 'v2')
  const actual = { ...project(), versions: [], revision: 2 }
  if (fail) {
    call.mockResolvedValueOnce(actual)
    reject(new Error('清理失败'))
  }
  else { resolve(actual) }
  await pending
  expect(test.updated).toEqual([actual])
  expect(test.actions.selectedId.value).toBe('v2')
  expect(test.actions.targetProject.value?.modId).toBe('b')
  expect(test.actions.detailsOpen.value).toBe(true)
  expect(test.actions.error.value).toBe('')
  expect(test.actions.notice.value).toBe('')
  test.stop()
})
it('切项目后的旧导出结果不提示在新项目中', async () => {
  const test = setup()
  let resolve!: (value: string) => void
  call.mockImplementationOnce(() => new Promise<string>((done) => {
    resolve = done
  }))
  const pending = test.actions.act('export', 'v1')
  test.current.value = project('b')
  resolve('a.zip')
  await pending
  expect(test.actions.notice.value).toBe('')
  test.stop()
})
it('共享项目刷新后不保留已删除版本的详情或确认', async () => {
  const test = setup()
  await test.actions.act('details', 'v1')
  test.current.value = { ...project(), versions: [project().versions[1]!] }
  expect(test.actions.detailsOpen.value).toBe(false)
  expect(test.actions.selectedId.value).toBe('v2')
  expect(test.actions.selected.value?.versionId).toBe('v2')
  test.stop()
})
