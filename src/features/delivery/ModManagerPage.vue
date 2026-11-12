<script setup lang="ts">
import type { ModProject } from '@/contracts/mod'
import { computed, defineAsyncComponent, shallowRef, watch } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { Button } from '@/components/ui/button'
import { Collapsible, CollapsibleContent, CollapsibleTrigger } from '@/components/ui/collapsible'
import { Dialog, DialogContent, DialogDescription, DialogHeader, DialogTitle } from '@/components/ui/dialog'
import { useDeliveriesStore } from '@/features/delivery/useDeliveries'
import { modService } from '@/features/projects/service'
import { useVerification } from '@/features/verification/useVerification'
import { displayPath } from '@/lib/displayPath'
import { useProjectsStore } from '@/stores/projects'
import { deliveryService } from './service'
import { useVersionOperations } from './useVersionOperations'
import VersionList from './VersionList.vue'

const router = useRouter()
const route = useRoute()
const project = shallowRef<ModProject>()
const projects = useProjectsStore()
const deliveries = useDeliveriesStore()
const { selectedId, selected, error, notice, busy, installing, deleting, detailsOpen, targetProject, act: versionAction, confirmDelete: deleteVersion } = useVersionOperations({
  project: () => project.value,
  afterDelete: 'latest',
  updateProject: deliveries.update,
})
const ArtifactPanel = defineAsyncComponent(() => import('./ArtifactPanel.vue'))
const InstallDialog = defineAsyncComponent(() => import('./InstallDialog.vue'))
const DeleteVersionDialog = defineAsyncComponent(() => import('./DeleteVersionDialog.vue'))
const historyOpen = shallowRef(false)
async function continueRepair(record: import('@/features/verification/types').VerificationRecord): Promise<void> {
  detailsOpen.value = false
  await router.push({ path: `/mods/${record.modId}/chat`, query: { repair: record.recordId } })
}

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
const verification = useVerification(() => project.value?.modId)
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
watch(() => projects.projects.find(item => item.modId === String(route.params.modId)), (value) => {
  if (value) {
    project.value = value
  }
})
async function openSource(): Promise<void> {
  const modId = project.value?.modId
  if (!modId || busy.value) {
    return
  }
  const current = generation
  busy.value = true
  error.value = ''
  notice.value = ''
  try {
    await deliveryService.openDirectory(modId, null)
  }
  catch (cause) {
    if (current === generation) {
      error.value = (cause as Error).message
    }
  }
  finally {
    if (current === generation) {
      busy.value = false
    }
  }
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
        <Button size="sm" variant="outline" :disabled="busy || !project" @click="openSource">
          打开源码
        </Button>
        <Button v-if="project" as-child size="sm" variant="ghost">
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
        <Collapsible class="min-w-0 flex-1">
          <CollapsibleTrigger as-child>
            <Button type="button" variant="ghost" size="sm" class="h-auto w-full justify-start whitespace-normal text-left text-sm">
              当前安装：{{ installationSummary }}
            </Button>
          </CollapsibleTrigger>
          <CollapsibleContent>
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
          </CollapsibleContent>
        </Collapsible>
        <Button size="sm" variant="ghost" class="h-7 shrink-0 px-2 text-xs" @click="historyOpen = true">
          安装记录（{{ project.installations.length }}）
        </Button>
      </div>
      <VersionList :mod-id="project.modId" :verification-records="verification.data.value.records" :versions="project.versions" :latest="project.latestVersionId" :installed="installedIds" :busy="busy || installing || deleting" :read-only="project.archived" @action="versionAction" />
      <ArtifactPanel v-if="selected && targetProject" v-model:open="detailsOpen" :version="selected" :mod-id="targetProject.modId" @repair="continueRepair" />
      <InstallDialog v-if="selected && targetProject" :key="`${targetProject.modId}:${selected.versionId}`" v-model:open="installing" :mod-id="targetProject.modId" :version-id="selected.versionId" :previous-target="targetProject.lastDeploymentDirectory" />
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
                {{ record.error }}{{ record.rollback === 'notAttempted' ? ' · 未备份、未回退' : '' }}
              </p>
            </div>
          </div>
        </DialogContent>
      </Dialog>
    </template>
    <p v-else-if="!error" class="text-sm text-muted-foreground">
      正在加载版本…
    </p>
    <DeleteVersionDialog v-model:open="deleting" :version-id="selectedId" :installed="installedIds.includes(selectedId)" :busy="busy" @confirm="deleteVersion" />
  </section>
</template>
