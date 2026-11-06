import type { RuntimeSnapshot } from './types'
import { expect, it, vi } from 'vitest'
import { nativeCall } from '@/services/native'
import { pendingSubmission } from './pendingSubmission'
import { createSessionRuntime } from './sessionRuntime'

vi.mock('@/services/native', () => ({ nativeCall: vi.fn(), modService: {} }))
vi.mock('@/stores/projects', () => ({ useProjectsStore: vi.fn() }))
vi.mock('./pendingSubmission', () => ({ pendingSubmission: { remove: vi.fn().mockResolvedValue(undefined) } }))
vi.mock('@tauri-apps/api/core', () => ({ Channel: class {} }))

const snapshot: RuntimeSnapshot = { modId: 'p', sessionId: 's', generation: 0, sequence: 0, subscription: 0, connected: false, active: false, running: false, stopping: false, compacting: false, task: null, messages: [], tools: [] }

it('恢复失败后阻止新提交和引导，成功重新核对后解锁', async () => {
  const runtime = createSessionRuntime()
  runtime.modId.value = 'p'
  runtime.sessionId.value = 's'
  runtime.modelState.value = { selection: { providerId: 'provider', modelId: 'model' }, error: '', thinkingLevel: 'medium', thinkingLevels: ['medium'] }
  runtime.client.select('p', 's', 0)
  vi.mocked(nativeCall).mockRejectedValue(new Error('恢复失败'))
  await expect(runtime.recover(true)).rejects.toThrow('恢复失败')
  expect(runtime.client.synchronizing.value).toBe(false)
  expect(runtime.busy.value).toBe(true)
  await expect(runtime.send('继续')).rejects.toThrow('会话状态尚未确认')
  runtime.client.active.value = true
  expect(runtime.canSteer.value).toBe(false)
  vi.mocked(nativeCall).mockImplementation(async (command) => {
    if (command === 'reserve_pi_subscription') {
      return { generation: 0, ticket: 0 }
    }
    return command === 'read_pi_runtime' ? snapshot : []
  })
  await runtime.recover(true)
  expect(runtime.busy.value).toBe(false)
  expect(runtime.connectionError.value).toBe('')
})

it('磁盘历史携带持久请求关联时移除本地重复占位并清理保留提交', async () => {
  const runtime = createSessionRuntime()
  runtime.modId.value = 'p'
  runtime.sessionId.value = 's'
  runtime.client.select('p', 's', 0)
  runtime.pendingInput.value = { requestId: 'request', modId: 'p', sessionId: 's', text: '继续', message: '继续', attachments: [], images: [], timestamp: 10 }
  runtime.chat.value.messages = [{ id: 'request', role: 'user', parts: [{ type: 'text', text: '继续' }], final: false, inputStatus: 'unknown' }]
  vi.mocked(nativeCall).mockImplementation(async (command) => {
    if (command === 'reserve_pi_subscription') {
      return { generation: 0, ticket: 0 }
    }
    if (command === 'read_pi_runtime') {
      return { ...snapshot, messages: [{ role: 'user', timestamp: 20, content: '继续', appInputId: 'request', appTaskId: 'task' }] }
    }
    return command === 'read_submission_result' ? { status: 'accepted', task: { status: 'finished', context: { taskId: 'task' } } } : []
  })
  await runtime.recover(true)
  expect(runtime.chat.value.messages).toHaveLength(1)
  expect(runtime.chat.value.messages[0]).toMatchObject({ id: 'request', final: true, taskId: 'task' })
  expect(pendingSubmission.remove).toHaveBeenCalledWith('p', 's', 'request')
  expect(runtime.pendingInput.value).toBeUndefined()
})
