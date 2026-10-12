<script setup lang="ts">
import type { VersionAction } from './VersionList.vue'
import type { ModProject } from '@/contracts/mod'
import { computed, shallowRef, watch } from 'vue'
import { useRoute } from 'vue-router'
import { Button } from '@/components/ui/button'
import { Dialog, DialogContent, DialogDescription, DialogHeader, DialogTitle } from '@/components/ui/dialog'
import { displayPath } from '@/lib/displayPath'
import { modService, nativeCall } from '@/services/native'
import { useProjectsStore } from '@/stores/projects'
import { useSessionStore } from '@/stores/session'
import ArtifactPanel from './ArtifactPanel.vue'
import InstallDialog from './InstallDialog.vue'
import VersionList from './VersionList.vue'

const route = useRoute()
const project = shallowRef<ModProject>()
const selectedId = shallowRef('')
const selected = computed(() => project.value?.versions.find(v => v.versionId === selectedId.value))
const error = shallowRef('')
const notice = shallowRef('')
const busy = shallowRef(false)
const installing = shallowRef(false)
const deleting = shallowRef(false)
const detailsOpen = shallowRef(false)
const historyOpen = shallowRef(false)
/** 只标记仍在目标状态中已知的当前版本，历史安装成功不等于当前安装。 */
const installedIds = computed(() => project.value?.installedTargets.filter(target => target.state === 'known' && target.versionId).map(target => target.versionId!) ?? [])
const installationSummary = computed(() => {
  const targets = project.value?.installedTargets ?? []
  if (!targets.length) {
    return '尚未安装'
  }
  if (targets.length === 1) {
    const target = targets[0]!
    return target.state === 'uncertain' ? '目标状态不确定' : `当前版本 ${target.versionId?.slice(0, 8) ?? '未知'}`
  }
  const uncertain = targets.filter(target => target.state === 'uncertain').length
  return `${targets.length} 个目标目录${uncertain ? ` · ${uncertain} 个状态不确定` : ''}`
})
const previousTarget = computed(() => {
  const value = project.value
  return value?.lastDeploymentDirectory ?? value?.installations[value.installations.length - 1]?.targetDirectory
})
const projects = useProjectsStore()
const session = useSessionStore()
let generation = 0
async function refresh(): Promise<void> {
  const id = String(route.params.modId)
  const current = ++generation
  try {
    const result = await modService.get(id)
    if (current !== generation) {
      return
    }
    project.value = result
    projects.upsert(result)
    if (!result.versions.some(version => version.versionId === selectedId.value)) {
      selectedId.value = result.latestVersionId ?? ''
      detailsOpen.value = false
      deleting.value = false
      installing.value = false
    }
  }
  catch (cause) {
    if (current === generation) {
      error.value = (cause as Error).message
    }
  }
}
watch(() => String(route.params.modId), () => {
  project.value = undefined
  selectedId.value = ''
  error.value = ''
  notice.value = ''
  detailsOpen.value = false
  historyOpen.value = false
  installing.value = false
  deleting.value = false
  void refresh()
}, { immediate: true })
watch(() => projects.projects.find(item => item.modId === String(route.params.modId))?.versions.length, refresh)
async function action(kind: 'source' | 'version' | 'export', versionId?: string): Promise<void> {
  if (!project.value || busy.value || (kind !== 'source' && !project.value.versions.some(version => version.versionId === versionId))) {
    return
  }
  busy.value = true
  error.value = ''
  notice.value = ''
  try {
    if (kind === 'export') {
      const path = await nativeCall<string | null>('export_version', { modId: project.value.modId, versionId })
      if (path) {
        notice.value = `已导出：${path}`
      }
    }
    else { await nativeCall('open_mod_directory', { modId: project.value.modId, versionId: kind === 'version' ? versionId : null }) }
  }
  catch (cause) { error.value = (cause as Error).message }
  finally { busy.value = false }
}
/** 行菜单显式传入版本 ID，避免操作落到上次查看的版本。 */
function versionAction(kind: VersionAction, id: string): void {
  if (busy.value || installing.value || deleting.value || !project.value?.versions.some(version => version.versionId === id)) {
    return
  }
  selectedId.value = id
  if (kind === 'details') {
    detailsOpen.value = true
  }
  else if (kind === 'install') {
    installing.value = true
  }
  else if (kind === 'delete') {
    deleting.value = true
  }
  else { void action(kind === 'open' ? 'version' : 'export', id) }
}
function rememberTarget(directory: string): void {
  if (!project.value) {
    return
  }
  project.value = { ...project.value, lastDeploymentDirectory: directory }
  projects.upsert(project.value)
}
/** 删除选定的应用内版本包；原生负责目录边界与元数据更新。 */
async function deleteVersion(): Promise<void> {
  if (!project.value || !selected.value || busy.value) {
    return
  }
  const id = selected.value.versionId
  busy.value = true
  error.value = ''
  try {
    const result = await nativeCall<ModProject>('delete_version', { modId: project.value.modId, versionId: id })
    project.value = result
    projects.upsert(result)
    selectedId.value = result.latestVersionId ?? ''
    session.artifactRevision += 1
    deleting.value = false
    notice.value = '版本已删除，已部署文件和安装记录保留。'
  }
  catch (cause) {
    error.value = (cause as Error).message
    deleting.value = false
    await refresh()
  }
  finally { busy.value = false }
}
</script>

<template>
  <section class="mx-auto flex h-full min-h-0 max-w-5xl flex-col gap-3 overflow-hidden px-4 py-4 lg:px-6">
    <header class="flex shrink-0 flex-wrap items-center justify-between gap-2">
      <div class="flex min-w-0 items-baseline gap-3">
        <h1 class="truncate text-base font-semibold" :title="project?.name">
          {{ project?.name ?? '版本管理' }}
        </h1>
        <span v-if="project" class="shrink-0 text-xs text-muted-foreground">{{ project.versions.length }} 个版本</span>
      </div>
      <div class="flex shrink-0 gap-1">
        <Button size="sm" variant="outline" class="h-7 text-xs" :disabled="busy || !project" @click="action('source')">
          打开源码
        </Button>
        <Button v-if="project" as-child size="sm" variant="ghost" class="h-7 text-xs">
          <RouterLink :to="`/mods/${project.modId}/chat`">
            返回对话
          </RouterLink>
        </Button>
      </div>
    </header>
    <div v-if="error || notice" class="max-h-20 shrink-0 overflow-y-auto text-xs">
      <p v-if="error" role="alert" class="break-all text-destructive">
        {{ error }}
      </p>
      <p v-if="notice" role="status" class="break-all">
        {{ notice }}
      </p>
    </div>
    <template v-if="project">
      <div class="flex shrink-0 items-start gap-2 rounded-lg border bg-muted/20 px-3 py-2">
        <details class="min-w-0 flex-1">
          <summary class="cursor-pointer text-xs leading-7">
            当前安装：{{ installationSummary }}
          </summary>
          <div class="max-h-28 space-y-3 overflow-y-auto pb-1 pt-2 text-xs">
            <p v-if="!project.installedTargets.length" class="text-muted-foreground">
              尚未安装任何版本。
            </p>
            <div v-for="target in project.installedTargets" :key="target.targetDirectory">
              <p class="break-all font-mono">
                {{ displayPath(target.targetDirectory) }}
              </p>
              <p class="mt-1" :class="target.state === 'uncertain' ? 'text-destructive' : 'text-muted-foreground'">
                {{ target.state === 'uncertain' ? '目标状态不确定，请检查部署文件' : `当前版本 ${target.versionId?.slice(0, 8)}` }}<span v-if="target.state === 'known' && target.versionId !== project.latestVersionId" class="ml-2">最新版本待安装</span>
              </p>
            </div>
          </div>
        </details>
        <Button size="sm" variant="ghost" class="h-7 shrink-0 px-2 text-xs" @click="historyOpen = true">
          安装记录（{{ project.installations.length }}）
        </Button>
      </div>
      <VersionList :versions="project.versions" :latest="project.latestVersionId" :installed="installedIds" :busy="busy || installing || deleting" @action="versionAction" />
      <ArtifactPanel v-if="selected" v-model:open="detailsOpen" :version="selected" />
      <InstallDialog v-if="selected" v-model:open="installing" :mod-id="project.modId" :version-id="selected.versionId" :previous-target="previousTarget" @target-selected="rememberTarget" @installed="refresh" />
      <Dialog v-model:open="historyOpen">
        <DialogContent class="flex max-h-[calc(100dvh-3rem)] flex-col gap-3 overflow-hidden p-5 sm:max-w-2xl">
          <DialogHeader class="shrink-0 pr-6">
            <DialogTitle>安装记录</DialogTitle><DialogDescription>共 {{ project.installations.length }} 条记录，最近安装在前。</DialogDescription>
          </DialogHeader>
          <div class="min-h-0 overflow-y-auto text-xs">
            <p v-if="!project.installations.length" class="py-3 text-muted-foreground">
              暂无安装记录。
            </p>
            <div v-for="record in [...project.installations].reverse()" :key="record.installationId" class="border-b py-3 first:pt-0 last:border-b-0">
              <p>{{ record.versionId.slice(0, 8) }} · {{ record.status === 'succeeded' ? '成功' : '失败' }} · {{ new Date(record.attemptedAt).toLocaleString() }}</p>
              <p class="mt-1 break-all">
                {{ displayPath(record.targetDirectory) }}
              </p>
              <p v-if="record.error" class="mt-1 break-all text-destructive">
                {{ record.error }}{{ record.rollback === 'notAttempted' ? ' · 未备份、未回退' : record.rollback === 'notNeeded' ? '' : ` · 回退 ${record.rollback}` }}
              </p>
              <p v-if="record.backupDirectory" class="mt-1 break-all text-muted-foreground">
                备份 {{ displayPath(record.backupDirectory) }}
              </p>
            </div>
          </div>
        </DialogContent>
      </Dialog>
    </template>
    <p v-else-if="!error" class="text-sm text-muted-foreground">
      正在加载版本…
    </p>
    <Dialog :open="deleting" @update:open="value => !busy && (deleting = value)">
      <DialogContent @interact-outside="busy && $event.preventDefault()" @escape-key-down="busy && $event.preventDefault()">
        <DialogHeader><DialogTitle>删除版本 {{ selectedId.slice(0, 8) }}</DialogTitle><DialogDescription>删除应用内的版本文件和 ZIP，无法撤销。已部署到游戏目录的文件、源码及安装记录会保留。</DialogDescription></DialogHeader>
        <p v-if="installedIds.includes(selectedId)" class="text-sm text-muted-foreground">
          此版本仍有已安装记录；删除后仍可继续使用已部署的文件。
        </p>
        <div class="flex justify-end gap-2">
          <Button variant="ghost" :disabled="busy" @click="deleting = false">
            取消
          </Button><Button variant="destructive" :disabled="busy" @click="deleteVersion">
            {{ busy ? '正在删除…' : '删除版本' }}
          </Button>
        </div>
      </DialogContent>
    </Dialog>
  </section>
</template>
