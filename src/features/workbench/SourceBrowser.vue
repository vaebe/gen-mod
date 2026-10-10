<script setup lang="ts">
import { shallowRef, watch } from "vue";
import { Maximize2, RefreshCw } from "@lucide/vue";
import { Button } from "@/components/ui/button";
import { Dialog, DialogContent, DialogDescription, DialogHeader, DialogTitle } from "@/components/ui/dialog";
import { ResizableHandle, ResizablePanel, ResizablePanelGroup } from "@/components/ui/resizable";
import SourceFileTree from "./SourceFileTree.vue";
import SourceFilePreview from "./SourceFilePreview.vue";
import { useSourceBrowser } from "./useSourceBrowser";

const props = defineProps<{ modId: string; revision: number; active: boolean }>();
const enlarged = shallowRef(false);
const { rows, selected, preview, previewLoading, previewError, rootLoading, rootError, toggle, select, refresh, loadDirectory } = useSourceBrowser(props);
watch(() => props.modId, () => { enlarged.value = false; });

/** 根目录重试需一并恢复已展开分支；子目录只重试当前层。 */
function retry(path: string): void {
  if (path) { void loadDirectory(path); }
  else { void refresh(); }
}
</script>

<template>
  <div class="flex min-h-0 flex-1 flex-col">
    <p v-if="!modId" class="p-4 text-xs text-muted-foreground">选择项目后浏览源码。</p>
    <template v-else>
      <div class="flex shrink-0 items-center justify-between border-b px-3 py-2"><span class="font-mono text-[11px] font-medium text-muted-foreground">source</span><div class="flex items-center gap-1"><Button size="icon" variant="ghost" class="size-7" aria-label="刷新源码文件" title="刷新" :disabled="rootLoading" @click="refresh"><RefreshCw class="size-3.5" :class="rootLoading && 'animate-spin'" /></Button><Button size="icon" variant="ghost" class="size-7" aria-label="放大源码浏览器" title="放大预览" @click="enlarged = true"><Maximize2 class="size-3.5" /></Button></div></div>
      <ResizablePanelGroup v-if="!enlarged" direction="vertical" class="min-h-0 flex-1">
        <ResizablePanel :default-size="40" :min-size="20"><SourceFileTree class="h-full" :rows="rows" :selected="selected" :loading="rootLoading" :error="rootError" @select="select" @toggle="toggle" @retry="retry" /></ResizablePanel>
        <ResizableHandle with-handle />
        <ResizablePanel :default-size="60" :min-size="25"><SourceFilePreview class="h-full" :mod-id="modId" :path="selected" :preview="preview" :loading="previewLoading" :error="previewError" /></ResizablePanel>
      </ResizablePanelGroup>
      <Dialog v-model:open="enlarged">
        <DialogContent class="flex h-[85dvh] w-[calc(100vw-2rem)] max-w-none flex-col gap-0 overflow-hidden p-0 sm:max-w-[1400px]">
          <DialogHeader class="shrink-0 border-b px-5 py-4 pr-12"><DialogTitle>项目源码</DialogTitle><DialogDescription>展开文件夹，选择文件查看当前源码。预览为只读。</DialogDescription></DialogHeader>
          <div class="flex shrink-0 items-center justify-between border-b px-4 py-2"><span class="font-mono text-xs text-muted-foreground">source</span><Button size="sm" variant="ghost" :disabled="rootLoading" @click="refresh"><RefreshCw class="size-3.5" :class="rootLoading && 'animate-spin'" />刷新</Button></div>
          <ResizablePanelGroup direction="horizontal" class="min-h-0 flex-1">
            <ResizablePanel :default-size="25" :min-size="15" :max-size="50"><SourceFileTree class="h-full" :rows="rows" :selected="selected" :loading="rootLoading" :error="rootError" @select="select" @toggle="toggle" @retry="retry" /></ResizablePanel>
            <ResizableHandle with-handle />
            <ResizablePanel :default-size="75" :min-size="40"><SourceFilePreview class="h-full" :mod-id="modId" :path="selected" :preview="preview" :loading="previewLoading" :error="previewError" /></ResizablePanel>
          </ResizablePanelGroup>
        </DialogContent>
      </Dialog>
    </template>
  </div>
</template>
