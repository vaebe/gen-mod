import type { ModProject } from '@/contracts/mod'
import type { PromptAttachment, SessionIdentity, SessionSummary } from '@/contracts/session'
import type { ExtensionRequest, ExtensionResponse } from '@/services/pi/types'
import { defineStore } from 'pinia'
import { computed, reactive, readonly, shallowRef } from 'vue'
import { coordinateProjectEvent } from '@/application/projectRefresh'
import { useDeliveriesStore } from '@/features/delivery/useDeliveries'
import { createSessionRuntime } from '@/features/sessions/sessionRuntime'

/** 页面只选择展示槽；后台通信和归并随应用 store 存续。 */
export const useSessionStore = defineStore('session', () => {
  const runtimes = reactive(new Map<string, ReturnType<typeof runtime>>())
  const selectedKey = shallowRef('')
  const fallback = runtime()
  let selection = 0
  const projectGenerations = new Map<string, number>()
  function runtime() {
    return reactive(createSessionRuntime(coordinateProjectEvent))
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
      existing.updateAccess(readOnly)
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
  const selectionId = computed(() => selectedKey.value)
  const initialized = computed(() => selected.value.initialized)
  function isSelected(target: SessionIdentity): boolean {
    return selected.value.modId === target.modId && selected.value.sessionId === target.sessionId
  }
  function summaryFor(modId: string, sessionId: string): SessionSummary | undefined {
    const value = stateFor(modId, sessionId)
    return value ? Object.freeze({ initialized: value.initialized, busy: value.busy, running: value.working || value.compacting, activity: value.activity }) : undefined
  }
  function isProjectBusy(modId: string): boolean {
    return [...runtimes.entries()].some(([id, value]) => id.startsWith(`${modId}/`) && value.busy)
  }
  function syncProjectAccess(project: ModProject): void {
    for (const value of runtimes.values()) {
      if (value.modId === project.modId) {
        const metadata = project.sessions.find(item => item.sessionId === value.sessionId)
        if (metadata) {
          value.updateAccess(project.archived || metadata.readOnly)
        }
      }
    }
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
    useDeliveriesStore().forget(id)
    projectGenerations.set(id, (projectGenerations.get(id) ?? 0) + 1)
    ++selection
    for (const [runtimeKey, value] of runtimes) {
      if (runtimeKey.startsWith(`${id}/`)) {
        value.release()
        runtimes.delete(runtimeKey)
        if (selectedKey.value === runtimeKey) {
          selectedKey.value = ''
        }
      }
    }
  }
  const extensionRequest = computed(() => selected.value.extensionRequest)
  async function respondExtensionRequest(request: Readonly<ExtensionRequest>, response: ExtensionResponse): Promise<void> {
    const owner = runtimes.get(key(request.modId, request.sessionId))
    await owner?.respondExtensionRequest(request, response)
  }
  function inputOwner(target: SessionIdentity) {
    const owner = stateFor(target.modId, target.sessionId)
    if (!owner) {
      throw new Error('会话已关闭，请重新打开')
    }
    return owner
  }
  const addAttachments = (files: PromptAttachment[]) => selected.value.addAttachments(files)
  const removeAttachment = (id: string) => selected.value.removeAttachment(id)
  const submitInput = (target: SessionIdentity, ...args: Parameters<ReturnType<typeof createSessionRuntime>['submitInput']>) => inputOwner(target).submitInput(...args)
  const editFollowup = (target: SessionIdentity, ...args: Parameters<ReturnType<typeof createSessionRuntime>['editFollowup']>) => inputOwner(target).editFollowup(...args)
  const operateFollowup = (target: SessionIdentity, ...args: Parameters<ReturnType<typeof createSessionRuntime>['operateFollowup']>) => inputOwner(target).operateFollowup(...args)
  const restorePendingInput = (target: SessionIdentity, ...args: Parameters<ReturnType<typeof createSessionRuntime>['restorePendingInput']>) => inputOwner(target).restorePendingInput(...args)
  const chat = computed(() => selected.value.chat)
  const modId = computed(() => selected.value.modId)
  const sessionId = computed(() => selected.value.sessionId)
  const task = computed(() => selected.value.task)
  const error = computed(() => selected.value.error)
  const diagnostics = computed(() => selected.value.diagnostics)
  const readOnly = computed(() => selected.value.readOnly)
  const restartAvailable = computed(() => selected.value.restartAvailable)
  const loadedSkills = computed(() => selected.value.loadedSkills)
  const working = computed(() => selected.value.working)
  const busy = computed(() => selected.value.busy)
  const modelState = computed(() => selected.value.modelState)
  const modelChangeError = computed(() => selected.value.modelChangeError)
  const selectModel = (...args: Parameters<ReturnType<typeof createSessionRuntime>['selectModel']>) => selected.value.selectModel(...args)
  const refreshModel = () => selected.value.refreshModel()
  const switching = computed(() => selected.value.switching)
  const compacting = computed(() => selected.value.compacting)
  const changingThinking = computed(() => selected.value.changingThinking)
  const thinkingLevel = computed(() => selected.value.thinkingLevel)
  const thinkingLevels = computed(() => selected.value.thinkingLevels)
  const contextUsage = computed(() => selected.value.contextUsage)
  const autoCompaction = computed(() => selected.value.autoCompaction)
  const activity = computed(() => selected.value.activity)
  const lastActivityAt = computed(() => selected.value.lastActivityAt)
  const startedAt = computed(() => selected.value.startedAt)
  const endedAt = computed(() => selected.value.endedAt)
  const abort = (...args: Parameters<ReturnType<typeof createSessionRuntime>['abort']>) => selected.value.abort(...args)
  const restart = (...args: Parameters<ReturnType<typeof createSessionRuntime>['restart']>) => selected.value.restart(...args)
  const setThinking = (...args: Parameters<ReturnType<typeof createSessionRuntime>['setThinking']>) => selected.value.setThinking(...args)
  const compact = (...args: Parameters<ReturnType<typeof createSessionRuntime>['compact']>) => selected.value.compact(...args)
  const attachments = computed(() => selected.value.attachments)
  const canSteer = computed(() => selected.value.canSteer)
  const followups = computed(() => readonly(selected.value.followups))
  const stopping = computed(() => selected.value.stopping)
  const steeringInputs = computed(() => selected.value.steeringInputs)
  const anyBusy = computed(() => [...runtimes.values()].some(value => value.busy))
  const connectionError = computed(() => selected.value.connectionError)
  const pendingInput = computed(() => selected.value.pendingInput)
  const pendingStatus = computed(() => selected.value.pendingStatus)
  const synchronizing = computed(() => selected.value.synchronizing)
  const uncertain = computed(() => selected.value.uncertain)
  const recover = (force = false) => selected.value.recover(force)
  const clearPending = () => selected.value.clearPending()
  return { selectionId, initialized, isSelected, summaryFor, isProjectBusy, syncProjectAccess, addAttachments, removeAttachment, submitInput, editFollowup, operateFollowup, restorePendingInput, extensionRequest, respondExtensionRequest, followups, connectionError, pendingInput, pendingStatus, synchronizing, uncertain, recover, clearPending, forgetProject, attachments, canSteer, stopping, steeringInputs, isBusy, isRunning, anyBusy, open, close, chat, modId, sessionId, task, error, diagnostics, readOnly, restartAvailable, loadedSkills, working, busy, switching, compacting, changingThinking, modelState, modelChangeError, selectModel, refreshModel, thinkingLevel, thinkingLevels, contextUsage, autoCompaction, activity, lastActivityAt, startedAt, endedAt, abort, restart, setThinking, compact }
})
