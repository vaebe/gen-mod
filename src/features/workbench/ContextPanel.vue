<script setup lang="ts">
import { ArrowUpRight } from '@lucide/vue'
import { computed, defineAsyncComponent, shallowRef, watch } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import { ScrollArea } from '@/components/ui/scroll-area'
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs'
import { Tooltip, TooltipContent, TooltipProvider, TooltipTrigger } from '@/components/ui/tooltip'
import SessionLogsPanel from '@/features/chat/SessionLogsPanel.vue'
import { useDeliveriesStore } from '@/features/delivery/useDeliveries'
import { useVersionOperations } from '@/features/delivery/useVersionOperations'
import VersionActions from '@/features/delivery/VersionActions.vue'
import { useSessionStore } from '@/features/sessions/useSessionStore'
import WorkspacePanel from '@/features/workspace/WorkspacePanel.vue'
import { useProjectsStore } from '@/stores/projects'

const session = useSessionStore()
const ProjectMemoryTab = defineAsyncComponent(() => import('@/features/project-memory/ProjectMemoryTab.vue'))
const route = useRoute()
const router = useRouter()
const projects = useProjectsStore()
const deliveries = useDeliveriesStore()
const artifactProject = computed(() => deliveries.projectFor(session.modId) ?? projects.projects.find(item => item.modId === session.modId))
const { selectedId: artifactVersionId, selected: selectedArtifact, busy: artifactBusy, installing: artifactInstalling, deleting: artifactDeleting, detailsOpen: artifactDetailsOpen, error: artifactError, notice: artifactNotice, actionsBusy: artifactActionsBusy, targetProject: operationProject, act: artifactAction, confirmDelete: deleteArtifact } = useVersionOperations({
  project: () => artifactProject.value,
  afterDelete: 'clear',
  updateProject: deliveries.update,
})
const ArtifactPanel = defineAsyncComponent(() => import('@/features/delivery/ArtifactPanel.vue'))
const InstallDialog = defineAsyncComponent(() => import('@/features/delivery/InstallDialog.vue'))
const DeleteVersionDialog = defineAsyncComponent(() => import('@/features/delivery/DeleteVersionDialog.vue'))
// 与版本管理页一致，只标记目标当前确认已安装的版本。
const installedVersionIds = computed(() => new Set(artifactProject.value?.installedTargets.flatMap(target => target.state === 'known' && target.versionId ? [target.versionId] : []) ?? []))
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
watch(() => session.modId, (id) => {
  if (id) {
    void deliveries.refresh(id).catch(() => {})
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
      <TabsContent value="files" force-mount class="flex min-h-0 flex-1 flex-col overflow-hidden data-[state=inactive]:hidden">
        <WorkspacePanel :mod-id="session.modId" :task-id="session.task?.taskId" :read-only="artifactProject?.archived" :active="selectedTab === 'files'" />
      </TabsContent>
      <TabsContent value="logs" force-mount class="flex min-h-0 flex-1 flex-col overflow-hidden data-[state=inactive]:hidden">
        <SessionLogsPanel :active="selectedTab === 'logs'" />
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
          <p v-if="artifactError || deliveries.errorFor(session.modId)" role="alert" class="p-3 text-xs text-destructive">
            {{ artifactError || deliveries.errorFor(session.modId) }}
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
            <p class="whitespace-pre-wrap break-words text-sm">
              {{ version.summary }}
            </p>
            <p class="flex flex-wrap items-center gap-x-2 gap-y-1 text-xs text-muted-foreground">
              <span>{{ version.files.length }} 个文件</span>
              <time :datetime="version.createdAt">{{ new Date(version.createdAt).toLocaleString() }}</time>
            </p>
          </div>
        </ScrollArea>
      </TabsContent>
      <TabsContent value="memory" class="flex min-h-0 flex-1 flex-col overflow-hidden data-[state=inactive]:hidden">
        <ProjectMemoryTab :mod-id="session.modId" :active="selectedTab === 'memory'" :read-only="session.readOnly" />
      </TabsContent>
    </Tabs>
    <ArtifactPanel v-if="selectedArtifact && operationProject" v-model:open="artifactDetailsOpen" :version="selectedArtifact" :mod-id="operationProject.modId" @repair="record => { artifactDetailsOpen = false; $router.push({ path: `/mods/${record.modId}/chat`, query: { repair: record.recordId } }) }" />
    <InstallDialog v-if="selectedArtifact && operationProject" :key="`${operationProject.modId}:${selectedArtifact.versionId}`" v-model:open="artifactInstalling" :mod-id="operationProject.modId" :version-id="selectedArtifact.versionId" :previous-target="operationProject.lastDeploymentDirectory" />
    <DeleteVersionDialog v-model:open="artifactDeleting" :version-id="artifactVersionId" :installed="installedVersionIds.has(artifactVersionId)" :busy="artifactBusy" @confirm="deleteArtifact" />
  </section>
</template>
