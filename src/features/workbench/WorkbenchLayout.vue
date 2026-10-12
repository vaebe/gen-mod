<script setup lang="ts">
import { PanelRight } from '@lucide/vue'
import { useElementSize, useMediaQuery } from '@vueuse/core'
import { computed, onMounted, shallowRef, useTemplateRef } from 'vue'
import { useRoute } from 'vue-router'
import { Button } from '@/components/ui/button'
import { ResizableHandle, ResizablePanel, ResizablePanelGroup } from '@/components/ui/resizable'
import { Sheet, SheetContent, SheetDescription, SheetHeader, SheetTitle } from '@/components/ui/sheet'
import { SidebarInset, SidebarProvider, SidebarTrigger } from '@/components/ui/sidebar'
import ExtensionDialog from '@/features/chat/ExtensionDialog.vue'
import { useProjectsStore } from '@/stores/projects'
import { useSessionStore } from '@/stores/session'
import ContextPanel from './ContextPanel.vue'
import ProjectSidebar from './ProjectSidebar.vue'

const route = useRoute()
const projects = useProjectsStore()
const session = useSessionStore()
const contextOpen = shallowRef(false)
const wide = useMediaQuery('(min-width: 1100px)')
const contextContainer = useTemplateRef<HTMLDivElement>('contextContainer')
const { width: contextContainerWidth } = useElementSize(contextContainer)
const contextMaxSize = computed(() => Math.max(500, contextContainerWidth.value * 0.8))
/** 后台会话持续存在，面板只属于当前路由已选中的会话。 */
const hasContext = computed(() => {
  const modId = String(route.params.modId ?? '')
  if (route.path !== `/mods/${modId}/chat` || route.query.new || session.modId !== modId || !session.sessionId || session.switching) {
    return false
  }
  const project = projects.projects.find(item => item.modId === modId)
  const selectedId = route.query.session ?? project?.sessions[project.sessions.length - 1]?.sessionId
  return selectedId === session.sessionId && !!session.stateFor(modId, session.sessionId)?.initialized
})
// 隐藏不清除展开偏好，回到有效会话时恢复；抽屉关闭仍更新同一个开关。
const contextVisible = computed({
  get: () => hasContext.value && contextOpen.value,
  set: (value: boolean) => { contextOpen.value = value },
})
onMounted(() => {
  void projects.refresh()
})
</script>

<template>
  <SidebarProvider class="h-full min-h-0" :style="{ '--sidebar-width': '15rem' }">
    <ProjectSidebar />
    <SidebarInset class="min-h-0 overflow-hidden [@media(max-width:768px)]:pl-12 md:peer-data-[state=collapsed]:pl-12">
      <SidebarTrigger class="absolute left-3 top-3 z-10 hidden [@media(max-width:768px)]:flex group-has-[[data-slot=sidebar][data-state=collapsed]]/sidebar-wrapper:flex" aria-label="展开项目导航" title="展开项目导航" />
      <div ref="contextContainer" class="flex min-h-0 flex-1">
        <ResizablePanelGroup direction="horizontal" class="min-h-0 flex-1">
          <ResizablePanel :order="0" :default-size="75" :min-size="20">
            <div class="relative h-full min-h-0">
              <Button v-if="hasContext" variant="ghost" size="icon" class="absolute right-3 top-3 z-10 size-7 bg-background/90 shadow-sm backdrop-blur-sm" :aria-label="contextVisible ? '收起项目上下文' : '展开项目上下文'" :title="contextVisible ? '收起项目上下文' : '展开项目上下文'" :aria-expanded="contextVisible" @click="contextOpen = !contextOpen">
                <PanelRight class="size-4" />
              </Button>
              <RouterView />
            </div>
          </ResizablePanel>
          <template v-if="wide && contextVisible">
            <ResizableHandle with-handle />
            <ResizablePanel :order="1" size-unit="px" :default-size="500" :min-size="300" :max-size="contextMaxSize">
              <ContextPanel :key="session.selectedKey" />
            </ResizablePanel>
          </template>
        </ResizablePanelGroup>
      </div>
    </SidebarInset>
    <ExtensionDialog />
    <Sheet v-if="!wide && hasContext" v-model:open="contextVisible">
      <SheetContent class="flex w-[500px] min-w-[300px] flex-col p-0 sm:max-w-none">
        <SheetHeader class="p-4">
          <SheetTitle>上下文</SheetTitle><SheetDescription>文件、命令日志与交付产物</SheetDescription>
        </SheetHeader><ContextPanel :key="session.selectedKey" />
      </SheetContent>
    </Sheet>
  </SidebarProvider>
</template>
