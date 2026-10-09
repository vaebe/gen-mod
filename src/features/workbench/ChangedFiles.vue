<script setup lang="ts">
import type { FileChange } from "@/contracts/changes";
defineProps<{ changes: FileChange[]; selected?: string }>();
defineEmits<{ select: [path: string] }>();
const labels = { added: "新增", modified: "修改", deleted: "删除" };
</script>
<template>
  <div><p v-if="!changes.length" class="px-3 py-6 text-xs text-muted-foreground">此任务尚无源码变更。</p><button v-for="change in changes" :key="change.path" class="flex w-full flex-col gap-1 border-b px-3 py-3 text-left hover:bg-accent" :class="selected === change.path ? 'bg-accent' : ''" @click="$emit('select', change.path)"><span class="break-all font-mono text-xs">{{ change.path }}</span><span class="flex gap-2 text-[11px] text-muted-foreground"><span>{{ labels[change.kind] }}</span><template v-if="change.textAvailable"><span class="text-green-700">+{{ change.added }}</span><span class="text-red-700">−{{ change.removed }}</span></template><span v-else>二进制或大文件</span></span></button></div>
</template>
