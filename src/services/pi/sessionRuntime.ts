import type { ContextUsage, ImageInput, PromptAttachment, SteeringInput, ThinkingLevel } from '@/contracts/session'
import type { TaskContext } from '@/contracts/task'
import type { PiEnvelope, PiRecord } from '@/services/pi/types'
import { computed, shallowRef } from 'vue'
import { modService, nativeCall } from '@/services/native'

import { createPiClient } from '@/services/pi/client'
import { emptyChat, reduceRecord, restoreChat } from '@/services/pi/reducer'
import { useProjectsStore } from '@/stores/projects'

/** 页面卸载不结束任务；通信、事件归并与活动状态归此单一 store。 */
export function createSessionRuntime() {
  const client = createPiClient()
  const initialized = shallowRef(false)
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
  const submitting = shallowRef(false)
  const switching = shallowRef(false)
  const compacting = shallowRef(false)
  const manualCompaction = shallowRef(false)
  const changingThinking = shallowRef(false)
  const thinkingLevel = shallowRef<ThinkingLevel>('medium')
  const thinkingLevels = shallowRef<ThinkingLevel[]>([])
  const contextUsage = shallowRef<ContextUsage>()
  const autoCompaction = shallowRef(true)
  const activity = shallowRef('正在准备…')
  const lastActivityAt = shallowRef(0)
  const startedAt = shallowRef<number>()
  const endedAt = shallowRef<number>()
  const readOnly = shallowRef(false)
  const canSteer = computed(() => client.active.value && !submitting.value && !compacting.value && !stopping.value && !steeringSubmitting.value && !readOnly.value)
  const restartAvailable = shallowRef(false)
  const loadedSkills = shallowRef<string[]>([])
  let abortTimer: ReturnType<typeof setTimeout> | undefined
  const working = computed(() => submitting.value || client.active.value || client.submissionState.value === 'running' || client.submissionState.value === 'queued')
  const busy = computed(() => working.value || compacting.value || manualCompaction.value || switching.value || changingThinking.value)
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

  async function open(projectId: string, id?: string, historyOnly = false): Promise<void> {
    switching.value = true
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
      startedAt.value = undefined
      endedAt.value = undefined
      const targetId = id ?? await nativeCall<string>('create_mod_session', { modId: projectId })
      sessionId.value = targetId
      const [history, metadata] = await Promise.all([
        nativeCall<{ messages: unknown[] }>('read_session_history', { modId: projectId, sessionId: targetId }),
        nativeCall<{ thinkingLevel: ThinkingLevel, contextUsage?: ContextUsage, generation: number }>('read_session_metadata', { modId: projectId, sessionId: targetId }),
      ])
      chat.value = restoreChat(history.messages)
      contextUsage.value = metadata.contextUsage ? { ...metadata.contextUsage, historical: true } : undefined
      thinkingLevel.value = metadata.thinkingLevel
      thinkingLevels.value = ['low', 'medium', 'high']
      client.select(projectId, targetId, metadata.generation)
      await refreshInputs()
      initialized.value = true
    }
    finally { switching.value = false }
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
      else {
        await nativeCall('save_session_thinking', { modId: modId.value, sessionId: sessionId.value, level })
        thinkingLevel.value = level
      }
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
  async function send(requirement: string, images: ImageInput[] = [], saveDraft?: () => Promise<boolean>): Promise<void> {
    if (readOnly.value) {
      throw new Error('此会话只能查看历史，请新建会话')
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
    startedAt.value = Date.now()
    endedAt.value = undefined
    activity.value = '正在准备开发环境'
    lastActivityAt.value = Date.now()
    chat.value = { ...chat.value, pendingUserId: localId, messages: [...chat.value.messages, { id: localId, role: 'user', parts: [{ type: 'text', text: requirement }, ...images], final: false, timestamp: startedAt.value }] }
    try {
      if (saveDraft && !await saveDraft()) {
        throw new Error('需求保存失败，请重试')
      }
      task.value = await client.prompt(modId.value, sessionId.value, requirement, thinkingLevel.value, images)
      void refreshStats()
    }
    catch (cause) {
      flush()
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
  return { initialized, attachments, steeringInputs, stopping, canSteer, steeringSubmitting, client, chat, modId, sessionId, task, error, diagnostics, uiRequest, changeRevision, artifactRevision, readOnly, restartAvailable, loadedSkills, working, busy, switching, compacting, changingThinking, thinkingLevel, thinkingLevels, contextUsage, autoCompaction, activity, lastActivityAt, startedAt, endedAt, open, send, abort, restart, flush, refreshStats, setThinking, compact, close }
}
