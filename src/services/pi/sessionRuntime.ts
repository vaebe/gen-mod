import type { ModelSelection, SessionModelState } from '@/contracts/config'
import type { ContextUsage, ImageInput, PromptAttachment, SteeringInput, ThinkingLevel } from '@/contracts/session'
import type { TaskContext } from '@/contracts/task'
import type { PendingSubmission } from '@/services/pi/pendingSubmission'
import type { PiEnvelope, PiRecord, RuntimeSnapshot, SubmissionResult } from '@/services/pi/types'
import { computed, shallowRef } from 'vue'

import { modService, nativeCall } from '@/services/native'
import { createPiClient } from '@/services/pi/client'
import { pendingSubmission } from '@/services/pi/pendingSubmission'
import { emptyChat, reduceRecord, restoreChat } from '@/services/pi/reducer'
import { useProjectsStore } from '@/stores/projects'

/** 页面卸载不结束任务；通信、事件归并与活动状态归此单一 store。 */
export function createSessionRuntime() {
  const client = createPiClient()
  const initialized = shallowRef(false)
  const pendingInput = shallowRef<PendingSubmission>()
  const uncertain = shallowRef(false)
  const runtimeConfirmed = shallowRef(false)
  const pendingStatus = shallowRef<SubmissionResult['status']>('unknown')
  const connectionError = client.connectionError
  const attachments = shallowRef<PromptAttachment[]>([])
  const steeringInputs = shallowRef<SteeringInput[]>([])
  const stopping = shallowRef(false)
  const steeringSubmitting = shallowRef(false)
  const chat = shallowRef(emptyChat())
  const modId = shallowRef('')
  const sessionId = shallowRef('')
  const task = shallowRef<TaskContext>()
  const error = shallowRef('')
  const diagnostics = shallowRef<string[]>([])
  const uiRequest = shallowRef<PiRecord>()
  const changeRevision = shallowRef(0)
  const artifactRevision = shallowRef(0)
  const verificationRevision = shallowRef(0)
  const memoryRevision = shallowRef(0)
  const submitting = shallowRef(false)
  const switching = shallowRef(false)
  const compacting = shallowRef(false)
  const manualCompaction = shallowRef(false)
  const changingThinking = shallowRef(false)
  const changingModel = shallowRef(false)
  const modelState = shallowRef<SessionModelState>()
  const modelChangeError = shallowRef('')
  let modelEpoch = 0
  const thinkingLevel = shallowRef<ThinkingLevel>('medium')
  const thinkingLevels = shallowRef<ThinkingLevel[]>([])
  const contextUsage = shallowRef<ContextUsage>()
  const autoCompaction = shallowRef(true)
  const activity = shallowRef('正在准备…')
  const lastActivityAt = shallowRef(0)
  const startedAt = shallowRef<number>()
  const endedAt = shallowRef<number>()
  const readOnly = shallowRef(false)
  const canSteer = computed(() => runtimeConfirmed.value && client.active.value && !submitting.value && !compacting.value && !stopping.value && !steeringSubmitting.value && !readOnly.value && !client.synchronizing.value && !uncertain.value)
  const restartAvailable = shallowRef(false)
  const loadedSkills = shallowRef<string[]>([])
  let abortTimer: ReturnType<typeof setTimeout> | undefined
  const working = computed(() => submitting.value || client.active.value || client.submissionState.value === 'running' || client.submissionState.value === 'queued')
  const busy = computed(() => (!readOnly.value && !runtimeConfirmed.value) || uncertain.value || client.synchronizing.value || working.value || compacting.value || manualCompaction.value || switching.value || changingThinking.value || changingModel.value)
  let statsPending = false
  let statsAgain = false
  let statsEpoch = 0
  let inputEpoch = 0
  let queued: PiRecord[] = []
  let frame: number | undefined

  function flush(): void {
    if (frame !== undefined) {
      cancelAnimationFrame(frame)
      frame = undefined
    }
    let next = chat.value
    for (const record of queued) {
      next = reduceRecord(next, { ...record, taskId: record.taskId ?? task.value?.taskId })
    }
    queued = []
    chat.value = next
  }
  function receive(event: PiEnvelope): void {
    const record = event.record
    if (event.kind === 'record' && record.type === 'message_end' && record.message && (record.message as { role?: string }).role === 'user' && record.inputId === pendingInput.value?.requestId) {
      void clearPending().catch((cause) => {
        error.value = cause.message
      })
    }
    if (event.kind === 'task' && record.type === 'project_memory_updated' && !readOnly.value && event.modId === modId.value) {
      memoryRevision.value += 1
    }
    if (event.kind === 'task' && record.type === 'feedback_draft_updated') {
      verificationRevision.value += 1
    }
    if (event.kind === 'task' && record.type === 'steering_confirmed') {
      void refreshInputs().catch((cause) => {
        error.value = cause.message
      })
    }
    if (event.kind === 'record' && (record.type === 'steering_confirmed' || record.type === 'queue_update' || record.type === 'agent_settled' || (record.type === 'response' && record.command === 'steer') || (record.type === 'message_end' && record.inputId))) {
      void refreshInputs().catch((cause) => {
        error.value = cause.message
      })
    }
    if (event.kind === 'exit' && contextUsage.value) {
      contextUsage.value = { ...contextUsage.value, historical: true }
    }
    if (event.kind === 'exit') {
      void refreshInputs().catch((cause) => {
        error.value = cause.message
      })
    }
    if (event.kind === 'exit' && record.reason === 'unexpected') {
      error.value = 'pi 进程意外退出，任务已中断；请检查运行日志'
    }
    if (event.kind === 'exit') {
      changeRevision.value += 1
    }
    if (event.kind === 'exit') {
      stopping.value = false
      compacting.value = false
      endedAt.value ??= Date.now()
    }
    if (event.kind === 'task' && record.type === 'source_changed') {
      changeRevision.value += 1
      diagnostics.value = [...diagnostics.value.slice(-199), String(record.message)]
    }
    if (event.kind === 'task' && record.type === 'workspace_waiting') {
      activity.value = '等待共享源码操作'
      lastActivityAt.value = Date.now()
    }
    if (event.kind === 'task' && record.type === 'task_started') {
      startedAt.value = Date.parse((record.context as TaskContext).createdAt)
      if (chat.value.pendingUserId) {
        chat.value = { ...chat.value, messages: chat.value.messages.map(message => message.id === chat.value.pendingUserId ? { ...message, taskId: (record.context as TaskContext).taskId, timestamp: startedAt.value } : message) }
      }
      endedAt.value = undefined
      activity.value = '正在准备开发环境'
      lastActivityAt.value = Date.now()
    }
    if (event.kind === 'record') {
      if (record.type === 'agent_start' || record.type === 'turn_start') {
        activity.value = '等待模型响应…'
        lastActivityAt.value = Date.now()
      }
      if (['agent_start', 'message_update', 'tool_execution_start', 'tool_execution_update', 'tool_execution_end'].includes(record.type)) {
        lastActivityAt.value = Date.now()
      }
      if (record.type === 'message_update') {
        const update = record.assistantMessageEvent as { type?: string } | undefined
        if (update?.type === 'thinking_start' || update?.type === 'thinking_delta') {
          activity.value = '正在思考…'
        }
        else if (update?.type === 'thinking_end' || update?.type === 'start') {
          activity.value = '等待模型响应…'
        }
        else { activity.value = '正在整理回复' }
      }
      if (record.type === 'tool_execution_start') {
        const names: Record<string, string> = { read: '正在读取文件', write: '正在写入源码', edit: '正在修改源码', grep: '正在搜索代码', find: '正在查找文件', ls: '正在查看目录', run_command: '正在执行开发命令', prepare_delivery: '正在整理交付文件' }
        activity.value = names[String(record.toolName)] || '正在执行工具'
      }
      if (record.type === 'compaction_start') {
        compacting.value = true
        activity.value = '正在压缩上下文'
        lastActivityAt.value = Date.now()
      }
      if (record.type === 'compaction_end') {
        compacting.value = false
        if (!client.active.value) {
          stopping.value = false
        }
        statsEpoch += 1
        if (contextUsage.value) {
          contextUsage.value = { ...contextUsage.value, tokens: null, percent: null }
        }
        if (typeof record.errorMessage === 'string') {
          error.value = record.errorMessage
        }
        void refreshStats()
      }
      if (record.type === 'auto_retry_start') {
        activity.value = `正在重试连接 ${record.attempt ?? ''}/${record.maxAttempts ?? ''}`
        lastActivityAt.value = Date.now()
      }
      if (record.type === 'agent_settled') {
        stopping.value = false
        endedAt.value ??= Date.now()
        void refreshStats()
      }
      if (record.type === 'turn_end') {
        void refreshStats()
      }
      if (record.type === 'thinking_level_changed' && typeof record.level === 'string') {
        thinkingLevel.value = record.level as ThinkingLevel
      }
      if (record.type === 'response' && record.command === 'get_available_thinking_levels' && record.success && Array.isArray(record.data?.levels)) {
        thinkingLevels.value = record.data.levels as ThinkingLevel[]
      }
      if (record.type === 'response' && record.command === 'get_state' && record.success) {
        compacting.value = record.data?.isCompacting === true
        autoCompaction.value = record.data?.autoCompactionEnabled !== false
        if (typeof record.data?.thinkingLevel === 'string') {
          thinkingLevel.value = record.data.thinkingLevel as ThinkingLevel
        }
      }
    }
    if (event.kind === 'exit' || (event.kind === 'record' && record.type === 'agent_settled')) {
      if (abortTimer) {
        clearTimeout(abortTimer)
        abortTimer = undefined
      }
      restartAvailable.value = false
      if (event.kind === 'exit') {
        loadedSkills.value = []
      }
    }
    if (event.kind === 'artifact' || event.kind === 'exit' || (event.kind === 'record' && record.type === 'agent_settled')) {
      void modService.get(modId.value).then(value => useProjectsStore().upsert(value)).catch((cause) => {
        error.value = cause.message
      })
    }
    if (event.kind === 'artifact') {
      artifactRevision.value += 1
      changeRevision.value += 1
      return
    }
    if (event.kind === 'task' && record.type === 'task_started') {
      task.value = record.context as TaskContext
      return
    }
    if (event.kind === 'diagnostic') {
      if (record.type === 'stop_error') {
        error.value = String(record.message)
        restartAvailable.value = true
      }
      diagnostics.value = [...diagnostics.value.slice(-199), String(record.message ?? '运行时诊断')]
      return
    }
    if (event.kind === 'task' && record.type === 'changes_updated') {
      changeRevision.value += 1
      return
    }
    if (event.kind === 'exit') {
      flush()
      chat.value = reduceRecord(chat.value, { type: 'agent_settled', aborted: record.reason !== 'unexpected' })
      uiRequest.value = undefined
      return
    }
    if (event.kind !== 'record') {
      return
    }
    if (record.type === 'response' && record.command === 'get_state' && typeof record.data?.sessionId === 'string') {
      sessionId.value = record.data.sessionId
    }
    if (record.type === 'response' && record.command === 'get_commands' && Array.isArray(record.data?.commands)) {
      loadedSkills.value = (record.data.commands as { name?: string, source?: string }[]).filter(item => item.source === 'skill' && typeof item.name === 'string' && item.name.startsWith('skill:')).map(item => item.name!.slice(6))
    }
    if (record.type === 'extension_ui_request') {
      flush()
      uiRequest.value = record
      activity.value = '等待输入'
      lastActivityAt.value = Date.now()
      return
    }
    queued.push(record)
    if (record.type === 'message_end' || record.type === 'agent_settled') {
      flush()
    }
    else if (frame === undefined) {
      frame = requestAnimationFrame(flush)
    }
  }
  async function refreshInputs(): Promise<void> {
    if (!modId.value || !sessionId.value) {
      return
    }
    const epoch = ++inputEpoch
    const inputs = await nativeCall<SteeringInput[]>('list_steering_inputs', { modId: modId.value, sessionId: sessionId.value })
    if (epoch !== inputEpoch) {
      return
    }
    steeringInputs.value = inputs
    chat.value = { ...chat.value, messages: chat.value.messages.map((message) => {
      const submitted = steeringInputs.value.find(input => input.inputId === message.inputId)
      if (submitted && submitted.status !== 'confirmed') {
        return { ...message, inputStatus: submitted.status }
      }
      const input = steeringInputs.value.find(input => input.status === 'confirmed' && ((input.entryId && message.entryId === input.entryId) || (message.inputId === input.inputId)))
      return input ? { ...message, taskId: input.taskId, inputId: input.inputId, inputStatus: input.status } : message
    }) }
  }
  client.subscribe(receive)

  async function clearPending(): Promise<void> {
    const value = pendingInput.value
    if (!value) {
      return
    }
    await pendingSubmission.remove(value.modId, value.sessionId, value.requestId)
    if (pendingInput.value?.requestId === value.requestId) {
      pendingInput.value = undefined
      uncertain.value = false
    }
  }
  function pendingMessage(value: PendingSubmission) {
    return { id: value.requestId, role: 'user' as const, parts: [{ type: 'text' as const, text: value.message }, ...value.images], timestamp: value.timestamp, final: false, inputStatus: 'unknown' }
  }
  function restoreSnapshot(value: RuntimeSnapshot, full: boolean): void {
    flush()
    if (full) {
      let next = restoreChat(value.messages)
      for (const event of value.tools) {
        next = reduceRecord(next, event.record)
      }
      const local = chat.value.messages.filter(message => !message.final && ['unknown', 'failed', 'accepted'].includes(message.inputStatus ?? ''))
      for (const message of local) {
        if (!next.messages.some(item => item.id === message.id || (message.inputId && item.inputId === message.inputId))) {
          next.messages.push(message)
        }
      }
      const pending = pendingInput.value
      if (pending && !next.messages.some(message => message.id === pending.requestId || message.inputId === pending.requestId)) {
        next.messages.push(pendingMessage(pending))
      }
      chat.value = next
    }
    if (!value.running) {
      // 使用后端任务终态完成展示清理；无进程不能推断为成功。
      chat.value = reduceRecord(chat.value, { type: 'agent_settled', aborted: value.task?.status !== 'finished' })
    }
    task.value = value.task?.context
    stopping.value = value.stopping
    compacting.value = value.compacting
    if (value.task) {
      startedAt.value = Date.parse(value.task.context.createdAt)
      endedAt.value = value.task.finishedAt ? Date.parse(value.task.finishedAt) : undefined
    }
    if (value.running) {
      activity.value = value.stopping ? '正在停止…' : value.compacting ? '正在压缩上下文' : value.active ? '正在执行任务' : '等待模型响应…'
      lastActivityAt.value = Date.now()
    }
  }
  async function reconcileSubmission(): Promise<SubmissionResult | undefined> {
    const value = pendingInput.value
    if (!value) {
      uncertain.value = false
      return
    }
    const result = await nativeCall<SubmissionResult>('read_submission_result', { modId: value.modId, sessionId: value.sessionId, requestId: value.requestId })
    pendingStatus.value = result.status
    uncertain.value = result.status === 'unknown' || result.status === 'pending'
    if (result.status === 'accepted' && result.task) {
      const matched = chat.value.messages.some(message => message.inputId === value.requestId && message.final)
      if (matched) {
        await clearPending()
      }
      else {
        chat.value = { ...chat.value, messages: chat.value.messages.map(message => message.id === value.requestId ? { ...message, taskId: result.task!.context.taskId, inputStatus: 'accepted', error: result.task!.status === 'failed' ? (result.error || '任务执行失败，请查看执行过程') : undefined } : message) }
      }
    }
    if (result.status === 'rejected') {
      chat.value = { ...chat.value, messages: chat.value.messages.map(message => message.id === value.requestId ? { ...message, inputStatus: 'failed', error: result.error || '需求未被接收' } : message) }
    }
    return result
  }
  async function recover(force = false): Promise<SubmissionResult | undefined> {
    if (readOnly.value || !sessionId.value) {
      return
    }
    const attach = force || client.connection.value !== 'connected'
    runtimeConfirmed.value = false
    try {
      await client.synchronize(attach, value => restoreSnapshot(value, attach))
    }
    catch (cause) {
      if (attach || client.connection.value !== 'error') {
        throw cause
      }
      await client.synchronize(true, value => restoreSnapshot(value, true))
    }
    runtimeConfirmed.value = true
    const result = await reconcileSubmission()
    await refreshInputs()
    return result
  }
  async function open(projectId: string, id?: string, historyOnly = false): Promise<void> {
    switching.value = true
    runtimeConfirmed.value = false
    try {
      flush()
      chat.value = emptyChat()
      task.value = undefined
      diagnostics.value = []
      uiRequest.value = undefined
      modId.value = projectId
      // 连接失败也保留目标 ID，重新连接不能退化为新建会话。
      sessionId.value = id ?? ''
      error.value = ''
      readOnly.value = historyOnly
      loadedSkills.value = []
      contextUsage.value = undefined
      statsEpoch += 1
      thinkingLevels.value = []
      thinkingLevel.value = 'medium'
      modelEpoch += 1
      modelState.value = undefined
      modelChangeError.value = ''
      startedAt.value = undefined
      endedAt.value = undefined
      if (historyOnly && !id) {
        initialized.value = true
        return
      }
      const targetId = id ?? await nativeCall<string>('create_mod_session', { modId: projectId })
      sessionId.value = targetId
      const [history, metadata] = await Promise.all([
        nativeCall<{ messages: unknown[] }>('read_session_history', { modId: projectId, sessionId: targetId }),
        nativeCall<{ modelState: SessionModelState, contextUsage?: ContextUsage, generation: number }>('read_session_metadata', { modId: projectId, sessionId: targetId }),
      ])
      chat.value = restoreChat(history.messages)
      contextUsage.value = metadata.contextUsage ? { ...metadata.contextUsage, historical: true } : undefined
      applyModel(metadata.modelState)
      client.select(projectId, targetId, metadata.generation)
      pendingInput.value = await pendingSubmission.read(projectId, targetId)
      uncertain.value = !!pendingInput.value
      if (!historyOnly) {
        await recover(true)
      }
      await refreshInputs()
      initialized.value = true
    }
    finally { switching.value = false }
  }

  function applyModel(value: SessionModelState): void {
    modelState.value = value
    thinkingLevel.value = value.thinkingLevel
    thinkingLevels.value = value.thinkingLevels
  }

  async function refreshModel(): Promise<void> {
    if (busy.value || !initialized.value) {
      return
    }
    const projectId = modId.value
    const id = sessionId.value
    const epoch = ++modelEpoch
    const metadata = await nativeCall<{ modelState: SessionModelState }>('read_session_metadata', { modId: projectId, sessionId: id })
    if (epoch !== modelEpoch || projectId !== modId.value || id !== sessionId.value || busy.value) {
      return
    }
    if (client.connection.value === 'connected') {
      modelState.value = metadata.modelState
    }
    else { applyModel(metadata.modelState) }
  }

  async function selectModel(selection: ModelSelection): Promise<void> {
    if (busy.value || readOnly.value) {
      return
    }
    const projectId = modId.value
    const id = sessionId.value
    const epoch = ++modelEpoch
    changingModel.value = true
    error.value = ''
    restartAvailable.value = false
    try {
      const value = await client.selectModel(selection)
      if (epoch !== modelEpoch || projectId !== modId.value || id !== sessionId.value) {
        return
      }
      applyModel(value)
      modelChangeError.value = ''
      statsEpoch += 1
      contextUsage.value = undefined
      if (client.connection.value === 'connected') {
        await refreshStats()
      }
    }
    catch (cause) {
      if (epoch === modelEpoch && projectId === modId.value && id === sessionId.value) {
        modelChangeError.value = (cause as Error).message
        error.value = modelChangeError.value
        restartAvailable.value = true
      }
    }
    finally {
      if (epoch === modelEpoch) {
        changingModel.value = false
      }
    }
  }

  /** 事件触发的统计请求合并，旧连接的数据不得覆盖当前会话。 */
  async function refreshStats(): Promise<void> {
    if (readOnly.value || client.connection.value !== 'connected') {
      return
    }
    if (statsPending) {
      statsAgain = true
      return
    }
    statsPending = true
    const id = sessionId.value
    const generation = client.generation.value
    const epoch = statsEpoch
    try {
      const result = await client.request('get_session_stats')
      if (id !== sessionId.value || generation !== client.generation.value || epoch !== statsEpoch) {
        return
      }
      const usage = result.data?.contextUsage as ContextUsage | undefined
      contextUsage.value = usage && Number.isFinite(usage.contextWindow) && usage.contextWindow > 0 ? { ...usage, sampledAt: new Date().toISOString(), estimated: true, generation, historical: false } : undefined
    }
    catch { /* 统计暂不可用不能阻断正文；界面显示待更新。 */ }
    finally {
      statsPending = false
      if (statsAgain) {
        statsAgain = false
        void refreshStats()
      }
    }
  }

  /** 空闲时修改实际 pi 强度；新任务启动时将所选强度传给新的 RPC 进程。 */
  async function setThinking(level: ThinkingLevel): Promise<void> {
    if (busy.value || readOnly.value || !thinkingLevels.value.includes(level)) {
      return
    }
    changingThinking.value = true
    error.value = ''
    try {
      if (client.connection.value === 'connected') {
        await client.request('set_thinking_level', { level })
        await client.request('get_state')
      }
      await nativeCall('save_session_thinking', { modId: modId.value, sessionId: sessionId.value, level })
      thinkingLevel.value = level
    }
    catch (cause) { error.value = (cause as Error).message }
    finally { changingThinking.value = false }
  }

  /** 只有空闲可手动压缩；自动压缩在 pi 内继续运行。 */
  async function compact(): Promise<void> {
    if (busy.value || readOnly.value || !chat.value.messages.length) {
      return
    }
    compacting.value = true
    manualCompaction.value = true
    activity.value = '正在压缩上下文'
    error.value = ''
    try {
      if (client.connection.value !== 'connected') {
        await client.connect(modId.value, sessionId.value)
      }
      await client.request('compact', {}, 180_000)
      await refreshStats()
    }
    catch (cause) { error.value = (cause as Error).message }
    finally {
      // 超时不假定压缩已结束，重新查询权威状态；查询失败时保留忙碌状态。
      try {
        await client.request('get_state')
      }
      catch { /* 可使用现有重启入口恢复。 */ restartAvailable.value = true }
      manualCompaction.value = false
    }
  }

  async function close(): Promise<void> {
    if (busy.value) {
      throw new Error('请等待会话空闲后操作')
    }
    await client.disconnect()
    modId.value = ''
    sessionId.value = ''
    chat.value = emptyChat()
    contextUsage.value = undefined
  }

  /** 先显示本地提交，再保存草稿和启动任务；pi 用户事件沿用占位 ID。 */
  async function send(requirement: string, images: ImageInput[] = [], saveDraft?: () => Promise<boolean>, original?: { text: string, attachments: PromptAttachment[] }): Promise<void> {
    if (readOnly.value) {
      throw new Error('此会话只能查看历史，请新建会话')
    }
    if (!modelState.value || modelState.value.error || modelChangeError.value) {
      throw new Error(modelChangeError.value || modelState.value?.error || '请先选择模型')
    }
    if (images.length && !modelState.value.definition?.input.includes('image')) {
      throw new Error('当前模型不支持图片，请移除图片附件或切换模型')
    }
    if (pendingInput.value) {
      throw new Error('请先核对并处理保留的提交内容')
    }
    if (!runtimeConfirmed.value) {
      throw new Error('会话状态尚未确认，请重新连接后再提交')
    }
    if (working.value) {
      await steer(requirement, images, saveDraft)
      return
    }
    if (busy.value) {
      throw new Error('会话暂不能提交，请保留输入')
    }
    submitting.value = true
    stopping.value = false
    error.value = ''
    flush()
    const localId = crypto.randomUUID()
    const submission: PendingSubmission = { requestId: localId, modId: modId.value, sessionId: sessionId.value, text: original?.text ?? requirement, message: requirement, attachments: original?.attachments ?? [], images, timestamp: Date.now() }
    startedAt.value = Date.now()
    endedAt.value = undefined
    activity.value = '正在准备开发环境'
    lastActivityAt.value = Date.now()
    chat.value = { ...chat.value, pendingUserId: localId, messages: [...chat.value.messages, { id: localId, role: 'user', parts: [{ type: 'text', text: requirement }, ...images], final: false, timestamp: startedAt.value }] }
    try {
      if (saveDraft && !await saveDraft()) {
        throw new Error('需求保存失败，请重试')
      }
      await pendingSubmission.save(JSON.parse(JSON.stringify(submission)))
      pendingInput.value = submission
      task.value = await client.prompt(modId.value, sessionId.value, requirement, thinkingLevel.value, images, localId)
      pendingStatus.value = 'accepted'
      uncertain.value = false
      // 收到正常提交回执后仍保留输入，直到用户消息确认已进入历史。
      if (chat.value.messages.some(message => message.inputId === localId && message.final)) {
        await clearPending()
      }
      void refreshStats()
    }
    catch (cause) {
      flush()
      if (pendingInput.value?.requestId === localId) {
        try {
          const result = await recover()
          if (!pendingInput.value || result?.status === 'accepted') {
            error.value = result?.task?.status === 'failed' ? (result.error || '任务执行失败，请查看执行过程') : ''
            return
          }
          if (result?.status === 'rejected') {
            await clearPending()
          }
          else {
            uncertain.value = true
            throw new Error('提交结果未确认，请重新连接核对；输入已保留')
          }
        }
        catch (recoveryError) {
          if (pendingInput.value) {
            uncertain.value = true
            chat.value = { ...chat.value, messages: chat.value.messages.map(message => message.id === localId ? { ...message, inputStatus: 'unknown', error: '提交结果未确认，输入已保留' } : message) }
            throw new Error(`提交结果未确认；${(recoveryError as Error).message}`)
          }
        }
      }
      if (chat.value.pendingUserId === localId) {
        const uncertain = /超时|未确认/.test((cause as Error).message)
        chat.value = { ...chat.value, pendingUserId: uncertain ? localId : undefined, messages: chat.value.messages.map(message => message.id === localId ? { ...message, inputStatus: uncertain ? 'unknown' : 'failed', error: (cause as Error).message } : message) }
      }
      error.value = (cause as Error).message
      throw cause
    }
    finally { submitting.value = false }
  }
  /** 多条补充各有占位，正式关联由原生队列与内容确认。 */
  async function steer(message: string, images: ImageInput[], saveDraft?: () => Promise<boolean>): Promise<void> {
    if (!canSteer.value || !task.value) {
      throw new Error('任务暂不能接收引导，请保留输入')
    }
    steeringSubmitting.value = true
    const inputId = crypto.randomUUID()
    flush()
    chat.value = { ...chat.value, pendingInputs: [...(chat.value.pendingInputs ?? []), inputId], messages: [...chat.value.messages, { id: inputId, inputId, taskId: task.value.taskId, role: 'user', parts: [{ type: 'text', text: message }, ...images], final: false, timestamp: Date.now(), inputStatus: 'submitted' }] }
    try {
      if (saveDraft && !await saveDraft()) {
        throw new Error('需求保存失败，请重试')
      }
      const result = await client.steer(task.value.taskId, inputId, message, images)
      chat.value = { ...chat.value, messages: chat.value.messages.map(value => value.id === inputId ? { ...value, inputStatus: String(result.data?.disposition ?? 'unknown') } : value) }
      activity.value = '补充要求已提交'
      await refreshInputs()
    }
    catch (cause) {
      const uncertain = String((cause as Error).message).includes('未确认')
      chat.value = { ...chat.value, pendingInputs: chat.value.pendingInputs?.filter(id => id !== inputId), messages: chat.value.messages.map(value => value.id === inputId && !value.final && !['confirmed', 'handled', 'queued', 'started'].includes(value.inputStatus ?? '') ? { ...value, inputStatus: uncertain ? 'unknown' : 'failed', error: (cause as Error).message } : value) }
      error.value = (cause as Error).message
      throw cause
    }
    finally { steeringSubmitting.value = false }
  }
  async function abort(): Promise<void> {
    stopping.value = true
    if (abortTimer) {
      clearTimeout(abortTimer)
    }
    abortTimer = setTimeout(() => {
      if (working.value || compacting.value || manualCompaction.value) {
        restartAvailable.value = true
        error.value = '停止后会话仍忙碌，可重启运行时回收进程'
      }
    }, 30_000)
    try {
      await client.request('abort')
    }
    catch (cause) {
      error.value = (cause as Error).message
      restartAvailable.value = true
    }
  }
  async function restart(): Promise<void> {
    const project = modId.value
    const id = sessionId.value
    await client.disconnect()
    flush()
    await open(project, id)
  }
  return { initialized, connectionError, pendingInput, pendingStatus, uncertain, recover, clearPending, attachments, steeringInputs, stopping, canSteer, steeringSubmitting, client, chat, modId, sessionId, task, error, diagnostics, uiRequest, changeRevision, artifactRevision, verificationRevision, memoryRevision, readOnly, restartAvailable, loadedSkills, working, busy, switching, compacting, changingThinking, changingModel, modelState, modelChangeError, thinkingLevel, thinkingLevels, contextUsage, autoCompaction, activity, lastActivityAt, startedAt, endedAt, open, send, abort, restart, flush, refreshStats, refreshModel, selectModel, setThinking, compact, close }
}
