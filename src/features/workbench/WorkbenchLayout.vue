<script setup lang="ts">
import { onMounted, shallowRef } from "vue";
import { useRoute } from "vue-router";
import { useMediaQuery } from "@vueuse/core";
import { PanelRight } from "@lucide/vue";
import { Button } from "@/components/ui/button";
import { SidebarInset, SidebarProvider, SidebarTrigger } from "@/components/ui/sidebar";
import { ResizableHandle, ResizablePanel, ResizablePanelGroup } from "@/components/ui/resizable";
import { Sheet, SheetContent, SheetDescription, SheetHeader, SheetTitle } from "@/components/ui/sheet";
import ProjectSidebar from "./ProjectSidebar.vue";
import ContextPanel from "./ContextPanel.vue";
import NewModDialog from "@/features/mods/NewModDialog.vue";
import { useProjectsStore } from "@/stores/projects";
import ExtensionDialog from "@/features/chat/ExtensionDialog.vue";
const route = useRoute();
const projects = useProjectsStore();
const createOpen = shallowRef(false);
const contextOpen = shallowRef(false);
const wide = useMediaQuery("(min-width: 1100px)");
onMounted(() => { void projects.refresh(); });
</script>
<template>
  <SidebarProvider class="h-full min-h-0" :style="{ '--sidebar-width': '15rem' }">
    <ProjectSidebar @create="createOpen = true" />
    <SidebarInset class="min-h-0 overflow-hidden">
      <header class="flex h-13 shrink-0 items-center gap-3 border-b px-4"><SidebarTrigger /><span class="text-sm font-medium">{{ route.meta.title }}</span><div class="ml-auto flex items-center gap-3"><span class="text-xs text-muted-foreground">本地工作区</span><Button variant="ghost" size="icon" aria-label="切换上下文面板" @click="contextOpen = !contextOpen"><PanelRight class="size-4" /></Button></div></header>
      <ResizablePanelGroup direction="horizontal" class="min-h-0 flex-1"><ResizablePanel :order="0" :default-size="75" :min-size="45"><main class="h-full min-h-0"><RouterView /></main></ResizablePanel><template v-if="wide && contextOpen"><ResizableHandle with-handle /><ResizablePanel :order="1" :default-size="25" :min-size="20" :max-size="40"><ContextPanel /></ResizablePanel></template></ResizablePanelGroup>
    </SidebarInset>
    <NewModDialog v-model:open="createOpen" />
    <ExtensionDialog />
    <Sheet v-if="!wide" v-model:open="contextOpen"><SheetContent class="flex flex-col p-0"><SheetHeader class="p-4"><SheetTitle>上下文</SheetTitle><SheetDescription>文件、命令日志与交付产物</SheetDescription></SheetHeader><ContextPanel /></SheetContent></Sheet>
  </SidebarProvider>
</template>
