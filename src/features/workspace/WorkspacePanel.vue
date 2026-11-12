<script setup lang="ts">
import { shallowRef } from 'vue'
import { ScrollArea } from '@/components/ui/scroll-area'
import { ToggleGroup, ToggleGroupItem } from '@/components/ui/toggle-group'
import ChangedFiles from './ChangedFiles.vue'
import FileDiff from './FileDiff.vue'
import SourceBrowser from './SourceBrowser.vue'
import { sourceRevision, useTaskChanges } from './useTaskChanges'

const props = defineProps<{ modId: string, taskId?: string, readOnly?: boolean, active: boolean }>()
const fileMode = shallowRef('all')
const { changes, operations, diff, selected, error, select } = useTaskChanges(() => props.modId, () => props.taskId)
</script>

<template>
  <ToggleGroup type="single" :model-value="fileMode" size="sm" class="shrink-0 border-b p-2" aria-label="文件查看范围" @update:model-value="($event === 'all' || $event === 'changes') && (fileMode = $event)">
    <ToggleGroupItem value="all" class="flex-1">
      全部文件
    </ToggleGroupItem>
    <ToggleGroupItem value="changes" class="flex-1">
      当前变更
    </ToggleGroupItem>
  </ToggleGroup>
  <SourceBrowser v-show="fileMode === 'all'" :mod-id="modId" :revision="sourceRevision(modId)" :read-only="readOnly" :active="active && fileMode === 'all'" />
  <ScrollArea v-if="fileMode === 'changes'" class="min-h-0 flex-1">
    <div v-if="taskId" class="space-y-2 border-b p-3 text-xs">
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
    </div><p v-if="!taskId" class="p-3 text-xs leading-6 text-muted-foreground">
      任务开始后显示源码变更与差异。
    </p><ChangedFiles v-else :changes="changes" :selected="selected" @select="select" /><FileDiff v-if="diff" :diff="diff" :mod-id="modId" /><p v-if="error" role="alert" class="p-3 text-xs text-destructive">
      {{ error }}
    </p>
  </ScrollArea>
</template>
