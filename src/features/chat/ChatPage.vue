<script setup lang="ts">
import type { ModProject } from '@/contracts/mod'
import type { TaskRecord } from '@/contracts/task'
import type { VerificationRecord } from '@/features/verification/types'
import { RotateCw, Unplug } from '@lucide/vue'
import { computed, defineAsyncComponent, onBeforeUnmount, shallowRef, watch } from 'vue'
import { onBeforeRouteLeave, onBeforeRouteUpdate, useRoute, useRouter } from 'vue-router'
import { Alert, AlertDescription, AlertTitle } from '@/components/ui/alert'
import { Button } from '@/components/ui/button'
import { Collapsible, CollapsibleContent, CollapsibleTrigger } from '@/components/ui/collapsible'
import { Empty, EmptyDescription, EmptyHeader, EmptyTitle } from '@/components/ui/empty'
import { Spinner } from '@/components/ui/spinner'
import { useDeliveriesStore } from '@/features/delivery/useDeliveries'
import { modService } from '@/features/projects/service'
import { sessionService } from '@/features/sessions/service'
import { useSessionStore } from '@/features/sessions/useSessionStore'
import { useConfiguration } from '@/features/settings/useConfiguration'
import { useVerification } from '@/features/verification/useVerification'
import { referencePrompt, removeSourceReference, sourceLeaveGuards, sourceReferences } from '@/features/workspace/sourceWorkspace'
import { sourceRevision } from '@/features/workspace/useTaskChanges'
import { useProjectsStore } from '@/stores/projects'
import { readAttachments } from './attachments'
import ChatComposer from './ChatComposer.vue'
import MessageTimeline from './MessageTimeline.vue'
import { useDraft } from './useDraft'

const VerificationHistory = defineAsyncComponent(() => import('@/features/verification/VerificationHistory.vue'))
const FollowupQueue = defineAsyncComponent(() => import('./FollowupQueue.vue'))
const route = useRoute()
const router = useRouter()
const project = shallowRef<ModProject>()
const error = shallowRef('')
const opening = shallowRef(false)
const loadingProject = shallowRef(true)
const attachmentError = shallowRef('')
const readingAttachments = shallowRef(false)
const sending = shallowRef(false)
const editingFollowup = shallowRef(false)
let followupEdit: Promise<void> | undefined
let attachmentGeneration = 0
const tasks = shallowRef<TaskRecord[]>([])
const draft = useDraft()
const { requirement, error: saveError } = draft
const configuration = useConfiguration()
const { status } = configuration
const session = useSessionStore()
const deliveries = useDeliveriesStore()
const timelineMessages = computed(() => session.chat.messages.filter(message => message.final || (!message.inputId && !session.followups.items.some(item => item.id === message.id))))
const pendingSteering = computed(() => session.steeringInputs.filter(input => input.status !== 'confirmed' && !session.followups.items.some(item => item.id === input.inputId)))
async function editFollowup(id: string): Promise<void> {
  const target = { modId: session.modId, sessionId: session.sessionId }
  if (session.readOnly || sending.value || editingFollowup.value || readingAttachments.value) {
    return
  }
  editingFollowup.value = true
  try {
    followupEdit = session.editFollowup(target, id, (text) => {
      if (!session.isSelected(target) || requirement.value) {
        throw new Error('输入框已有草稿，请先处理草稿，再编辑待发送消息')
      }
      requirement.value = text
      return () => draft.flush()
    })
    await followupEdit
  }
  catch { /* 错误由原会话队列展示。 */ }
  finally {
    followupEdit = undefined
    editingFollowup.value = false
  }
}
async function operateFollowup(action: 'remove' | 'deliver' | 'resume', id = ''): Promise<void> {
  const target = { modId: session.modId, sessionId: session.sessionId }
  if (session.readOnly) {
    return
  }
  try {
    if (action !== 'remove') {
      const guard = sourceLeaveGuards.get(target.modId)
      if (guard && !await guard()) {
        return
      }
      if (!session.isSelected(target)) {
        return
      }
    }
    await session.operateFollowup(target, action, id)
  }
  catch (cause) {
    if (session.isSelected(target)) {
      error.value = (cause as Error).message
    }
  }
}
const verification = useVerification(() => project.value?.modId)
const pendingFeedback = computed(() => verification.data.value.drafts.filter(d => d.status === 'pending' && d.payload.sourceRefs.some(ref => ref.sessionId === session.sessionId)))
async function repair(record: VerificationRecord): Promise<void> {
  const target = project.value?.modId
  if (!target || record.modId !== target) {
    return
  }
  const text = `请修复交付版本 ${record.versionId}，玩家确认记录 ${record.recordId}：\n${record.payload.observations}\n结果：${record.payload.result}。先核对当前源码与该版本的差异。`
  if (session.readOnly) {
    await openSession(undefined, false)
  }
  if (session.readOnly || project.value?.modId !== target || session.modId !== target) {
    return
  }
  requirement.value = requirement.value.trim() ? `${requirement.value}\n\n${text}` : text
}
const attachments = computed(() => session.attachments)
const projects = useProjectsStore()
const connectionIssue = computed(() => session.connectionError)
const operationIssue = computed(() => error.value || session.error || session.modelState?.error || configuration.error.value)
const displayedIssue = computed(() => connectionIssue.value || operationIssue.value)
const modelConfigured = computed(() => {
  const selected = session.modelState
  if (!selected || selected.error || session.modelChangeError) {
    return false
  }
  return !!status.value?.providers.some(provider => provider.id === selected.selection.providerId && provider.keyConfigured && provider.models.some(model => model.id === selected.selection.modelId))
})
watch(() => [status.value, session.selectionId], () => {
  void session.refreshModel()
})
const runtimeClosed = computed(() => connectionIssue.value === 'pi 进程已退出')
const reconnectId = computed(() => session.sessionId || String(route.query.session ?? ''))
watch(() => `${session.modId}/${session.sessionId}`, () => {
  attachmentGeneration += 1
  attachmentError.value = ''
  readingAttachments.value = false
})
/** 切换会话期间完成的文件读取不得附加到另一个会话。 */
async function attachFiles(files: File[]): Promise<void> {
  if (readingAttachments.value || editingFollowup.value) {
    return
  }
  const generation = attachmentGeneration
  readingAttachments.value = true
  attachmentError.value = ''
  try {
    const added = await readAttachments(files, attachments.value)
    if (generation === attachmentGeneration) {
      session.addAttachments(added)
    }
  }
  catch (cause) {
    if (generation === attachmentGeneration) {
      attachmentError.value = (cause as Error).message
    }
  }
  finally {
    if (generation === attachmentGeneration) {
      readingAttachments.value = false
    }
  }
}
let loadGeneration = 0
let saveTimer: ReturnType<typeof setTimeout> | undefined
watch(requirement, () => {
  if (saveTimer) {
    clearTimeout(saveTimer)
  }
  saveTimer = setTimeout(() => {
    void draft.flush()
  }, 600)
})

async function openSession(id?: string, readOnly = false): Promise<void> {
  if (!project.value || !await draft.flush()) {
    return
  }
  const epoch = loadGeneration
  opening.value = true
  error.value = ''
  try {
    const modId = project.value.modId
    await session.open(modId, id, project.value.archived || readOnly)
    if (epoch !== loadGeneration || project.value?.modId !== modId || (id && session.sessionId !== id)) {
      return
    }
    const metadata = await sessionService.metadata({ modId, sessionId: session.sessionId })
    if (epoch !== loadGeneration) {
      return
    }
    draft.load(modId, metadata.draft, session.sessionId)
    const records = await sessionService.tasks({ modId })
    if (epoch !== loadGeneration) {
      return
    }
    tasks.value = records
    project.value = await modService.get(project.value.modId)
    projects.upsert(project.value)
    session.syncProjectAccess(project.value)
  }
  catch (cause) { error.value = (cause as Error).message }
  finally {
    if (epoch === loadGeneration) {
      opening.value = false
    }
  }
}

/** 重连只打开已选中的会话，不允许空 ID 触发新建。 */
async function reconnect(): Promise<void> {
  if (opening.value || session.synchronizing || !reconnectId.value) {
    return
  }
  opening.value = true
  try {
    await session.recover(true)
  }
  catch (cause) { error.value = (cause as Error).message }
  finally { opening.value = false }
}

watch(() => [String(route.params.modId), String(route.query.session ?? ''), String(route.query.new ?? '')], async ([id, requested, fresh]) => {
  if (!id) {
    return
  }
  const generation = ++loadGeneration
  loadingProject.value = true
  const changedProject = project.value?.modId !== id
  if (changedProject) {
    project.value = undefined
    tasks.value = []
  }
  error.value = ''
  try {
    const [value] = await Promise.all([modService.get(id), configuration.refresh()])
    if (generation !== loadGeneration) {
      return
    }
    project.value = value

    const previous = fresh ? undefined : requested ? value.sessions.find(item => item.sessionId === requested) : value.sessions[value.sessions.length - 1]
    if (requested && !previous && !fresh) {
      throw new Error('该会话已删除或不属于当前项目')
    }
    if (!fresh && previous?.sessionId === session.sessionId && session.modId === id && session.summaryFor(id, previous.sessionId)?.initialized) {
      session.syncProjectAccess(value)
      const metadata = await sessionService.metadata({ modId: id, sessionId: previous.sessionId })
      if (generation !== loadGeneration) {
        return
      }
      draft.load(id, metadata.draft, previous.sessionId)
      const records = await sessionService.tasks({ modId: id })
      if (generation !== loadGeneration) {
        return
      }
      tasks.value = records
      return
    }
    await openSession(previous?.sessionId, previous?.readOnly)
    if (generation === loadGeneration && session.sessionId && !error.value) {
      await router.replace({ path: route.path, query: { session: session.sessionId, ...(route.query.repair ? { repair: route.query.repair } : {}), ...(route.query.task ? { task: route.query.task } : {}) } })
    }
  }
  catch (cause) {
    if (generation === loadGeneration) {
      error.value = (cause as Error).message
    }
  }
  finally {
    if (generation === loadGeneration) {
      loadingProject.value = false
    }
  }
}, { immediate: true })

function rememberTarget(target: { modId: string, directory: string }): void {
  if (project.value?.modId !== target.modId) {
    return
  }
  const current = projects.projects.find(project => project.modId === target.modId) ?? project.value
  project.value = { ...current, lastDeploymentDirectory: target.directory }
  projects.upsert(project.value)
}

async function send(): Promise<void> {
  if (sending.value || editingFollowup.value || readingAttachments.value || opening.value || session.readOnly || session.followups.busy || (session.busy && !session.working)) {
    return
  }
  const submittingKey = session.selectionId
  const target = { modId: session.modId, sessionId: session.sessionId }
  if (!target.sessionId || (!requirement.value.trim() && !attachments.value.length && !sourceReferences.value[target.modId]?.length)) {
    return
  }
  sending.value = true
  error.value = ''
  try {
    const guard = sourceLeaveGuards.get(target.modId)
    if (guard && !await guard()) {
      return
    }
    const reference = await referencePrompt(target.modId)
    if (session.selectionId !== submittingKey) {
      return
    }
    const submitted = [requirement.value, reference.text].filter(Boolean).join('\n\n')
    const saved = await session.submitInput(target, submitted, draft.captureSubmitted)
    for (const entry of reference.entries) {
      removeSourceReference(target.modId, entry.path)
    }
    if (!saved && session.selectionId === submittingKey) {
      error.value = '消息已提交，草稿保存失败，请勿重复发送'
    }
  }
  catch (cause) {
    if (session.selectionId === submittingKey) {
      error.value = (cause as Error).message
    }
  }
  finally { sending.value = false }
}

async function restorePendingInput(): Promise<void> {
  const target = { modId: session.modId, sessionId: session.sessionId }
  try {
    await session.restorePendingInput(target, (text) => {
      if (requirement.value && requirement.value !== text) {
        throw new Error('当前已有新草稿，请先保存或移走草稿，再恢复保留的提交')
      }
      requirement.value = text
      return () => draft.flush()
    })
  }
  catch (cause) { error.value = (cause as Error).message }
}
const handledRepair = new Set<string>()
watch(() => [String(route.query.repair ?? ''), opening.value, loadingProject.value, verification.data.value.records] as const, async ([recordId, isOpening, isLoading, records]) => {
  if (!recordId || isOpening || isLoading || handledRepair.has(recordId) || session.modId !== project.value?.modId) {
    return
  }
  const record = records.find(item => item.recordId === recordId)
  if (!record) {
    return
  }
  handledRepair.add(recordId)
  await repair(record)
  await router.replace({ path: route.path, query: { session: session.sessionId } })
})
async function leave(): Promise<boolean> {
  if (followupEdit) {
    try {
      await followupEdit
    }
    catch { /* 编辑失败时由队列保留来源关联，草稿仍需保存。 */ }
  }
  if (saveTimer) {
    clearTimeout(saveTimer)
    saveTimer = undefined
  }
  return project.value && (project.value.archived || projects.isRemoved(project.value.modId)) ? true : draft.flush()
}
watch(() => deliveries.projectFor(project.value?.modId ?? ''), (value) => {
  if (value && value.modId === project.value?.modId) {
    project.value = value
  }
})
/** 原生终态事件之后读取任务结束时间及新版本，不用前端猜测完成。 */
watch(() => [session.modId, session.sessionId, sourceRevision(session.modId), deliveries.revisionFor(session.modId), session.task?.taskId], async (_value, _previous, onCleanup) => {
  let valid = true
  onCleanup(() => {
    valid = false
  })
  const id = project.value?.modId
  if (!id || session.modId !== id) {
    return
  }
  try {
    const records = await sessionService.tasks({ modId: id })
    if (!valid || project.value?.modId !== id) {
      return
    }
    tasks.value = records
  }
  catch (cause) {
    if (valid) {
      error.value = (cause as Error).message
    }
  }
}, { flush: 'sync' })
onBeforeUnmount(() => {
  if (saveTimer) {
    clearTimeout(saveTimer)
  }
})
onBeforeRouteLeave(leave)
onBeforeRouteUpdate(leave)
</script>

<template>
  <div class="flex h-full min-h-0 flex-col">
    <p v-if="project && session.task?.contextSnapshotId" class="shrink-0 px-4 py-2 text-xs text-muted-foreground">
      本轮使用项目记忆快照 {{ session.task.contextSnapshotId.slice(0, 8) }}
    </p>
    <Alert v-if="displayedIssue" :role="runtimeClosed ? 'status' : 'alert'" :variant="runtimeClosed ? 'default' : 'destructive'" class="shrink-0">
      <Unplug v-if="runtimeClosed" />
      <AlertTitle>{{ connectionIssue ? (runtimeClosed ? '会话连接已断开' : '会话连接异常') : '会话操作提示' }}</AlertTitle>
      <AlertDescription class="flex flex-wrap items-center justify-between gap-3">
        <span class="min-w-0 flex-1 break-words">{{ displayedIssue }}</span>
        <Button v-if="session.restartAvailable" variant="outline" size="sm" class="h-7 shrink-0 gap-1.5 rounded-md text-xs" :disabled="opening" @click="session.restart().catch(cause => error = cause.message)">
          <RotateCw class="size-3.5" />重启运行时
        </Button>
        <Button v-else-if="project && reconnectId && (connectionIssue || session.uncertain)" variant="outline" size="sm" class="h-7 shrink-0 gap-1.5 rounded-md text-xs" :disabled="opening || session.synchronizing" @click="reconnect">
          <RotateCw class="size-3.5" />{{ opening ? '正在连接…' : '重新连接' }}
        </Button>
      </AlertDescription>
    </Alert>
    <Alert v-if="session.pendingInput" class="shrink-0">
      <AlertTitle>{{ session.uncertain ? '提交结果待确认' : session.pendingStatus === 'rejected' ? '提交未被接收' : '原始提交内容已保留' }}</AlertTitle>
      <AlertDescription class="flex flex-col gap-2">
        <span class="max-h-24 overflow-auto whitespace-pre-wrap break-words">{{ session.pendingInput.text }}</span>
        <span v-if="session.pendingInput.attachments.length">附件：{{ session.pendingInput.attachments.map(file => file.name).join('、') }}</span>
        <Button v-if="session.uncertain" variant="outline" size="sm" :disabled="opening || session.synchronizing" @click="reconnect">
          核对提交结果
        </Button>
        <Button v-else-if="session.pendingStatus === 'rejected'" variant="outline" size="sm" @click="restorePendingInput">
          恢复到输入框
        </Button>
        <Button v-else-if="session.pendingStatus === 'accepted'" variant="outline" size="sm" @click="session.clearPending().catch(cause => error = cause.message)">
          已查看提交结果，关闭保留提示
        </Button>
      </AlertDescription>
    </Alert>
    <p v-if="opening" class="px-5 py-3 text-xs text-muted-foreground">
      正在同步会话状态…
    </p>
    <MessageTimeline v-if="project && session.modId === project.modId && (session.chat.messages.length || session.working)" :key="`${session.modId}-${session.sessionId}`" :project="project" :focus-task-id="String(route.query.task ?? '')" :messages="timelineMessages" :tools="session.chat.tools" :mod-id="String(route.params.modId)" :tasks="tasks.filter(item => item.context.sessionId === session.sessionId)" :working="session.working" :started-at="session.startedAt" :ended-at="session.endedAt" :activity="session.activity" :last-activity-at="session.lastActivityAt" :feedback-drafts="pendingFeedback" :verification-records="verification.data.value.records" @target-selected="rememberTarget" @feedback-changed="verification.refresh" />
    <div v-else-if="loadingProject || opening" role="status" class="flex min-h-0 flex-1 items-center justify-center gap-2 text-sm text-muted-foreground">
      <Spinner aria-hidden="true" />
      <span>正在加载会话…</span>
    </div>
    <Empty v-else class="min-h-0 overflow-auto rounded-none">
      <EmptyHeader>
        <EmptyDescription>
          {{ project?.gameSkillName }}
        </EmptyDescription><EmptyTitle>
          你的 MOD，想从哪里开始？
        </EmptyTitle><EmptyDescription>
          描述希望实现的行为。源码会保存在项目中，每次成功交付形成独立版本。
        </EmptyDescription>
      </EmptyHeader>
    </Empty>
    <Collapsible v-if="verification.data.value.records.length" class="max-h-56 shrink-0 overflow-auto border-t px-5 py-2">
      <CollapsibleTrigger as-child>
        <Button variant="ghost" size="sm">
          玩家验收历史（{{ verification.data.value.records.length }}）
        </Button>
      </CollapsibleTrigger>
      <CollapsibleContent>
        <VerificationHistory :records="verification.data.value.records" :read-only="session.readOnly" @repair="repair" />
      </CollapsibleContent>
    </Collapsible>
    <p v-if="verification.error.value" role="alert" class="px-5 text-xs text-destructive">
      {{ verification.error.value }}
    </p>
    <p v-if="session.readOnly" class="border-t px-5 py-5 text-xs text-muted-foreground">
      {{ project?.archived ? '项目已归档，仅可查看；在全部项目的“已归档项目”中恢复后继续制作。' : '此会话使用旧的能力上下文，只能查看历史。创建新会话继续开发。' }}
    </p>
    <FollowupQueue v-if="project" :items="session.followups.items" :editing-id="session.followups.editingId" :pending="pendingSteering" :paused="session.followups.paused" :busy="session.followups.busy || sending || readingAttachments || opening" :working="session.working" :can-steer="session.canSteer" :can-send="!session.busy && !session.pendingInput && modelConfigured" :has-draft="!!requirement || !!attachments.length" :read-only="session.readOnly" :error="session.followups.error" @edit="editFollowup" @remove="operateFollowup('remove', $event)" @deliver="operateFollowup('deliver', $event)" @resume="operateFollowup('resume')" />
    <ChatComposer v-if="project && !session.readOnly" v-model="requirement" :restoring="editingFollowup" :attachments="attachments" :reading-attachments="readingAttachments" :working="session.working" :can-queue="session.working && !sending && !opening && !session.followups.busy" :stopping="session.stopping" :busy="session.busy || sending || opening || session.followups.busy" :configured="modelConfigured && !opening" :providers="status?.providers || []" :model-state="session.modelState" :error="attachmentError || saveError" :thinking-level="session.thinkingLevel" :thinking-levels="session.thinkingLevels" :context-usage="session.contextUsage" :compacting="session.compacting" :can-compact="!!session.chat.messages.length" @files="attachFiles" @remove-attachment="session.removeAttachment" @send="send" @save="draft.flush" @cancel="session.abort" @model="session.selectModel" @thinking="session.setThinking" @compact="session.compact" />
  </div>
</template>
