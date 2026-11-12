import type { FollowupInput } from './followupQueue'
import type { ContextUsage, ImageInput, PromptAttachment, SteeringInput, ThinkingLevel } from '@/contracts/session'
import type { TaskContext } from '@/contracts/task'
import type { PendingSubmission } from '@/features/sessions/pendingSubmission'
import type { ModelSelection, SessionModelState } from '@/features/settings/types'
import type { ExtensionRequest, ExtensionResponse, PiEnvelope, PiRecord, RuntimeSnapshot, SubmissionResult } from '@/services/pi/types'
import { computed, readonly, shallowRef, watch } from 'vue'

import { emptyChat, reduceRecord, restoreChat } from '@/features/chat/reducer'
import { createFollowupQueue } from '@/features/sessions/followupQueue'
import { pendingSubmission } from '@/features/sessions/pendingSubmission'
import { sessionService } from '@/features/sessions/service'
import { createPiClient } from '@/services/pi/client'
import { InputNotSentError } from '@/services/pi/types'

/** 页面卸载不结束任务；通信、事件归并与活动状态归此单一 store。 */
export function createSessionRuntime(onProjectEvent: (event: PiEnvelope) => Promise<unknown> | void = () => {}) {
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
  let extensionIdentity = { modId: '', sessionId: '', generation: 0 }
  const extensionRequest = computed<Readonly<ExtensionRequest> | undefined>(() => {
    const request = uiRequest.value
    return request?.id ? readonly({ ...request, ...extensionIdentity, id: request.id }) : undefined
  })
  async function respondExtensionRequest(request: Readonly<ExtensionRequest>, response: ExtensionResponse): Promise<void> {
    if (request !== extensionRequest.value || request.generation !== client.generation.value) {
      return
    }
    // 先消费请求，重复点击或弹窗关闭事件不能再次发送同一回应。
    uiRequest.value = undefined
    try {
      await client.respondExtension(request, response)
    }
    catch (cause) {
      error.value = (cause as Error).message
      throw cause
    }
  }
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
  const queueReady = shallowRef(false)
  const followups = createFollowupQueue({
    identity: () => ({ modId: modId.value, sessionId: sessionId.value }),
    allowed: mode => mode === 'steer' ? canSteer.value : !busy.value && !readOnly.value && !pendingInput.value,
    validate: (item) => {
      if (!modelState.value || modelState.value.error || modelChangeError.value) {
        throw new Error(modelChangeError.value || modelState.value?.error || '请先选择可用模型')
      }
      if (item.images.length && !modelState.value.definition?.input.includes('image')) {
        throw new Error('当前模型不支持图片，请编辑消息移除图片或切换模型')
      }
    },
    dispatch: async (item, mode) => {
      if (mode === 'steer') {
        await steer(item.message, item.images, undefined, item.id)
      }
      else {
        await send(item.message, item.images, undefined, { text: item.text, attachments: item.attachments }, item.id)
      }
    },
    status: followupStatus,
    removed: (id) => {
      flush()
      chat.value = {
        ...chat.value,
        messages: chat.value.messages.filter(message => message.final || (message.id !== id && message.inputId !== id)),
        pendingUserId: chat.value.pendingUserId === id ? undefined : chat.value.pendingUserId,
        pendingInputs: chat.value.pendingInputs?.filter(value => value !== id),
      }
    },
  })
  async function followupStatus(item: FollowupInput): Promise<FollowupInput['status'] | 'confirmed'> {
    if (chat.value.messages.some(message => message.final && (message.inputId === item.id || message.id === item.id))) {
      return 'confirmed'
    }
    if (item.mode === 'steer') {
      const inputs = await sessionService.steeringInputs({ modId: modId.value, sessionId: sessionId.value })
      const input = inputs.find(input => input.inputId === item.id)
      if (input?.status === 'confirmed' || input?.status === 'handled') {
        return input.status
      }
      if (input?.status === 'rejected') {
        return 'failed'
      }
      return input && ['queued', 'submitted'].includes(input.status) ? 'accepted' : 'unknown'
    }
    const result = await sessionService.submission({ modId: modId.value, sessionId: sessionId.value, requestId: item.id })
    if (result.status === 'accepted' && result.task && ['failed', 'cancelled', 'interrupted'].includes(result.task.status)) {
      // 队列已保留原始输入，终态任务的重复保留提示不再阻止用户重新提交。
      if (pendingInput.value?.requestId === item.id) {
        await clearPending()
      }
      return 'interrupted'
    }
    return result.status === 'rejected' ? 'failed' : result.status === 'accepted' ? 'accepted' : 'unknown'
  }
  function queueError(cause: unknown): void {
    followups.error.value = (cause as Error).message
  }
  async function finishQueueTask(id?: string): Promise<void> {
    if (!id || task.value?.taskId !== id) {
      return
    }
    // 恢复快照会取得原生槽锁，等待结束事件对应的任务终态落盘。
    await recover()
  }
  watch(() => [queueReady.value, busy.value, followups.busy.value, followups.paused.value, followups.items.value, followups.editingId.value, pendingInput.value], () => {
    if (!queueReady.value || busy.value || followups.busy.value || followups.paused.value || followups.editingId.value || readOnly.value || pendingInput.value) {
      return
    }
    if (followups.items.value.some(item => ['sending', 'accepted', 'unknown', 'failed', 'interrupted'].includes(item.status))) {
      return
    }
    const next = followups.items.value.find(item => item.status === 'pending')
    if (next) {
      queueReady.value = false
      void followups.deliver(next.id, 'prompt', true).catch(async (cause) => {
        await followups.pause().catch(queueError)
        queueError(cause)
      })
    }
  })
  async function resumeQueue(): Promise<void> {
    if (readOnly.value) {
      return
    }
    await followups.resume()
    queueReady.value = !working.value
  }

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
    void Promise.resolve(onProjectEvent(event)).catch((cause) => {
      error.value = (cause as Error).message
    })
    if (event.kind === 'record' && record.type === 'message_end' && record.message && (record.message as { role?: string }).role === 'user' && record.inputId === pendingInput.value?.requestId) {
      void clearPending().catch((cause) => {
        error.value = cause.message
      })
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
      queueReady.value = false
      void followups.pause().catch(queueError)
      stopping.value = false
      compacting.value = false
      endedAt.value ??= Date.now()
    }
    if (event.kind === 'task' && record.type === 'source_changed') {
      diagnostics.value = [...diagnostics.value.slice(-199), String(record.message)]
    }
    if (event.kind === 'task' && record.type === 'workspace_waiting') {
      activity.value = '等待共享源码操作'
      lastActivityAt.value = Date.now()
    }
    if (event.kind === 'task' && record.type === 'task_started') {
      queueReady.value = false
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
        if (record.aborted === true) {
          void followups.pause().catch(queueError)
        }
        else {
          void finishQueueTask(task.value?.taskId).catch(queueError)
        }
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
    if (event.kind === 'artifact') {
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
      extensionIdentity = { modId: event.modId, sessionId: event.sessionId, generation: event.generation }
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
    const inputs = await sessionService.steeringInputs({ modId: modId.value, sessionId: sessionId.value })
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
    void followups.reconcile().catch(queueError)
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
    queueReady.value = !value.running && value.task?.status === 'finished'
    if (!value.running && value.task && value.task.status !== 'finished') {
      void followups.pause().catch(queueError)
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
    const result = await sessionService.submission({ modId: value.modId, sessionId: value.sessionId, requestId: value.requestId })
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
  function updateAccess(historyOnly: boolean): void {
    readOnly.value = historyOnly
  }
  // 项目已完成归档或删除，撤销订阅身份，不向已释放的项目发送停止命令。
  function release(): void {
    client.select('', '', 0)
    readOnly.value = true
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
      const targetId = id ?? await sessionService.create({ modId: projectId })
      sessionId.value = targetId
      const [history, metadata] = await Promise.all([
        sessionService.history({ modId: projectId, sessionId: targetId }),
        sessionService.metadata({ modId: projectId, sessionId: targetId }),
      ])
      chat.value = restoreChat(history.messages)
      contextUsage.value = metadata.contextUsage ? { ...metadata.contextUsage, historical: true } : undefined
      applyModel(metadata.modelState)
      client.select(projectId, targetId, metadata.generation)
      pendingInput.value = await pendingSubmission.read(projectId, targetId)
      await followups.load()
      uncertain.value = !!pendingInput.value
      if (!historyOnly) {
        await recover(true)
      }
      else {
        const tasks = await sessionService.tasks({ modId: projectId })
        task.value = tasks.find(item => item.context.sessionId === targetId)?.context
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
    try {
      const metadata = await sessionService.metadata({ modId: projectId, sessionId: id })
      if (epoch !== modelEpoch || projectId !== modId.value || id !== sessionId.value || busy.value) {
        return
      }
      if (client.connection.value === 'connected') {
        modelState.value = metadata.modelState
      }
      else { applyModel(metadata.modelState) }
    }
    catch (cause) { error.value = (cause as Error).message }
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
      await sessionService.saveThinking({ modId: modId.value, sessionId: sessionId.value, level })
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
    if (busy.value || followups.busy.value) {
      throw new Error('请等待会话空闲后操作')
    }
    await client.disconnect()
    modId.value = ''
    sessionId.value = ''
    chat.value = emptyChat()
    contextUsage.value = undefined
  }

  function addAttachments(files: PromptAttachment[]): void {
    if (!readOnly.value) {
      attachments.value = [...attachments.value, ...files.map(file => ({ ...file }))]
    }
  }
  function removeAttachment(id: string): void {
    if (!readOnly.value) {
      attachments.value = attachments.value.filter(file => file.id !== id)
    }
  }
  /** 提交与补偿绑定本会话；页面只提供已经捕获身份的草稿持久化操作。 */
  async function submitInput(text: string, captureDraft: () => { save: () => Promise<boolean>, finish: (restore: boolean, preserve: boolean) => Promise<boolean> }): Promise<boolean> {
    const selected = attachments.value
    const textFiles = selected.filter(file => file.type === 'text').map(file => `附件文件：${file.name}\n${file.text}`).join('\n\n')
    const message = [text.trim() || (selected.length ? '请根据引用和附件继续处理当前 MOD。' : ''), textFiles].filter(Boolean).join('\n\n')
    const images = selected.filter(file => file.type === 'image').map(file => ({ type: 'image' as const, data: file.data, mimeType: file.mimeType }))
    const previousIds = new Set(chat.value.messages.map(item => item.id))
    const enqueue = working.value || !!followups.editingId.value || followups.items.value.some(item => item.status === 'pending')
    const submission = captureDraft()
    attachments.value = attachments.value.filter(file => !selected.some(sent => sent.id === file.id))
    let restore = false
    let saved = true
    try {
      if (readOnly.value) {
        throw new InputNotSentError('此会话只能查看历史，请新建会话')
      }
      if (enqueue) {
        if (!await submission.save()) {
          throw new InputNotSentError('草稿保存失败，未加入队列')
        }
        await followups.add({ text, message, attachments: selected, images })
      }
      else {
        await send(message, images, submission.save, { text, attachments: selected })
      }
    }
    catch (cause) {
      const confirmed = chat.value.messages.some(item => !previousIds.has(item.id) && item.role === 'user' && (item.final || (item.inputStatus && !['submitted', 'failed', 'unknown'].includes(item.inputStatus))))
      restore = enqueue || (!confirmed && !uncertain.value && !/超时|未确认/.test((cause as Error).message))
      if (restore) {
        attachments.value = [...selected, ...attachments.value.filter(file => !selected.some(sent => sent.id === file.id))]
      }
      throw cause
    }
    finally {
      saved = await submission.finish(restore, uncertain.value)
    }
    return saved
  }
  async function editFollowup(id: string, restoreText: (text: string) => () => Promise<boolean>): Promise<void> {
    if (readOnly.value) {
      return
    }
    await followups.remove(id, (item) => {
      if (attachments.value.length) {
        throw new Error('输入框已有草稿，请先处理草稿，再编辑待发送消息')
      }
      const persist = restoreText(item.text)
      attachments.value = item.attachments
      return persist
    })
  }
  async function operateFollowup(action: 'remove' | 'deliver' | 'resume', id = ''): Promise<void> {
    if (readOnly.value) {
      return
    }
    try {
      if (action === 'remove') {
        await followups.remove(id)
      }
      else if (action === 'resume') {
        await resumeQueue()
      }
      else {
        await followups.deliver(id, working.value ? 'steer' : 'prompt')
      }
    }
    catch (cause) {
      queueError(cause)
      throw cause
    }
  }
  /** 只恢复明确拒绝的提交；草稿保存成功后才清除保留记录。 */
  async function restorePendingInput(restoreText: (text: string) => () => Promise<boolean>): Promise<void> {
    const value = pendingInput.value
    if (!value || pendingStatus.value !== 'rejected') {
      return
    }
    const persist = restoreText(value.text)
    attachments.value = [...value.attachments, ...attachments.value.filter(file => !value.attachments.some(original => original.id === file.id))]
    if (await persist()) {
      await clearPending()
    }
  }

  /** 先显示本地提交，再保存草稿和启动任务；pi 用户事件沿用占位 ID。 */
  async function send(requirement: string, images: ImageInput[] = [], saveDraft?: () => Promise<boolean>, original?: { text: string, attachments: PromptAttachment[] }, requestId?: string): Promise<void> {
    if (readOnly.value) {
      throw new InputNotSentError('此会话只能查看历史，请新建会话')
    }
    if (!modelState.value || modelState.value.error || modelChangeError.value) {
      throw new InputNotSentError(modelChangeError.value || modelState.value?.error || '请先选择模型')
    }
    if (images.length && !modelState.value.definition?.input.includes('image')) {
      throw new InputNotSentError('当前模型不支持图片，请移除图片附件或切换模型')
    }
    if (pendingInput.value) {
      throw new InputNotSentError('请先核对并处理保留的提交内容')
    }
    if (!runtimeConfirmed.value) {
      throw new InputNotSentError('会话状态尚未确认，请重新连接后再提交')
    }
    if (working.value) {
      await steer(requirement, images, saveDraft)
      return
    }
    if (busy.value) {
      throw new InputNotSentError('会话暂不能提交，请保留输入')
    }
    submitting.value = true
    queueReady.value = false
    stopping.value = false
    error.value = ''
    flush()
    const localId = requestId ?? crypto.randomUUID()
    const submission: PendingSubmission = { requestId: localId, modId: modId.value, sessionId: sessionId.value, text: original?.text ?? requirement, message: requirement, attachments: original?.attachments ?? [], images, timestamp: Date.now() }
    startedAt.value = Date.now()
    endedAt.value = undefined
    activity.value = '正在准备开发环境'
    lastActivityAt.value = Date.now()
    chat.value = { ...chat.value, pendingUserId: localId, messages: [...chat.value.messages, { id: localId, role: 'user', parts: [{ type: 'text', text: requirement }, ...images], final: false, timestamp: startedAt.value }] }
    try {
      if (saveDraft && !await saveDraft()) {
        throw new InputNotSentError('需求保存失败，请重试')
      }
      try {
        await pendingSubmission.save(JSON.parse(JSON.stringify(submission)))
      }
      catch (cause) {
        throw new InputNotSentError((cause as Error).message)
      }
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
  async function steer(message: string, images: ImageInput[], saveDraft?: () => Promise<boolean>, requestId?: string): Promise<void> {
    if (!canSteer.value || !task.value) {
      throw new InputNotSentError('任务暂不能接收引导，请保留输入')
    }
    steeringSubmitting.value = true
    const inputId = requestId ?? crypto.randomUUID()
    flush()
    chat.value = { ...chat.value, pendingInputs: [...(chat.value.pendingInputs ?? []), inputId], messages: [...chat.value.messages, { id: inputId, inputId, taskId: task.value.taskId, role: 'user', parts: [{ type: 'text', text: message }, ...images], final: false, timestamp: Date.now(), inputStatus: 'submitted' }] }
    try {
      if (saveDraft && !await saveDraft()) {
        throw new InputNotSentError('需求保存失败，请重试')
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
    queueReady.value = false
    void followups.pause().catch(queueError)
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
  const queueView = { items: readonly(followups.items), paused: readonly(followups.paused), busy: readonly(followups.busy), error: readonly(followups.error), editingId: readonly(followups.editingId) }
  return { updateAccess, release, addAttachments, removeAttachment, submitInput, editFollowup, operateFollowup, restorePendingInput, extensionRequest, respondExtensionRequest, followups: queueView, synchronizing: readonly(client.synchronizing), initialized: readonly(initialized), connectionError: readonly(connectionError), pendingInput: readonly(pendingInput), pendingStatus: readonly(pendingStatus), uncertain: readonly(uncertain), recover, clearPending, attachments: readonly(attachments), steeringInputs: readonly(steeringInputs), stopping: readonly(stopping), canSteer: readonly(canSteer), chat: readonly(chat), modId: readonly(modId), sessionId: readonly(sessionId), task: readonly(task), error: readonly(error), diagnostics: readonly(diagnostics), readOnly: readonly(readOnly), restartAvailable: readonly(restartAvailable), loadedSkills: readonly(loadedSkills), working: readonly(working), busy: readonly(busy), switching: readonly(switching), compacting: readonly(compacting), changingThinking: readonly(changingThinking), modelState: readonly(modelState), modelChangeError: readonly(modelChangeError), thinkingLevel: readonly(thinkingLevel), thinkingLevels: readonly(thinkingLevels), contextUsage: readonly(contextUsage), autoCompaction: readonly(autoCompaction), activity: readonly(activity), lastActivityAt: readonly(lastActivityAt), startedAt: readonly(startedAt), endedAt: readonly(endedAt), open, abort, restart, refreshModel, selectModel, setThinking, compact, close }
}
