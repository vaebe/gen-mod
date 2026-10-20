<script setup lang="ts">
import type { FileChange, FileDiff as FileDiffData } from '@/contracts/changes'
import type { ModProject } from '@/contracts/mod'
import type { VersionAction } from '@/features/mods/VersionActions.vue'
import { ArrowUpRight, FileText, RefreshCw } from '@lucide/vue'
import { useIntervalFn } from '@vueuse/core'
import { computed, defineAsyncComponent, shallowRef, watch } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import { Dialog, DialogContent, DialogDescription, DialogHeader, DialogTitle } from '@/components/ui/dialog'
import { ScrollArea } from '@/components/ui/scroll-area'
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs'
import { ToggleGroup, ToggleGroupItem } from '@/components/ui/toggle-group'
import { Tooltip, TooltipContent, TooltipProvider, TooltipTrigger } from '@/components/ui/tooltip'
import ArtifactPanel from '@/features/mods/ArtifactPanel.vue'
import InstallDialog from '@/features/mods/InstallDialog.vue'
import VersionActions from '@/features/mods/VersionActions.vue'
import { nativeCall } from '@/services/native'
import { useProjectsStore } from '@/stores/projects'
import { useSessionStore } from '@/stores/session'
import ChangedFiles from './ChangedFiles.vue'
import FileDiff from './FileDiff.vue'
import SourceBrowser from './SourceBrowser.vue'

const session = useSessionStore()
const ProjectMemoryTab = defineAsyncComponent(() => import('./ProjectMemoryTab.vue'))
const route = useRoute()
const router = useRouter()
const changes = shallowRef<FileChange[]>([])
const operations = shallowRef<{ toolCallId: string, operation: string, path?: string, sourceRevision: number, fileChanges?: { path: string }[] }[]>([])
const diff = shallowRef<FileDiffData>()
const selected = shallowRef('')
const error = shallowRef('')
const project = shallowRef<ModProject>()
const projects = useProjectsStore()
const artifactProject = computed(() => projects.projects.find(item => item.modId === session.modId) ?? (project.value?.modId === session.modId ? project.value : undefined))
const artifactVersionId = shallowRef('')
const artifactDetailsOpen = shallowRef(false)
const selectedArtifact = computed(() => artifactProject.value?.versions.find(version => version.versionId === artifactVersionId.value))
const artifactBusy = shallowRef(false)
const artifactInstalling = shallowRef(false)
const artifactDeleting = shallowRef(false)
const artifactError = shallowRef('')
const artifactNotice = shallowRef('')
const previousTarget = computed(() => artifactProject.value?.lastDeploymentDirectory ?? artifactProject.value?.installations.at(-1)?.targetDirectory)
const artifactActionsBusy = computed(() => artifactBusy.value || artifactInstalling.value || artifactDeleting.value)
async function refreshArtifacts(): Promise<void> {
  const modId = session.modId
  try {
    const result = await nativeCall<ModProject>('get_mod', { modId })
    if (modId === session.modId) {
      projects.upsert(result)
    }
  }
  catch (cause) {
    if (modId === session.modId) {
      artifactError.value = (cause as Error).message
    }
  }
}
function rememberTarget(directory: string): void {
  if (artifactProject.value) {
    projects.upsert({ ...artifactProject.value, lastDeploymentDirectory: directory })
  }
}
async function artifactAction(kind: VersionAction, id: string): Promise<void> {
  const modId = session.modId
  if (artifactActionsBusy.value || !artifactProject.value?.versions.some(version => version.versionId === id)) {
    return
  }
  artifactVersionId.value = id
  artifactError.value = ''
  artifactNotice.value = ''
  if (kind === 'details') {
    viewArtifact(id)
    return
  }
  if (kind === 'install') {
    artifactInstalling.value = true
    return
  }
  if (kind === 'delete') {
    artifactDeleting.value = true
    return
  }
  artifactBusy.value = true
  try {
    if (kind === 'export') {
      const path = await nativeCall<string | null>('export_version', { modId, versionId: id })
      if (modId === session.modId && path) {
        artifactNotice.value = `已导出：${path}`
      }
    }
    else {
      await nativeCall('open_mod_directory', { modId, versionId: id })
    }
  }
  catch (cause) {
    if (modId === session.modId) {
      artifactError.value = (cause as Error).message
    }
  }
  finally { artifactBusy.value = false }
}
async function deleteArtifact(): Promise<void> {
  if (!selectedArtifact.value || artifactBusy.value) {
    return
  }
  const modId = session.modId
  const versionId = selectedArtifact.value.versionId
  artifactBusy.value = true
  try {
    const result = await nativeCall<ModProject>('delete_version', { modId, versionId })
    projects.upsert(result)
    if (modId === session.modId) {
      artifactVersionId.value = ''
      artifactDetailsOpen.value = false
      artifactNotice.value = '版本已删除，已部署文件和安装记录保留。'
      session.artifactRevision += 1
    }
  }
  catch (cause) {
    if (modId === session.modId) {
      artifactError.value = (cause as Error).message
      await refreshArtifacts()
    }
  }
  finally {
    artifactDeleting.value = false
    artifactBusy.value = false
  }
}
function viewArtifact(id: string): void {
  artifactVersionId.value = id
  artifactDetailsOpen.value = true
}
watch(() => session.modId, () => {
  artifactVersionId.value = ''
  artifactDetailsOpen.value = false
  artifactInstalling.value = false
  artifactDeleting.value = false
  artifactError.value = ''
  artifactNotice.value = ''
})
// 与版本管理页一致，只标记目标当前确认已安装的版本。
const installedVersionIds = computed(() => new Set(artifactProject.value?.installedTargets.flatMap(target => target.state === 'known' && target.versionId ? [target.versionId] : []) ?? []))
interface RuntimeLogEntry { timestamp: string, generation: number, taskId?: string, message: string }
const runtimeLogs = shallowRef<RuntimeLogEntry[]>([])
const logError = shallowRef('')
const selectedTab = shallowRef('files')
watch(() => route.query.context, (value) => {
  if (value === 'memory') {
    selectedTab.value = 'memory'
  }
}, { immediate: true })
watch(selectedTab, (value) => {
  if (value !== 'memory' && route.query.context === 'memory') {
    void router.replace({ query: { ...route.query, context: undefined } })
  }
})
const fileMode = shallowRef('all')
let readingLogs = false
/** 只刷新可见日志页；切换会话后丢弃旧请求，历史从磁盘恢复。 */
async function refreshLogs(): Promise<void> {
  if (readingLogs || selectedTab.value !== 'logs' || !session.modId || !session.sessionId) {
    return
  }
  readingLogs = true
  const modId = session.modId
  const sessionId = session.sessionId
  try {
    const entries = await nativeCall<RuntimeLogEntry[]>('read_session_runtime_logs', { modId, sessionId })
    if (modId === session.modId && sessionId === session.sessionId) {
      runtimeLogs.value = entries
      logError.value = ''
    }
  }
  catch (cause) {
    if (modId === session.modId && sessionId === session.sessionId) {
      logError.value = (cause as Error).message
    }
  }
  finally { readingLogs = false }
}
watch(() => [session.modId, session.sessionId], () => {
  runtimeLogs.value = []
  logError.value = ''
  void refreshLogs()
})
watch(selectedTab, () => {
  void refreshLogs()
})
useIntervalFn(() => {
  void refreshLogs()
}, 3000)
watch(() => [session.modId, session.artifactRevision, session.changeRevision], async (_value, _old, onCleanup) => {
  let valid = true
  onCleanup(() => {
    valid = false
  })
  project.value = undefined
  if (!session.modId) {
    return
  }
  try {
    const result = await nativeCall<ModProject>('get_mod', { modId: session.modId })
    if (valid) {
      project.value = result
      projects.upsert(result)
    }
  }
  catch (cause) {
    if (valid) {
      error.value = (cause as Error).message
    }
  }
}, { immediate: true })
async function openLogs(): Promise<void> {
  if (!session.task) {
    return
  }
  try {
    await nativeCall('open_task_logs', { modId: session.modId, taskId: session.task.taskId })
  }
  catch (cause) { error.value = (cause as Error).message }
}
let generation = 0
async function select(path: string): Promise<void> {
  const current = ++generation
  diff.value = undefined
  selected.value = path
  error.value = ''
  if (!session.task) {
    return
  }
  try {
    const result = await nativeCall<FileDiffData>('get_task_file_diff', { modId: session.modId, taskId: session.task.taskId, relativePath: path })
    if (current === generation) {
      diff.value = result
    }
  }
  catch (cause) {
    if (current === generation) {
      error.value = (cause as Error).message
    }
  }
}
watch(() => [session.task?.taskId, session.changeRevision], async (_value, _old, onCleanup) => {
  let valid = true
  onCleanup(() => {
    valid = false
  })
  changes.value = []
  operations.value = []
  diff.value = undefined
  error.value = ''
  generation += 1
  if (!session.task) {
    return
  }
  try {
    const recorded = await nativeCall<typeof operations.value>('list_task_operations', { modId: session.modId, taskId: session.task.taskId })
    if (!valid) {
      return
    }
    operations.value = recorded
    const result = await nativeCall<FileChange[]>('list_task_changes', { modId: session.modId, taskId: session.task.taskId })
    if (!valid) {
      return
    }
    changes.value = result
    if (selected.value && result.some(item => item.path === selected.value)) {
      await select(selected.value)
    }
  }
  catch (cause) {
    if (valid) {
      error.value = (cause as Error).message
    }
  }
}, { immediate: true })
</script>

<template>
  <section class="flex h-full min-h-0 flex-col border-l bg-card/50">
    <div class="px-4 pt-4 pb-3 text-sm font-medium">
      项目上下文
    </div>
    <Tabs v-model="selectedTab" class="flex min-h-0 flex-1 flex-col px-3">
      <TabsList class="w-full">
        <TabsTrigger value="files" class="flex-1">
          文件
        </TabsTrigger><TabsTrigger value="logs" class="flex-1">
          日志
        </TabsTrigger><TabsTrigger value="artifacts" class="flex-1">
          产物
        </TabsTrigger><TabsTrigger value="memory" class="flex-1">
          记忆
        </TabsTrigger>
      </TabsList>
      <TabsContent value="files" class="flex min-h-0 flex-1 flex-col overflow-hidden data-[state=inactive]:hidden">
        <ToggleGroup type="single" :model-value="fileMode" size="sm" class="shrink-0 border-b p-2" aria-label="文件查看范围" @update:model-value="($event === 'all' || $event === 'changes') && (fileMode = $event)">
          <ToggleGroupItem value="all" class="flex-1">
            全部文件
          </ToggleGroupItem>
          <ToggleGroupItem value="changes" class="flex-1">
            当前变更
          </ToggleGroupItem>
        </ToggleGroup>
        <SourceBrowser v-show="fileMode === 'all'" :mod-id="session.modId" :revision="session.changeRevision" :read-only="artifactProject?.archived" :active="selectedTab === 'files' && fileMode === 'all'" />
        <ScrollArea v-if="fileMode === 'changes'" class="min-h-0 flex-1">
          <div v-if="session.task" class="space-y-2 border-b p-3 text-xs">
            <p class="font-medium">
              本任务工具操作记录
            </p><p v-if="!operations.length" class="text-muted-foreground">
              暂无已记录的源码写入操作。
            </p><p v-for="item in operations" :key="item.toolCallId" class="break-all text-muted-foreground">
              {{ item.operation }} · {{ item.path || (`${item.fileChanges?.length} 个文件`) }} · revision {{ item.sourceRevision }}
            </p><p class="pt-2 font-medium">
              共享源码变化
            </p><p class="text-muted-foreground">
              下方是任务开始后的共享目录差异，可能包含其他会话操作。
            </p>
          </div><p v-if="!session.task" class="p-3 text-xs leading-6 text-muted-foreground">
            任务开始后显示源码变更与差异。
          </p><ChangedFiles v-else :changes="changes" :selected="selected" @select="select" /><FileDiff v-if="diff" :diff="diff" :mod-id="session.modId" /><p v-if="error" role="alert" class="p-3 text-xs text-destructive">
            {{ error }}
          </p>
        </ScrollArea>
      </TabsContent>
      <TabsContent value="logs" class="flex min-h-0 flex-1 flex-col overflow-hidden data-[state=inactive]:hidden">
        <div class="flex shrink-0 items-center justify-between gap-3 border-b px-3 py-2">
          <div class="flex min-w-0 flex-col gap-1">
            <p class="text-sm font-medium">
              会话日志
            </p>
            <p class="text-xs text-muted-foreground">
              最近 200 条 · 每 3 秒自动刷新
            </p>
          </div>
          <TooltipProvider :delay-duration="150">
            <div class="flex shrink-0 items-center gap-1">
              <Tooltip v-if="session.task">
                <TooltipTrigger as-child>
                  <Button size="icon-sm" variant="ghost" aria-label="打开任务完整日志" @click="openLogs">
                    <FileText aria-hidden="true" />
                  </Button>
                </TooltipTrigger>
                <TooltipContent>打开任务完整日志</TooltipContent>
              </Tooltip>
              <Tooltip>
                <TooltipTrigger as-child>
                  <Button size="icon-sm" variant="ghost" aria-label="刷新日志" @click="refreshLogs">
                    <RefreshCw aria-hidden="true" />
                  </Button>
                </TooltipTrigger>
                <TooltipContent>刷新日志</TooltipContent>
              </Tooltip>
            </div>
          </TooltipProvider>
        </div>
        <ScrollArea class="min-h-0 flex-1">
          <p v-if="logError" role="alert" class="p-3 text-xs text-destructive">
            {{ logError }}
          </p><p v-if="!runtimeLogs.length" class="p-3 text-xs text-muted-foreground">
            暂无已保存的运行日志。更新前的运行摘要不会补录，既有命令日志仍可通过上方按钮查看。
          </p><div v-for="(entry, index) in runtimeLogs" :key="`${entry.timestamp}-${index}`" class="border-b px-3 py-2 text-[11px] leading-5">
            <p class="text-muted-foreground">
              {{ new Date(entry.timestamp).toLocaleString() }} · {{ entry.taskId ? `任务 ${entry.taskId.slice(0, 8)}` : '连接' }}
            </p><pre class="mt-1 whitespace-pre-wrap break-all">{{ entry.message }}</pre>
          </div><template v-if="session.diagnostics.length">
            <p class="px-3 pt-4 text-xs font-medium">
              当前连接诊断
            </p><pre v-for="(line, index) in session.diagnostics" :key="index" class="border-b px-3 py-2 font-mono text-[11px] leading-5 whitespace-pre-wrap break-all">{{ line }}</pre>
          </template>
        </ScrollArea>
      </TabsContent>
      <TabsContent value="artifacts" class="flex min-h-0 flex-1 flex-col overflow-hidden data-[state=inactive]:hidden">
        <div v-if="session.modId" class="flex shrink-0 items-center justify-between gap-3 border-b px-3 py-2">
          <div class="flex min-w-0 flex-wrap items-baseline gap-x-2 gap-y-1">
            <p class="text-sm font-medium">
              交付版本
            </p>
            <span class="text-xs text-muted-foreground">共 {{ artifactProject?.versions.length ?? 0 }} 个版本</span>
          </div>
          <TooltipProvider :delay-duration="150">
            <Tooltip>
              <TooltipTrigger as-child>
                <Button as-child size="icon-sm" variant="ghost" aria-label="版本与安装管理">
                  <RouterLink :to="`/mods/${session.modId}/versions`">
                    <ArrowUpRight aria-hidden="true" />
                  </RouterLink>
                </Button>
              </TooltipTrigger>
              <TooltipContent>版本与安装管理</TooltipContent>
            </Tooltip>
          </TooltipProvider>
        </div>
        <ScrollArea class="min-h-0 flex-1" role="region" aria-label="产物版本列表">
          <p v-if="!artifactProject?.versions.length" class="p-3 text-xs leading-6 text-muted-foreground">
            尚无成功交付版本。
          </p>
          <p v-if="artifactError" role="alert" class="p-3 text-xs text-destructive">
            {{ artifactError }}
          </p>
          <p v-if="artifactNotice" role="status" class="p-3 text-xs">
            {{ artifactNotice }}
          </p>
          <div v-for="version in [...(artifactProject?.versions ?? [])].reverse()" :key="version.versionId" class="flex flex-col gap-1 border-b px-3 py-2.5 text-sm">
            <div class="flex items-center justify-between gap-2">
              <div class="flex min-w-0 flex-wrap items-center gap-1.5">
                <span class="font-mono font-medium" :title="version.versionId">{{ version.versionId.slice(0, 8) }}</span>
                <Badge v-if="artifactProject?.latestVersionId === version.versionId" variant="secondary" class="shrink-0 border-blue-200 bg-blue-100 px-1.5 py-0 text-[10px] leading-4 text-blue-700 hover:bg-blue-100 dark:border-blue-800 dark:bg-blue-950 dark:text-blue-300 dark:hover:bg-blue-950">
                  最新
                </Badge>
                <Badge v-if="installedVersionIds.has(version.versionId)" variant="outline" class="shrink-0 border-green-200 bg-green-100 px-1.5 py-0 text-[10px] leading-4 text-green-700 dark:border-green-800 dark:bg-green-950 dark:text-green-300">
                  已安装
                </Badge>
              </div>
              <VersionActions :version-id="version.versionId" :busy="artifactActionsBusy" :read-only="artifactProject?.archived" @action="artifactAction" />
            </div>
            <p class="flex flex-wrap items-center gap-x-2 gap-y-1 text-xs text-muted-foreground">
              <span>{{ version.files.length }} 个文件</span>
              <time :datetime="version.createdAt">{{ new Date(version.createdAt).toLocaleString() }}</time>
            </p>
          </div>
        </ScrollArea>
      </TabsContent>
      <TabsContent value="memory" class="flex min-h-0 flex-1 flex-col overflow-hidden data-[state=inactive]:hidden">
        <ProjectMemoryTab :mod-id="session.modId" :revision="session.memoryRevision" :active="selectedTab === 'memory'" :read-only="session.readOnly" />
      </TabsContent>
    </Tabs>
    <ArtifactPanel v-if="selectedArtifact" v-model:open="artifactDetailsOpen" :version="selectedArtifact" :mod-id="session.modId" @repair="record => { artifactDetailsOpen = false; $router.push({ path: `/mods/${record.modId}/chat`, query: { repair: record.recordId } }) }" />
    <InstallDialog v-if="selectedArtifact && artifactProject" v-model:open="artifactInstalling" :mod-id="artifactProject.modId" :version-id="selectedArtifact.versionId" :previous-target="previousTarget" @target-selected="rememberTarget" @installed="refreshArtifacts" />
    <Dialog :open="artifactDeleting" @update:open="value => !artifactBusy && (artifactDeleting = value)">
      <DialogContent @interact-outside="artifactBusy && $event.preventDefault()" @escape-key-down="artifactBusy && $event.preventDefault()">
        <DialogHeader><DialogTitle>删除版本 {{ artifactVersionId.slice(0, 8) }}</DialogTitle><DialogDescription>删除应用内的版本文件和 ZIP，无法撤销。已部署到游戏目录的文件、源码及安装记录会保留。</DialogDescription></DialogHeader>
        <p v-if="installedVersionIds.has(artifactVersionId)" class="text-sm text-muted-foreground">
          此版本仍有已安装记录；删除后仍可继续使用已部署的文件。
        </p>
        <div class="flex justify-end gap-2">
          <Button variant="ghost" :disabled="artifactBusy" @click="artifactDeleting = false">
            取消
          </Button><Button variant="destructive" :disabled="artifactBusy" @click="deleteArtifact">
            {{ artifactBusy ? '正在删除…' : '删除版本' }}
          </Button>
        </div>
      </DialogContent>
    </Dialog>
  </section>
</template>
