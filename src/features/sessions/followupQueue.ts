import type { ImageInput, PromptAttachment } from '@/contracts/session'
import { shallowRef } from 'vue'
import { InputNotSentError } from '@/services/pi/types'
import { followupStorage } from './pendingSubmission'

export interface FollowupInput {
  id: string
  text: string
  message: string
  attachments: PromptAttachment[]
  images: ImageInput[]
  status: 'pending' | 'sending' | 'accepted' | 'unknown' | 'failed' | 'interrupted' | 'handled'
  mode?: 'steer' | 'prompt'
  error?: string
}
export interface FollowupState { items: FollowupInput[], paused: boolean, editingId?: string }

// 队列由会话持有；先持久化投递意图，再调用运行时，崩溃后不自动重发。
export function createFollowupQueue(options: {
  identity: () => { modId: string, sessionId: string }
  allowed: (mode: 'steer' | 'prompt') => boolean
  validate: (item: FollowupInput) => void
  dispatch: (item: FollowupInput, mode: 'steer' | 'prompt') => Promise<void>
  status: (item: FollowupInput) => Promise<FollowupInput['status'] | 'confirmed'>
  removed?: (id: string) => void
}) {
  const items = shallowRef<FollowupInput[]>([])
  const paused = shallowRef(true)
  const busy = shallowRef(false)
  const error = shallowRef('')
  const editingId = shallowRef<string>()
  let pauseRevision = 0
  let serial = Promise.resolve()
  function exclusive<T>(action: () => Promise<T>): Promise<T> {
    const operation = serial.then(async () => {
      busy.value = true
      try {
        return await action()
      }
      catch (cause) {
        error.value = (cause as Error).message
        throw cause
      }
      finally { busy.value = false }
    })
    serial = operation.then(() => undefined, () => undefined)
    return operation
  }
  async function save(next: FollowupInput[], pause = paused.value): Promise<void> {
    const identity = options.identity()
    const revision = pauseRevision
    const nextEditingId = next.some(item => item.id === editingId.value) ? editingId.value : undefined
    await followupStorage.save(identity.modId, identity.sessionId, { items: next, paused: pause, editingId: nextEditingId })
    const removed = items.value.filter(item => !next.some(value => value.id === item.id))
    items.value = next
    editingId.value = nextEditingId
    paused.value = pause || revision !== pauseRevision
    error.value = ''
    for (const item of removed) {
      options.removed?.(item.id)
    }
  }
  function load(): Promise<void> {
    return exclusive(async () => {
      const identity = options.identity()
      const saved = await followupStorage.read(identity.modId, identity.sessionId)
      items.value = (saved?.items ?? []).map(item => item.status === 'sending' ? { ...item, status: 'unknown' } : item)
      editingId.value = items.value.some(item => item.id === saved?.editingId) ? saved?.editingId : undefined
      // 重开应用后保留内容，等待明确继续；不根据旧终态自动启动任务。
      paused.value = true
    })
  }
  function add(value: Omit<FollowupInput, 'id' | 'status'>): Promise<void> {
    return exclusive(() => save([...items.value.filter(item => item.id !== editingId.value), { ...value, id: crypto.randomUUID(), status: 'pending' }], items.value.length ? paused.value : false))
  }
  function remove(id: string, restore?: (item: FollowupInput) => () => Promise<boolean>): Promise<void> {
    return exclusive(async () => {
      const item = items.value.find(item => item.id === id)
      if (!item || !['pending', 'failed', 'interrupted'].includes(item.status)) {
        return
      }
      try {
        if (restore) {
          // 同步恢复输入后建立来源关联，再持久化来源和草稿。
          const persistDraft = restore(item)
          editingId.value = id
          await save(items.value)
          if (!await persistDraft()) {
            await save(items.value, true)
            return
          }
        }
        await save(items.value.filter(item => item.id !== id))
      }
      catch (cause) {
        if (restore) {
          paused.value = true
        }
        throw cause
      }
    })
  }
  function pause(): Promise<void> {
    // 同步阻止已经排进微任务的自动投递。
    pauseRevision += 1
    paused.value = true
    return exclusive(() => save(items.value, true))
  }
  function deliver(id: string, mode: 'steer' | 'prompt', automatic = false): Promise<void> {
    return exclusive(async () => {
      const item = items.value.find(item => item.id === id)
      if (!item || item.status !== 'pending' || (automatic && paused.value)) {
        return
      }
      if (item.id === editingId.value) {
        throw new Error('请先提交输入框中正在编辑的消息，或撤回原队列消息')
      }
      if (!options.allowed(mode)) {
        throw new Error(mode === 'steer' ? '当前任务暂不能接收引导，消息已保留' : '会话暂不能发送，消息已保留')
      }
      options.validate(item)
      await save(items.value.map(value => value.id === id ? { ...value, mode, status: 'sending', error: undefined } : value))
      // 持久化期间任务可能结束或被停止；尚未投递时可以安全恢复为待发送。
      if (!options.allowed(mode) || (automatic && paused.value)) {
        // 未调用 dispatch，回滚保存失败时也保留内存中的确定未发送状态。
        items.value = items.value.map(value => value.id === id ? { ...value, status: 'pending', mode: undefined } : value)
        paused.value = true
        await save(items.value, true)
        return
      }
      try {
        await options.dispatch(item, mode)
        const status = await options.status({ ...item, mode })
        await save(status === 'confirmed' ? items.value.filter(value => value.id !== id) : items.value.map(value => value.id === id ? { ...value, status: status === 'pending' ? 'accepted' : status } : value), paused.value || status === 'failed' || status === 'interrupted')
      }
      catch (cause) {
        if (cause instanceof InputNotSentError) {
          items.value = items.value.map(value => value.id === id ? { ...value, status: 'failed', error: cause.message } : value)
          paused.value = true
          await save(items.value, true)
          return
        }
        // 投递后发生任何错误均先核对，不把超时误判成未发送。
        let status: FollowupInput['status'] | 'confirmed' = 'unknown'
        try {
          status = await options.status({ ...item, mode })
        }
        catch { /* 保留未确认状态，等待重新连接。 */ }
        await save(status === 'confirmed' ? items.value.filter(value => value.id !== id) : items.value.map(value => value.id === id ? { ...value, status: status === 'pending' ? 'unknown' : status, error: (cause as Error).message } : value), true)
      }
    })
  }
  function reconcile(): Promise<void> {
    return exclusive(async () => {
      const next: FollowupInput[] = []
      let changed = false
      for (const item of items.value) {
        if (['pending', 'failed'].includes(item.status)) {
          next.push(item)
          continue
        }
        const status = await options.status(item)
        changed ||= status !== item.status
        if (status !== 'confirmed') {
          next.push({ ...item, status })
        }
      }
      if (changed) {
        await save(next, paused.value || next.some(item => ['failed', 'interrupted'].includes(item.status)))
      }
    })
  }
  async function resume(): Promise<void> {
    if (editingId.value) {
      throw new Error('请先提交输入框中正在编辑的消息，或撤回原队列消息')
    }
    await exclusive(() => save(items.value, false))
  }
  return { items, paused, busy, error, editingId, load, add, remove, pause, deliver, reconcile, resume }
}
