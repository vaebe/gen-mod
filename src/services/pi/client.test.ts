import type { PiEnvelope, RuntimeSnapshot } from './types'
import { expect, it, vi } from 'vitest'
import { nativeCall } from '@/services/native'
import { createPiClient } from './client'

vi.mock('@/services/native', () => ({ nativeCall: vi.fn() }))
vi.mock('@tauri-apps/api/core', () => ({ Channel: class { onmessage?: (event: PiEnvelope) => void } }))

it('ipc 先返回新代次时重置旧订阅游标，旧代次迟到事件不能污染新序号', async () => {
  let channel: { onmessage: (event: PiEnvelope) => void } | undefined
  const snapshot: RuntimeSnapshot = { modId: 'p', sessionId: 's', generation: 1, subscription: 3, sequence: 80, connected: true, running: false, active: false, stopping: false, compacting: false, task: null, messages: [], tools: [] }
  vi.mocked(nativeCall).mockImplementation(async (command, args) => {
    if (command === 'reserve_pi_subscription') {
      return { generation: 1, ticket: 3 }
    }
    channel = args?.onEvent as typeof channel
    return command === 'read_pi_runtime' ? snapshot : { modId: 'p', sessionId: 's', generation: 2 }
  })
  const client = createPiClient()
  client.select('p', 's', 1)
  await client.synchronize(true, () => undefined)
  await client.connect('p', 's')
  const event = (generation: number, sequence: number, subscription: number, type: string): PiEnvelope => ({ modId: 'p', sessionId: 's', generation, sequence, subscription, kind: 'record', record: { type } })
  channel!.onmessage(event(2, 1, 0, 'agent_start'))
  expect(client.active.value).toBe(true)
  channel!.onmessage(event(1, 900, 3, 'agent_start'))
  expect(client.lastSequence.value).toBe(1)
  channel!.onmessage(event(2, 2, 0, 'agent_settled'))
  expect(client.active.value).toBe(false)
})

it('健康状态快照先返回时保留在途正文，旧生命周期不能覆盖快照', async () => {
  let channel: { onmessage: (event: PiEnvelope) => void } | undefined
  let sequence = 0
  vi.mocked(nativeCall).mockImplementation(async (command, args) => {
    if (command === 'reserve_pi_subscription') {
      return { generation: 1, ticket: 1 }
    }
    if (!channel) {
      channel = args?.onEvent as typeof channel
    }
    return { modId: 'p', sessionId: 's', generation: 1, sequence, subscription: 1, connected: true, running: false, active: false, stopping: false, compacting: false, task: null, messages: [], tools: [] }
  })
  const client = createPiClient()
  client.select('p', 's', 1)
  await client.synchronize(true, () => undefined)
  sequence = 3
  await client.synchronize(false, () => undefined)
  const received: string[] = []
  client.subscribe(event => received.push(event.record.type))
  const event = (sequence: number, type: string): PiEnvelope => ({ modId: 'p', sessionId: 's', generation: 1, sequence, subscription: 1, kind: 'record', record: { type } })
  channel!.onmessage(event(1, 'message_end'))
  channel!.onmessage(event(2, 'agent_start'))
  channel!.onmessage(event(3, 'agent_settled'))
  expect(received).toEqual(['message_end'])
  expect(client.active.value).toBe(false)
  channel!.onmessage(event(4, 'agent_start'))
  expect(client.active.value).toBe(true)
})
