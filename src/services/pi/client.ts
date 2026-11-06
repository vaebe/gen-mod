import type { PiEnvelope, PiRecord, RuntimeSnapshot } from './types'
import type { ModelSelection, SessionModelState } from '@/contracts/config'
import type { ImageInput, ThinkingLevel } from '@/contracts/session'
import type { TaskContext } from '@/contracts/task'
import { Channel } from '@tauri-apps/api/core'
import { shallowRef } from 'vue'
import { nativeCall } from '@/services/native'

interface Pending { resolve: (record: PiRecord) => void, reject: (error: Error) => void, timer: ReturnType<typeof setTimeout> }

/** 单连接请求匹配器，命令超时不重发，也不认定 Agent 已停止。 */
export function createPiClient() {
  const connection = shallowRef<'disconnected' | 'connecting' | 'connected' | 'error'>('disconnected')
  const submissionState = shallowRef<'idle' | 'submitting' | 'running' | 'queued' | 'finished' | 'failed'>('idle')
  const active = shallowRef(false)
  const synchronizing = shallowRef(false)
  const connectionError = shallowRef('')
  const lastSequence = shallowRef(0)
  let subscription = 0
  let stateBoundary = 0
  let buffer: PiEnvelope[] | undefined
  let recovery: Promise<RuntimeSnapshot> | undefined
  const generation = shallowRef(0)
  const pending = new Map<string, Pending>()
  const listeners = new Set<(event: PiEnvelope) => void>()
  let settledSeen = false
  let minimumGeneration = 0
  let identity = { modId: '', sessionId: '' }

  function rejectAll(message: string): void {
    for (const request of pending.values()) {
      clearTimeout(request.timer)
      request.reject(new Error(message))
    }
    pending.clear()
  }

  function advanceGeneration(next: number): void {
    if (next > generation.value) {
      generation.value = next
      subscription = 0
      lastSequence.value = 0
      stateBoundary = 0
    }
  }
  function receive(event: PiEnvelope): void {
    if (event.modId !== identity.modId || (identity.sessionId && event.sessionId !== identity.sessionId)) {
      return
    }
    if (buffer) {
      buffer.push(event)
      return
    }
    if (event.generation < Math.max(generation.value, minimumGeneration)) {
      return
    }
    if (event.generation === generation.value && (event.subscription !== subscription || event.sequence <= lastSequence.value)) {
      return
    }
    if (event.generation > generation.value) {
      lastSequence.value = 0
      stateBoundary = 0
      subscription = event.subscription
    }
    lastSequence.value = event.sequence
    identity.sessionId = event.sessionId
    if (event.generation > generation.value) {
      generation.value = event.generation
    }
    const record = event.record
    const historicalState = event.sequence <= stateBoundary
    if (event.kind === 'exit' && !historicalState) {
      connection.value = 'disconnected'
      if (record.reason === 'unexpected') {
        connectionError.value = 'pi 进程意外退出，任务已中断'
      }
      active.value = false
      submissionState.value = 'finished'
      rejectAll('pi 进程已退出')
    }
    else if (event.kind === 'record') {
      if (record.type === 'agent_start' && !historicalState) {
        active.value = true
      }
      if (record.type === 'agent_settled' && !historicalState) {
        settledSeen = true
        active.value = false
        if (submissionState.value !== 'submitting') {
          submissionState.value = 'finished'
        }
      }
      if (record.type === 'response' && record.id) {
        const request = pending.get(record.id)
        if (request) {
          clearTimeout(request.timer)
          pending.delete(record.id)
          if (record.success) {
            request.resolve(record)
          }
          else { request.reject(new Error(record.error || 'RPC 命令失败')) }
        }
      }
    }
    // 状态快照可以先于旧通道消息抵达；保留旧正文/工具记录，只抑制已被快照取代的状态。
    if (historicalState && (event.kind === 'exit' || ['agent_start', 'agent_settled', 'compaction_start', 'compaction_end', 'task_started'].includes(record.type) || (record.type === 'response' && record.command === 'get_state'))) {
      return
    }
    for (const listener of listeners) {
      listener(event)
    }
  }

  function channel(): Channel<PiEnvelope> {
    const result = new Channel<PiEnvelope>()
    result.onmessage = receive
    return result
  }

  function select(modId: string, sessionId: string, savedGeneration: number): void {
    identity = { modId, sessionId }
    generation.value = savedGeneration
    lastSequence.value = 0
  }
  async function connect(modId: string, sessionId?: string): Promise<void> {
    identity = { modId, sessionId: sessionId ?? '' }
    rejectAll('正在建立新连接')
    minimumGeneration = generation.value + 1
    connection.value = 'connecting'
    try {
      const result = await nativeCall<{ modId: string, sessionId: string, generation: number }>('connect_pi', { modId, sessionId, onEvent: channel() })
      advanceGeneration(result.generation)
      identity = result
      connection.value = 'connected'
    }
    catch (error) {
      minimumGeneration = generation.value
      connection.value = 'error'
      throw error
    }
  }

  function request(type: string, fields: Record<string, unknown> = {}, timeoutMs = 30_000): Promise<PiRecord> {
    const id = crypto.randomUUID()
    return new Promise((resolve, reject) => {
      const timer = setTimeout(() => {
        pending.delete(id)
        reject(new Error('RPC 响应超时；任务可能仍在运行，请查询状态'))
      }, timeoutMs)
      pending.set(id, { resolve, reject, timer })
      nativeCall<void>('send_pi_command', { ...identity, generation: generation.value, command: { ...fields, type, id } }).catch((error: Error) => {
        const item = pending.get(id)
        if (!item) {
          return
        }
        clearTimeout(item.timer)
        pending.delete(id)
        reject(error)
      })
    })
  }

  async function selectModel(selection: ModelSelection): Promise<SessionModelState> {
    rejectAll('正在切换模型')
    minimumGeneration = generation.value + 1
    connection.value = 'connecting'
    try {
      const result = await nativeCall<{ modelState: SessionModelState, generation: number | null }>('select_session_model', { ...identity, selection, onEvent: channel() })
      if (result.generation === null) {
        minimumGeneration = generation.value
        connection.value = 'disconnected'
        return result.modelState
      }
      advanceGeneration(result.generation)
      connection.value = 'connected'
      const state = await request('get_state')
      const model = state.data?.model as { provider?: string, id?: string } | undefined
      if (model?.provider !== selection.providerId || model.id !== selection.modelId) {
        throw new Error('运行时未确认所选模型，请重新选择或重新连接')
      }
      const levels = await request('get_available_thinking_levels')
      if (!Array.isArray(levels.data?.levels) || typeof state.data?.thinkingLevel !== 'string') {
        throw new TypeError('运行时未返回模型思考强度，请重新连接')
      }
      return { ...result.modelState, thinkingLevel: state.data.thinkingLevel as ThinkingLevel, thinkingLevels: levels.data.levels as ThinkingLevel[] }
    }
    catch (cause) {
      minimumGeneration = generation.value
      connection.value = 'error'
      throw cause
    }
  }

  /** 原生创建并绑定任务后才提交 prompt，普通 RPC 不能绕开此入口。 */
  async function prompt(modId: string, sessionId: string, requirement: string, thinkingLevel: ThinkingLevel, images: ImageInput[] = [], requestId = crypto.randomUUID()): Promise<TaskContext> {
    rejectAll('正在开始新任务')
    minimumGeneration = generation.value + 1
    submissionState.value = 'submitting'
    settledSeen = false
    connection.value = 'connecting'
    const id = requestId
    let response: Promise<PiRecord> | undefined
    try {
      // 原生在返回前可能已发响应，先注册 pending。
      response = new Promise<PiRecord>((resolve, reject) => {
        const timer = setTimeout(() => {
          pending.delete(id)
          reject(new Error('需求提交响应超时，未自动重发'))
        }, 30_000)
        pending.set(id, { resolve, reject, timer })
      })
      // 捕获提前失败，避免等待原生启动期间产生未处理拒绝。
      void response.catch(() => undefined)
      const context = await nativeCall<TaskContext>('start_mod_task', { modId, sessionId, requirement, thinkingLevel, images, requestId: id, onEvent: channel() })
      advanceGeneration(context.generation)
      connection.value = 'connected'
      const record = await response
      const disposition = record.data?.disposition
      if (disposition === 'handled') {
        submissionState.value = 'finished'
      }
      else if (settledSeen) {
        submissionState.value = 'finished'
      }
      else if (disposition === 'queued') {
        submissionState.value = 'queued'
      }
      else if (disposition === 'started') {
        submissionState.value = 'running'
        active.value = true
      }
      else { throw new Error('pi 返回未知 prompt disposition') }
      return context
    }
    catch (error) {
      minimumGeneration = generation.value
      const item = pending.get(id)
      if (item) {
        clearTimeout(item.timer)
        pending.delete(id)
        item.reject(error as Error)
      }
      submissionState.value = 'failed'
      if (connection.value === 'connecting') {
        connection.value = 'error'
      }
      throw error
    }
  }

  /** 引导使用独立受控入口，保持原 generation 与 taskId。 */
  async function steer(taskId: string, inputId: string, message: string, images: ImageInput[]): Promise<PiRecord> {
    const response = new Promise<PiRecord>((resolve, reject) => {
      const timer = setTimeout(() => {
        pending.delete(inputId)
        reject(new Error('引导响应未确认；未自动重发，请检查历史'))
      }, 30_000)
      pending.set(inputId, { resolve, reject, timer })
    })
    void response.catch(() => undefined)
    try {
      await nativeCall('steer_mod_task', { ...identity, generation: generation.value, taskId, inputId, message, images })
      return await response
    }
    catch (cause) {
      const item = pending.get(inputId)
      if (item) {
        clearTimeout(item.timer)
        pending.delete(inputId)
        item.reject(cause as Error)
      }
      throw cause
    }
  }
  async function disconnect(): Promise<void> {
    if (identity.modId && identity.sessionId && connection.value !== 'disconnected') {
      await nativeCall('disconnect_pi', { ...identity, generation: generation.value })
    }
    rejectAll('连接已关闭')
    connection.value = 'disconnected'
    active.value = false
    submissionState.value = 'idle'
  }
  function applySnapshot(value: RuntimeSnapshot, full: boolean): void {
    advanceGeneration(value.generation)
    minimumGeneration = value.generation
    if (value.connected || full) {
      subscription = value.subscription
    }
    stateBoundary = value.sequence
    if (full) {
      lastSequence.value = value.sequence
    }
    active.value = value.active
    submissionState.value = value.running ? 'running' : 'finished'
    connection.value = value.connected ? 'connected' : 'disconnected'
  }
  /** 同一 runtime 的恢复串行化；快照应用之后再交付边界之后的事件。 */
  async function synchronize(attach: boolean, restore: (value: RuntimeSnapshot) => void): Promise<RuntimeSnapshot> {
    if (recovery) {
      return recovery
    }
    synchronizing.value = true
    const operation = (async () => {
      if (attach) {
        buffer = []
      }
      try {
        const reservation = attach ? await nativeCall<{ generation: number, ticket: number }>('reserve_pi_subscription', identity) : undefined
        const value = await nativeCall<RuntimeSnapshot>('read_pi_runtime', { ...identity, attachGeneration: reservation?.generation, ticket: reservation?.ticket, onEvent: channel() })
        if (!attach && value.connected && (value.generation !== generation.value || value.subscription !== subscription)) {
          connection.value = 'error'
          throw new Error('会话订阅已变化，需要重新附着')
        }
        // 健康订阅查询期间已到达的事件优先于旧快照。
        if (attach || !value.connected || value.generation > generation.value || value.sequence >= lastSequence.value) {
          applySnapshot(value, attach)
          restore(value)
        }
        connectionError.value = ''
        return value
      }
      catch (cause) {
        connectionError.value = (cause as Error).message
        throw cause
      }
      finally {
        const events = buffer
        buffer = undefined
        if (events) {
          for (const event of events) {
            receive(event)
          }
        }
      }
    })()
    recovery = operation
    try {
      return await operation
    }
    finally {
      if (recovery === operation) {
        recovery = undefined
        synchronizing.value = false
      }
    }
  }
  function subscribe(listener: (event: PiEnvelope) => void): () => void {
    listeners.add(listener)
    return () => {
      listeners.delete(listener)
    }
  }
  return { connection, connectionError, synchronizing, lastSequence, synchronize, submissionState, active, generation, select, selectModel, connect, prompt, steer, request, disconnect, subscribe }
}
