<script setup lang="ts">
import type { FileChange } from '@/contracts/changes'
import { Button } from '@/components/ui/button'

defineProps<{ changes: FileChange[], selected?: string }>()
defineEmits<{ select: [path: string] }>()
const labels = { added: '新增', modified: '修改', deleted: '删除' }
</script>

<template>
  <div>
    <p v-if="!changes.length" class="px-3 py-6 text-xs text-muted-foreground">
      此任务尚无源码变更。
    </p><Button v-for="change in changes" :key="change.path" type="button" :variant="selected === change.path ? 'secondary' : 'ghost'" class="h-auto w-full flex-col items-start gap-1 whitespace-normal rounded-none border-b px-3 py-3 text-left" @click="$emit('select', change.path)">
      <span class="break-all font-mono text-xs">{{ change.path }}</span><span class="flex gap-2 text-[11px] text-muted-foreground"><span>{{ labels[change.kind] }}</span><template v-if="change.textAvailable"><span class="text-green-700 dark:text-green-400">+{{ change.added }}</span><span class="text-red-700 dark:text-red-400">−{{ change.removed }}</span></template><span v-else>二进制或大文件</span></span>
    </Button>
  </div>
</template>
