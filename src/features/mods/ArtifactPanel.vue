<script setup lang="ts">
import type { ArtifactVersion } from "@/contracts/mod";
import { Dialog, DialogContent, DialogDescription, DialogHeader, DialogTitle } from "@/components/ui/dialog";
defineProps<{ version: ArtifactVersion }>();
const open = defineModel<boolean>("open", { default: false });
function fileSize(bytes: number): string {
  if (bytes < 1024) { return `${bytes} B`; }
  if (bytes < 1024 * 1024) { return `${(bytes / 1024).toFixed(1)} KB`; }
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
}
</script>

<template>
  <Dialog v-model:open="open">
    <DialogContent class="flex max-h-[calc(100dvh-3rem)] flex-col gap-3 overflow-hidden p-5 sm:max-w-2xl">
      <DialogHeader class="shrink-0 pr-6"><DialogTitle>版本详情 {{ version.versionId.slice(0, 8) }}</DialogTitle><DialogDescription>版本信息及交付文件清单。</DialogDescription></DialogHeader>
      <div class="min-h-0 overflow-y-auto">
        <dl class="grid grid-cols-[auto_minmax(0,1fr)] gap-x-4 gap-y-2 rounded-md bg-muted/40 p-3 text-xs">
          <dt class="text-muted-foreground">版本号</dt><dd class="break-all font-mono select-text">{{ version.versionId }}</dd>
          <dt class="text-muted-foreground">任务 ID</dt><dd class="break-all font-mono select-text">{{ version.taskId }}</dd>
          <dt class="text-muted-foreground">生成时间</dt><dd>{{ new Date(version.createdAt).toLocaleString() }}</dd>
          <dt class="text-muted-foreground">文件数量</dt><dd>{{ version.files.length }} 个文件</dd>
        </dl>
        <div class="mt-4 flex justify-between border-b pb-2 text-xs text-muted-foreground"><span>文件路径</span><span>大小</span></div>
        <ul class="divide-y text-xs"><li v-for="file in version.files" :key="file.path" class="flex justify-between gap-4 py-2.5"><span class="min-w-0 break-all font-mono select-text">{{ file.path }}</span><span class="shrink-0 text-muted-foreground" :title="`${file.size} B`">{{ fileSize(file.size) }}</span></li></ul>
        <p class="mt-3 text-xs leading-5 text-muted-foreground">版本文件校验通过才可安装。交付成功不代表游戏运行效果已验证。</p>
      </div>
    </DialogContent>
  </Dialog>
</template>
