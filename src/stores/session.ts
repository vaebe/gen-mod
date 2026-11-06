import { defineStore } from 'pinia'
import { computed, reactive, shallowRef } from 'vue'
import { createSessionRuntime } from '@/services/pi/sessionRuntime'

/** 页面只选择展示槽；后台通信和归并随应用 store 存续。 */
export const useSessionStore = defineStore('session', () => {
  const runtimes = reactive(new Map<string, ReturnType<typeof runtime>>())
  const selectedKey = shallowRef('')
  const fallback = runtime()
  let selection = 0
  const projectGenerations = new Map<string, number>()
  function runtime() {
    return reactive(createSessionRuntime())
  }
  function key(modId: string, sessionId: string): string {
    return `${modId}/${sessionId}`
  }
  const selected = computed(() => runtimes.get(selectedKey.value) ?? fallback)
  async function open(modId: string, sessionId?: string, readOnly = false): Promise<void> {
    const epoch = ++selection
    const projectGeneration = projectGenerations.get(modId) ?? 0
    const existing = sessionId ? runtimes.get(key(modId, sessionId)) : undefined
    if (existing && (existing.initialized)) {
      existing.readOnly = readOnly
      selectedKey.value = key(modId, sessionId!)
      await existing.recover()
      return
    }
    const value = existing ?? runtime()
    const pendingKey = key(modId, sessionId ?? crypto.randomUUID())
    runtimes.set(pendingKey, value)
    selectedKey.value = pendingKey
    try {
      await value.open(modId, sessionId, readOnly)
    }
    finally {
      if (value.sessionId && projectGeneration === (projectGenerations.get(modId) ?? 0)) {
        const actual = key(modId, value.sessionId)
        runtimes.delete(pendingKey)
        runtimes.set(actual, value)
        if (epoch === selection) {
          selectedKey.value = actual
        }
      }
    }
  }
  function stateFor(modId: string, sessionId: string) {
    return runtimes.get(key(modId, sessionId))
  }
  function isBusy(modId: string, sessionId: string): boolean {
    return stateFor(modId, sessionId)?.busy ?? false
  }
  function isRunning(modId: string, sessionId: string): boolean {
    const value = stateFor(modId, sessionId)
    return !!value && (value.working || value.compacting)
  }
  async function close(): Promise<void> {
    const value = selected.value
    const id = selectedKey.value
    await value.close()
    runtimes.delete(id)
    if (selectedKey.value === id) {
      selectedKey.value = ''
    }
  }
  function forgetProject(id: string): void {
    projectGenerations.set(id, (projectGenerations.get(id) ?? 0) + 1)
    ++selection
    for (const [runtimeKey, value] of runtimes) {
      if (runtimeKey.startsWith(`${id}/`)) {
        value.client.select('', '', 0)
        value.readOnly = true
        runtimes.delete(runtimeKey)
        if (selectedKey.value === runtimeKey) {
          selectedKey.value = ''
        }
      }
    }
  }
  const client = computed({ get: () => selected.value.client, set: (value) => {
    selected.value.client = value
  } })
  const chat = computed({ get: () => selected.value.chat, set: (value) => {
    selected.value.chat = value
  } })
  const modId = computed({ get: () => selected.value.modId, set: (value) => {
    selected.value.modId = value
  } })
  const sessionId = computed({ get: () => selected.value.sessionId, set: (value) => {
    selected.value.sessionId = value
  } })
  const task = computed({ get: () => selected.value.task, set: (value) => {
    selected.value.task = value
  } })
  const error = computed({ get: () => selected.value.error, set: (value) => {
    selected.value.error = value
  } })
  const diagnostics = computed({ get: () => selected.value.diagnostics, set: (value) => {
    selected.value.diagnostics = value
  } })
  const uiRequest = computed({ get: () => selected.value.uiRequest, set: (value) => {
    selected.value.uiRequest = value
  } })
  const changeRevision = computed({ get: () => selected.value.changeRevision, set: (value) => {
    selected.value.changeRevision = value
  } })
  const artifactRevision = computed({ get: () => selected.value.artifactRevision, set: (value) => {
    selected.value.artifactRevision = value
  } })
  const verificationRevision = computed(() => selected.value.verificationRevision)
  const memoryRevision = computed(() => selected.value.memoryRevision)
  const readOnly = computed({ get: () => selected.value.readOnly, set: (value) => {
    selected.value.readOnly = value
  } })
  const restartAvailable = computed({ get: () => selected.value.restartAvailable, set: (value) => {
    selected.value.restartAvailable = value
  } })
  const loadedSkills = computed({ get: () => selected.value.loadedSkills, set: (value) => {
    selected.value.loadedSkills = value
  } })
  const working = computed(() => selected.value.working)
  const busy = computed(() => selected.value.busy)
  const modelState = computed(() => selected.value.modelState)
  const modelChangeError = computed(() => selected.value.modelChangeError)
  const selectModel = (...args: Parameters<ReturnType<typeof createSessionRuntime>['selectModel']>) => selected.value.selectModel(...args)
  const refreshModel = () => selected.value.refreshModel()
  const switching = computed({ get: () => selected.value.switching, set: (value) => {
    selected.value.switching = value
  } })
  const compacting = computed({ get: () => selected.value.compacting, set: (value) => {
    selected.value.compacting = value
  } })
  const changingThinking = computed({ get: () => selected.value.changingThinking, set: (value) => {
    selected.value.changingThinking = value
  } })
  const thinkingLevel = computed({ get: () => selected.value.thinkingLevel, set: (value) => {
    selected.value.thinkingLevel = value
  } })
  const thinkingLevels = computed({ get: () => selected.value.thinkingLevels, set: (value) => {
    selected.value.thinkingLevels = value
  } })
  const contextUsage = computed({ get: () => selected.value.contextUsage, set: (value) => {
    selected.value.contextUsage = value
  } })
  const autoCompaction = computed({ get: () => selected.value.autoCompaction, set: (value) => {
    selected.value.autoCompaction = value
  } })
  const activity = computed({ get: () => selected.value.activity, set: (value) => {
    selected.value.activity = value
  } })
  const lastActivityAt = computed({ get: () => selected.value.lastActivityAt, set: (value) => {
    selected.value.lastActivityAt = value
  } })
  const startedAt = computed({ get: () => selected.value.startedAt, set: (value) => {
    selected.value.startedAt = value
  } })
  const endedAt = computed({ get: () => selected.value.endedAt, set: (value) => {
    selected.value.endedAt = value
  } })
  const send = (...args: Parameters<ReturnType<typeof createSessionRuntime>['send']>) => selected.value.send(...args)
  const abort = (...args: Parameters<ReturnType<typeof createSessionRuntime>['abort']>) => selected.value.abort(...args)
  const restart = (...args: Parameters<ReturnType<typeof createSessionRuntime>['restart']>) => selected.value.restart(...args)
  const flush = (...args: Parameters<ReturnType<typeof createSessionRuntime>['flush']>) => selected.value.flush(...args)
  const refreshStats = (...args: Parameters<ReturnType<typeof createSessionRuntime>['refreshStats']>) => selected.value.refreshStats(...args)
  const setThinking = (...args: Parameters<ReturnType<typeof createSessionRuntime>['setThinking']>) => selected.value.setThinking(...args)
  const compact = (...args: Parameters<ReturnType<typeof createSessionRuntime>['compact']>) => selected.value.compact(...args)
  const attachments = computed({ get: () => selected.value.attachments, set: (value) => {
    selected.value.attachments = value
  } })
  const canSteer = computed(() => selected.value.canSteer)
  const stopping = computed(() => selected.value.stopping)
  const steeringInputs = computed(() => selected.value.steeringInputs)
  const anyBusy = computed(() => [...runtimes.values()].some(value => value.busy))
  const connectionError = computed(() => selected.value.connectionError)
  const pendingInput = computed(() => selected.value.pendingInput)
  const pendingStatus = computed(() => selected.value.pendingStatus)
  const synchronizing = computed(() => selected.value.client.synchronizing)
  const uncertain = computed(() => selected.value.uncertain)
  const recover = (force = false) => selected.value.recover(force)
  const clearPending = () => selected.value.clearPending()
  return { connectionError, pendingInput, pendingStatus, synchronizing, uncertain, recover, clearPending, forgetProject, attachments, canSteer, stopping, steeringInputs, runtimes, selectedKey, stateFor, isBusy, isRunning, anyBusy, open, close, client, chat, modId, sessionId, task, error, diagnostics, uiRequest, changeRevision, artifactRevision, verificationRevision, memoryRevision, readOnly, restartAvailable, loadedSkills, working, busy, switching, compacting, changingThinking, modelState, modelChangeError, selectModel, refreshModel, thinkingLevel, thinkingLevels, contextUsage, autoCompaction, activity, lastActivityAt, startedAt, endedAt, send, abort, restart, flush, refreshStats, setThinking, compact }
})
