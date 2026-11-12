import type { RuntimeSnapshot } from '@/services/pi/types'
import { createPinia, setActivePinia } from 'pinia'
import { beforeEach, expect, it, vi } from 'vitest'
import { useSessionStore } from '@/features/sessions/useSessionStore'
import { nativeCall } from '@/services/native'
import { followupStorage, pendingSubmission } from './pendingSubmission'
import { createSessionRuntime } from './sessionRuntime'

vi.mock('@/services/native', () => {
  const nativeCall = vi.fn()
  return { nativeCall, modService: { get: (modId: string) => nativeCall('get_mod', { modId }) } }
})
vi.mock('@/stores/projects', () => ({ useProjectsStore: vi.fn() }))
vi.mock('./pendingSubmission', () => ({ pendingSubmission: { save: vi.fn().mockResolvedValue(undefined), read: vi.fn().mockResolvedValue(undefined), remove: vi.fn().mockResolvedValue(undefined) }, followupStorage: { save: vi.fn().mockResolvedValue(undefined), read: vi.fn().mockResolvedValue(undefined) } }))
vi.mock('@tauri-apps/api/event', () => ({ listen: vi.fn().mockResolvedValue(() => {}) }))
vi.mock('@tauri-apps/api/core', () => ({ Channel: class {} }))

const snapshot: RuntimeSnapshot = { modId: 'p', sessionId: 's', generation: 0, sequence: 0, subscription: 0, connected: false, active: false, running: false, stopping: false, compacting: false, task: null, messages: [], tools: [] }

const modelState = { selection: { providerId: 'provider', modelId: 'model' }, error: '', thinkingLevel: 'medium', thinkingLevels: ['medium'] }
function nativeResult(command: string, args?: Record<string, unknown>): unknown {
  if (command === 'read_session_history') {
    return { messages: [] }
  }
  if (command === 'read_session_metadata') {
    return { generation: 0, modelState }
  }
  if (command === 'reserve_pi_subscription') {
    return { generation: 0, ticket: 0 }
  }
  if (command === 'read_pi_runtime') {
    return { ...snapshot, modId: args?.modId, sessionId: args?.sessionId }
  }
  return []
}
beforeEach(() => {
  vi.mocked(pendingSubmission.read).mockResolvedValue(undefined)
  vi.mocked(followupStorage.read).mockResolvedValue(undefined)
  vi.mocked(nativeCall).mockImplementation(async (command, args) => nativeResult(command, args))
})

it('恢复失败后阻止新提交和引导，成功重新核对后解锁', async () => {
  const runtime = createSessionRuntime()
  await runtime.open('p', 's')
  vi.mocked(nativeCall).mockRejectedValue(new Error('恢复失败'))
  await expect(runtime.recover(true)).rejects.toThrow('恢复失败')
  expect(runtime.synchronizing.value).toBe(false)
  expect(runtime.busy.value).toBe(true)
  await expect(runtime.submitInput('继续', () => ({ save: async () => true, finish: async () => true }))).rejects.toThrow('会话状态尚未确认')
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
  vi.mocked(pendingSubmission.read).mockResolvedValueOnce({ requestId: 'request', modId: 'p', sessionId: 's', text: '继续', message: '继续', attachments: [], images: [], timestamp: 10 })
  vi.mocked(nativeCall).mockImplementation(async (command) => {
    if (command === 'reserve_pi_subscription') {
      return { generation: 0, ticket: 0 }
    }
    if (command === 'read_pi_runtime') {
      return { ...snapshot, messages: [{ role: 'user', timestamp: 20, content: '继续', appInputId: 'request', appTaskId: 'task' }] }
    }
    return command === 'read_submission_result' ? { status: 'accepted', task: { status: 'finished', context: { taskId: 'task' } } } : nativeResult(command)
  })
  await runtime.open('p', 's')
  expect(runtime.chat.value.messages).toHaveLength(1)
  expect(runtime.chat.value.messages[0]).toMatchObject({ id: 'request', final: true, taskId: 'task' })
  expect(pendingSubmission.remove).toHaveBeenCalledWith('p', 's', 'request')
  expect(runtime.pendingInput.value).toBeUndefined()
})

it('扩展请求回应携带原会话身份且不等待普通 RPC 回执', async () => {
  const runtime = createSessionRuntime()
  let channel: { onmessage: (event: import('@/services/pi/types').PiEnvelope) => void } | undefined
  vi.mocked(nativeCall).mockImplementation(async (command, args) => {
    if (command === 'read_pi_runtime') {
      channel = args?.onEvent as typeof channel
      return { ...snapshot, modId: 'project', sessionId: 'session', generation: 7 }
    }
    if (command === 'reserve_pi_subscription') {
      return { generation: 7, ticket: 0 }
    }
    return nativeResult(command, args)
  })
  await runtime.open('project', 'session')
  channel!.onmessage({ modId: 'project', sessionId: 'session', generation: 7, subscription: 0, sequence: 1, kind: 'record', record: { type: 'extension_ui_request', id: 'question', method: 'confirm', title: '继续？' } })
  const request = runtime.extensionRequest.value!
  expect(request).toMatchObject({ modId: 'project', sessionId: 'session', generation: 7, id: 'question', title: '继续？' })
  await runtime.respondExtensionRequest(request, { confirmed: false })
  expect(nativeCall).toHaveBeenLastCalledWith('send_pi_command', { modId: 'project', sessionId: 'session', generation: 7, command: { type: 'extension_ui_response', id: 'question', confirmed: false } })
  expect(runtime.extensionRequest.value).toBeUndefined()
  vi.mocked(nativeCall).mockClear()
  await runtime.respondExtensionRequest(request, { confirmed: true })
  expect(nativeCall).not.toHaveBeenCalled()
})

it('切换会话后旧请求及回应失败仍归原会话，不清除新会话请求', async () => {
  setActivePinia(createPinia())
  const session = useSessionStore()
  const channels = new Map<string, { onmessage: (event: import('@/services/pi/types').PiEnvelope) => void }>()
  let rejectResponse: (cause: Error) => void = () => undefined
  vi.mocked(nativeCall).mockImplementation(async (command, args) => {
    if (command === 'read_session_history') {
      return { messages: [] }
    }
    if (command === 'read_session_metadata') {
      return { generation: 7, modelState: { thinkingLevel: 'medium', thinkingLevels: ['medium'] } }
    }
    if (command === 'reserve_pi_subscription') {
      return { generation: 7, ticket: 1 }
    }
    if (command === 'read_pi_runtime') {
      channels.set(String(args?.sessionId), args?.onEvent as { onmessage: (event: import('@/services/pi/types').PiEnvelope) => void })
      return { ...snapshot, modId: args?.modId, sessionId: args?.sessionId, generation: 7, subscription: 1 }
    }
    if (command === 'send_pi_command') {
      return new Promise((_, reject) => {
        rejectResponse = reject
      })
    }
    return []
  })
  const emit = (id: string) => channels.get(id)!.onmessage({ modId: 'project', sessionId: id, generation: 7, subscription: 1, sequence: 1, kind: 'record', record: { type: 'extension_ui_request', id: 'same-question', method: 'input' } })
  await session.open('project', 'first')
  emit('first')
  const first = session.extensionRequest!
  const response = session.respondExtensionRequest(first, { value: '原会话回答' })
  await session.open('project', 'second')
  emit('second')
  const second = session.extensionRequest
  rejectResponse(new Error('连接已被替换'))
  await expect(response).rejects.toThrow('连接已被替换')
  expect(session.extensionRequest).toBe(second)
  expect(session.error).toBe('')
  expect(nativeCall).toHaveBeenCalledWith('send_pi_command', { modId: 'project', sessionId: 'first', generation: 7, command: { type: 'extension_ui_response', id: 'same-question', value: '原会话回答' } })
  await session.open('project', 'first')
  expect(session.extensionRequest).toBeUndefined()
  expect(session.error).toBe('连接已被替换')
})

it('输入业务操作在未发送失败后恢复附件并完成原草稿恢复', async () => {
  const runtime = createSessionRuntime()
  const file = { id: 'file', type: 'text' as const, name: 'note.txt', text: '附件内容', size: 4 }
  runtime.addAttachments([file])
  const finish = vi.fn().mockResolvedValue(true)
  await expect(runtime.submitInput('需求', () => ({ save: vi.fn().mockResolvedValue(true), finish }))).rejects.toThrow('请先选择模型')
  expect(runtime.attachments.value).toEqual([file])
  expect(finish).toHaveBeenCalledWith(true, false)
})

it.each(['accepted', 'unknown'] as const)('响应丢失后核对原提交 %s，不重复启动且保留恢复副本', async (status) => {
  const runtime = createSessionRuntime()
  const context = { taskId: 'task', sessionId: 's', modId: 'p', generation: 1, createdAt: '2026-11-12T00:00:00Z' }
  vi.mocked(nativeCall).mockClear().mockImplementation(async (command) => {
    if (command === 'reserve_pi_subscription') {
      return { generation: 1, ticket: 1 }
    }
    if (command === 'read_pi_runtime') {
      return { ...snapshot, generation: 1, subscription: 1 }
    }
    if (command === 'start_mod_task') {
      throw new Error('响应连接丢失')
    }
    if (command === 'read_submission_result') {
      return { status, task: status === 'accepted' ? { status: 'running', context } : undefined }
    }
    return nativeResult(command)
  })
  await runtime.open('p', 's')
  const file = { id: 'file', type: 'text' as const, name: 'note.txt', text: '附件内容', size: 4 }
  runtime.addAttachments([file])
  const finish = vi.fn().mockResolvedValue(true)
  const operation = runtime.submitInput('需求', () => ({ save: vi.fn().mockResolvedValue(true), finish }))
  if (status === 'unknown') {
    await expect(operation).rejects.toThrow('提交结果未确认')
  }
  else {
    await operation
  }
  expect(vi.mocked(nativeCall).mock.calls.filter(([command]) => command === 'start_mod_task')).toHaveLength(1)
  expect(runtime.pendingStatus.value).toBe(status)
  expect(runtime.pendingInput.value?.attachments).toEqual([file])
  expect(runtime.attachments.value).toEqual([])
  expect(finish).toHaveBeenCalledWith(false, status === 'unknown')
})

it('队列保存失败保留输入与附件，仍保持暂停', async () => {
  const runtime = createSessionRuntime()
  vi.mocked(followupStorage.read).mockResolvedValueOnce({ items: [{ id: 'queued', text: '前一条', message: '前一条', attachments: [], images: [], status: 'pending' }], paused: true })
  await runtime.open('p', 's')
  const file = { id: 'file', type: 'text' as const, name: 'note.txt', text: '附件内容', size: 4 }
  runtime.addAttachments([file])
  vi.mocked(followupStorage.save).mockRejectedValueOnce(new Error('队列保存失败'))
  const finish = vi.fn().mockResolvedValue(true)
  await expect(runtime.submitInput('后一条', () => ({ save: vi.fn().mockResolvedValue(true), finish }))).rejects.toThrow('队列保存失败')
  expect(runtime.attachments.value).toEqual([file])
  expect(runtime.followups.items.value.map(item => item.text)).toEqual(['前一条'])
  expect(runtime.followups.paused.value).toBe(true)
  expect(finish).toHaveBeenCalledWith(true, false)
})

it('拒绝提交恢复草稿保存失败时保留独立副本，保存成功后清除', async () => {
  const runtime = createSessionRuntime()
  const file = { id: 'file', type: 'text' as const, name: 'note.txt', text: '附件内容', size: 4 }
  vi.mocked(pendingSubmission.read).mockResolvedValueOnce({ requestId: 'request', modId: 'p', sessionId: 's', text: '原需求', message: '原需求', attachments: [file], images: [], timestamp: 10 })
  vi.mocked(nativeCall).mockImplementation(async (command, args) => command === 'read_submission_result' ? { status: 'rejected' } : nativeResult(command, args))
  await runtime.open('p', 's')
  let draft = ''
  await runtime.restorePendingInput((text) => {
    draft = text
    return async () => false
  })
  expect(draft).toBe('原需求')
  expect(runtime.attachments.value).toEqual([file])
  expect(runtime.pendingInput.value?.requestId).toBe('request')
  await runtime.restorePendingInput(() => async () => true)
  expect(runtime.pendingInput.value).toBeUndefined()
  expect(runtime.attachments.value).toEqual([file])
})

it('切换会话期间队列提交失败只恢复原会话附件与草稿', async () => {
  setActivePinia(createPinia())
  const session = useSessionStore()
  vi.mocked(nativeCall).mockImplementation(async (command, args) => {
    if (command === 'read_session_history') {
      return { messages: [] }
    }
    if (command === 'read_session_metadata') {
      return { generation: 0, modelState: { thinkingLevel: 'medium', thinkingLevels: ['medium'] } }
    }
    if (command === 'reserve_pi_subscription') {
      return { generation: 0, ticket: 0 }
    }
    if (command === 'read_pi_runtime') {
      return { ...snapshot, modId: args?.modId, sessionId: args?.sessionId }
    }
    return []
  })
  vi.mocked(followupStorage.read).mockResolvedValueOnce({ items: [{ id: 'queued', text: '前一条', message: '前一条', attachments: [], images: [], status: 'pending' }], paused: true })
  await session.open('project', 'first')
  const file = { id: 'file', type: 'text' as const, name: 'note.txt', text: '原附件', size: 4 }
  session.addAttachments([file])
  let completeSave!: (saved: boolean) => void
  const finish = vi.fn().mockResolvedValue(true)
  const operation = session.submitInput({ modId: 'project', sessionId: 'first' }, '原需求', () => ({ save: () => new Promise(resolve => completeSave = resolve), finish }))
  const failure = expect(operation).rejects.toThrow('草稿保存失败')
  await session.open('project', 'second')
  const nextFile = { ...file, text: '新附件' }
  session.addAttachments([nextFile])
  completeSave(false)
  await failure
  expect(session.attachments).toEqual([nextFile])
  expect(finish).toHaveBeenCalledWith(true, false)
  await session.open('project', 'first')
  expect(session.attachments).toEqual([file])
  expect(session.followups.paused).toBe(true)
})

it('停止立即暂停队列，即使停止通信失败也不继续投递', async () => {
  vi.useFakeTimers()
  try {
    const runtime = createSessionRuntime()
    vi.mocked(followupStorage.read).mockResolvedValueOnce({ items: [{ id: 'queued', text: '待发送', message: '待发送', attachments: [], images: [], status: 'pending' }], paused: false })
    await runtime.open('p', 's')
    vi.mocked(nativeCall).mockClear().mockRejectedValue(new Error('停止响应丢失'))
    const stopping = runtime.abort()
    expect(runtime.followups.paused.value).toBe(true)
    await stopping
    expect(runtime.followups.items.value[0]?.status).toBe('pending')
    expect(runtime.followups.paused.value).toBe(true)
    expect(vi.mocked(nativeCall).mock.calls.some(([command]) => command === 'start_mod_task')).toBe(false)
  }
  finally {
    vi.clearAllTimers()
    vi.useRealTimers()
  }
})

it('会话选择摘要保留后台运行，释放项目后不再暴露旧会话', async () => {
  setActivePinia(createPinia())
  const session = useSessionStore()
  vi.mocked(nativeCall).mockImplementation(async (command, args) => {
    if (command === 'read_session_history') {
      return { messages: [] }
    }
    if (command === 'read_session_metadata') {
      return { generation: 0, modelState: { thinkingLevel: 'medium', thinkingLevels: ['medium'] } }
    }
    if (command === 'reserve_pi_subscription') {
      return { generation: 0, ticket: 0 }
    }
    if (command === 'read_pi_runtime') {
      return { ...snapshot, modId: args?.modId, sessionId: args?.sessionId, running: args?.sessionId === 'first', active: args?.sessionId === 'first' }
    }
    return []
  })
  await session.open('project', 'first')
  await session.open('other', 'second')
  expect(session.isSelected({ modId: 'other', sessionId: 'second' })).toBe(true)
  expect(session.summaryFor('project', 'first')).toMatchObject({ initialized: true, running: true, busy: true })
  expect(session.isProjectBusy('project')).toBe(true)
  expect(vi.mocked(nativeCall).mock.calls.some(([command]) => command === 'disconnect_pi')).toBe(false)
  session.forgetProject('project')
  expect(session.summaryFor('project', 'first')).toBeUndefined()
  expect(session.isProjectBusy('project')).toBe(false)
  expect(session.isSelected({ modId: 'other', sessionId: 'second' })).toBe(true)
})

it('只读会话恢复历史任务而不连接运行时', async () => {
  const runtime = createSessionRuntime()
  vi.mocked(nativeCall).mockClear()
  vi.mocked(nativeCall).mockImplementation(async (command) => {
    if (command === 'read_session_history') {
      return { messages: [] }
    }
    if (command === 'read_session_metadata') {
      return { generation: 0, modelState: { thinkingLevel: 'medium', thinkingLevels: ['medium'] } }
    }
    if (command === 'list_mod_tasks') {
      return [{ context: { taskId: 'historical', sessionId: 'archived' } }]
    }
    return []
  })
  await runtime.open('project', 'archived', true)
  expect(runtime.task.value?.taskId).toBe('historical')
  expect(runtime.readOnly.value).toBe(true)
  expect(runtime.busy.value).toBe(false)
  expect(vi.mocked(nativeCall).mock.calls.some(([command]) => ['read_pi_runtime', 'connect_pi', 'reserve_pi_subscription'].includes(command))).toBe(false)
})

it('后台反馈更新按原项目共享给各视图，其他项目不接收', async () => {
  setActivePinia(createPinia())
  const { effectScope, nextTick, shallowRef } = await import('vue')
  const { useVerification } = await import('@/features/verification/useVerification')
  const session = useSessionStore()
  const channels = new Map<string, { onmessage: (event: import('@/services/pi/types').PiEnvelope) => void }>()
  let updated = false
  vi.mocked(nativeCall).mockImplementation(async (command, args) => {
    if (command === 'read_session_history') {
      return { messages: [] }
    }
    if (command === 'read_session_metadata') {
      return { generation: 7, modelState: { thinkingLevel: 'medium', thinkingLevels: ['medium'] } }
    }
    if (command === 'reserve_pi_subscription') {
      return { generation: 7, ticket: 1 }
    }
    if (command === 'read_pi_runtime') {
      channels.set(String(args?.sessionId), args?.onEvent as { onmessage: (event: import('@/services/pi/types').PiEnvelope) => void })
      return { ...snapshot, modId: args?.modId, sessionId: args?.sessionId, generation: 7, subscription: 1 }
    }
    if (command === 'list_verification') {
      return { drafts: [], records: updated && args?.modId === 'project' ? [{ recordId: 'new' }] : [] }
    }
    return []
  })
  const scope = effectScope()
  const viewed = shallowRef('project')
  const views = scope.run(() => [useVerification(() => viewed.value), useVerification(() => 'project'), useVerification(() => 'other')])!
  await session.open('project', 'first')
  await session.open('other', 'second')
  updated = true
  channels.get('first')!.onmessage({ modId: 'project', sessionId: 'first', generation: 7, subscription: 1, sequence: 1, kind: 'task', record: { type: 'feedback_draft_updated' } })
  await nextTick()
  await nextTick()
  expect(views[0]!.data.value.records).toEqual([{ recordId: 'new' }])
  expect(views[1]!.data.value.records).toEqual([{ recordId: 'new' }])
  expect(views[2]!.data.value.records).toEqual([])
  expect(session.modId).toBe('other')
  scope.stop()
})

it('原项目交付刷新丢弃较旧请求，失败留在能力错误中', async () => {
  setActivePinia(createPinia())
  const { useDeliveriesStore } = await import('@/features/delivery/useDeliveries')
  const { useProjectsStore } = await import('@/stores/projects')
  vi.mocked(useProjectsStore).mockReturnValue({ projects: [], upsert: vi.fn(), isRemoved: () => false } as unknown as ReturnType<typeof useProjectsStore>)
  const deliveries = useDeliveriesStore()
  const pending: ((value: unknown) => void)[] = []
  vi.mocked(nativeCall).mockImplementation(() => new Promise(resolve => pending.push(resolve)))
  const first = deliveries.refresh('original')
  const second = deliveries.refresh('original')
  pending[1]!({ modId: 'original', revision: 2, versions: [{ versionId: 'new' }] })
  await second
  pending[0]!({ modId: 'original', revision: 1, versions: [] })
  await first
  expect(deliveries.projectFor('original')?.versions).toEqual([{ versionId: 'new' }])
  expect(deliveries.projectFor('other')).toBeUndefined()
  vi.mocked(nativeCall).mockRejectedValueOnce(new Error('读取交付失败'))
  await expect(deliveries.refresh('original')).rejects.toThrow('读取交付失败')
  expect(deliveries.errorFor('original')).toBe('读取交付失败')
  expect(deliveries.errorFor('other')).toBe('')
})

it('原会话文件与记忆事件刷新同项目能力数据', async () => {
  setActivePinia(createPinia())
  const { effectScope, nextTick } = await import('vue')
  const { useTaskChanges } = await import('@/features/workspace/useTaskChanges')
  const { useProjectMemory } = await import('@/features/project-memory/useProjectMemory')
  let current = false
  vi.mocked(nativeCall).mockImplementation(async (command) => {
    if (command === 'list_task_changes') {
      return current ? [{ path: 'updated.txt' }] : []
    }
    if (command === 'get_project_context') {
      return { confirmedRequirements: current ? ['新需求'] : [] }
    }
    if (command === 'get_mod') {
      return { modId: 'original', revision: 1, versions: [], archived: false }
    }
    if (command === 'connect_pi') {
      return { modId: 'original', sessionId: 'background', generation: 3 }
    }
    return []
  })
  const { coordinateProjectEvent } = await import('@/application/projectRefresh')
  const scope = effectScope()
  const views = scope.run(() => ({ files: useTaskChanges(() => 'original', () => 'task'), memory: useProjectMemory(() => 'original', () => true) }))!
  await nextTick()
  await nextTick()
  current = true
  coordinateProjectEvent({ modId: 'original', sessionId: 'background', generation: 3, subscription: 0, sequence: 1, kind: 'task', record: { type: 'source_changed' } })
  coordinateProjectEvent({ modId: 'original', sessionId: 'background', generation: 3, subscription: 0, sequence: 2, kind: 'task', record: { type: 'project_memory_updated' } })
  await nextTick()
  await nextTick()
  await nextTick()
  expect(views.files.changes.value).toEqual([{ path: 'updated.txt' }])
  expect(views.memory.memory.value?.confirmedRequirements).toEqual(['新需求'])
  scope.stop()
})

it('切换项目再返回后旧反馈响应不能覆盖新视图，卸载后停止写入', async () => {
  const { effectScope, shallowRef } = await import('vue')
  const { useVerification } = await import('@/features/verification/useVerification')
  const pending: { resolve: (value: unknown) => void, reject: (cause: Error) => void }[] = []
  vi.mocked(nativeCall).mockImplementation(() => new Promise((resolve, reject) => pending.push({ resolve, reject })))
  const scope = effectScope()
  const project = shallowRef('first')
  const view = scope.run(() => useVerification(() => project.value))!
  project.value = 'second'
  project.value = 'first'
  pending[2]!.resolve({ drafts: [], records: [{ recordId: 'latest' }] })
  await Promise.resolve()
  pending[0]!.resolve({ drafts: [], records: [{ recordId: 'stale' }] })
  pending[1]!.reject(new Error('旧项目失败'))
  await Promise.resolve()
  expect(view.data.value.records).toEqual([{ recordId: 'latest' }])
  expect(view.error.value).toBe('')
  const refresh = view.refresh()
  scope.stop()
  pending[3]!.resolve({ drafts: [], records: [] })
  await refresh
  expect(view.data.value.records).toEqual([{ recordId: 'latest' }])
})

it('只读输入投影不能改写保留附件，输入操作仍可删除附件', async () => {
  const runtime = createSessionRuntime()
  const file = { id: 'file', type: 'text' as const, name: 'note.txt', text: '原内容', size: 3 }
  runtime.addAttachments([file])
  file.text = '外部引用改写'
  const warn = vi.spyOn(console, 'warn').mockImplementation(() => {})
  try {
    Reflect.set(runtime.attachments.value[0]!, 'text', '投影改写')
    Reflect.set(runtime.attachments, 'value', [])
    expect(runtime.attachments.value).toMatchObject([{ id: 'file', text: '原内容' }])
    runtime.removeAttachment('file')
    expect(runtime.attachments.value).toEqual([])
  }
  finally { warn.mockRestore() }
})
