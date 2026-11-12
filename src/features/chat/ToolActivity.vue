<script setup lang="ts">
import type { DeepReadonly } from 'vue'
import type { ToolRun } from '@/contracts/session'
import { ChevronDown, FileText, Folder, Search, Terminal } from '@lucide/vue'
import { computed, defineAsyncComponent, shallowRef } from 'vue'
import { Button } from '@/components/ui/button'
import { Collapsible, CollapsibleContent, CollapsibleTrigger } from '@/components/ui/collapsible'

const props = defineProps<{ tool: DeepReadonly<ToolRun> }>()
const MemorySaveNotice = defineAsyncComponent(() => import('@/features/knowledge/MemorySaveNotice.vue'))
const open = shallowRef(false)
const icon = computed(() => {
  const icons: Record<string, typeof Terminal> = { read: FileText, write: FileText, edit: FileText, grep: Search, find: Search, ls: Folder }
  return icons[props.tool.name] || Terminal
})
const summary = computed(() => {
  const verbs: Record<string, string> = { read: '读取文件', write: '写入源码', edit: '修改源码', grep: '搜索代码', find: '查找文件', ls: '查看目录', run_command: '运行命令', prepare_delivery: '整理交付文件' }
  const verb = verbs[props.tool.name] || props.tool.name
  if (props.tool.status === 'failed') {
    return `${verb}失败`
  }
  const prefixes = { waiting: '等待', running: '正在', succeeded: '已', cancelled: '已取消' }
  return `${prefixes[props.tool.status]}${verb}`
})
const detail = computed(() => {
  const input = props.tool.args && typeof props.tool.args === 'object' ? props.tool.args as Record<string, unknown> : {}
  return typeof input.path === 'string' ? input.path : typeof input.program === 'string' ? `${input.program} ${Array.isArray(input.args) ? input.args.join(' ') : ''}`.trim() : ''
})
const args = computed(() => JSON.stringify(props.tool.args, null, 2))
const output = computed(() => {
  const text = typeof props.tool.output === 'string' ? props.tool.output : JSON.stringify(props.tool.output, null, 2) ?? ''
  return text.length > 8_000 ? `${text.slice(0, 8_000)}\n…输出已截断，完整记录保存在任务日志中。` : text
})
</script>

<template>
  <Collapsible v-model:open="open" class="my-2 text-xs text-muted-foreground">
    <CollapsibleTrigger as-child>
      <Button type="button" variant="ghost" size="sm" class="h-auto max-w-full justify-start gap-2 px-0 py-1 text-left text-sm font-normal hover:bg-transparent has-[>svg]:px-0" :class="tool.status === 'failed' ? 'text-destructive' : open ? 'text-foreground' : ''">
        <component :is="icon" class="size-3.5 shrink-0" aria-hidden="true" />
        <span class="min-w-0 truncate" :class="tool.status === 'running' && 'tool-shimmer'">{{ summary }}</span>
        <ChevronDown class="size-3 shrink-0 transition-transform" :class="open && 'rotate-180'" aria-hidden="true" />
      </Button>
    </CollapsibleTrigger><CollapsibleContent>
      <div class="mt-1 space-y-2 pl-5.5">
        <p v-if="detail" class="truncate text-sm" :title="detail">
          {{ detail }}
        </p>
        <pre v-if="args" class="max-h-72 overflow-auto whitespace-pre-wrap break-all">{{ args }}</pre>
        <pre v-if="output" class="max-h-72 overflow-auto whitespace-pre-wrap break-all leading-5">{{ output }}</pre>
      </div>
    </CollapsibleContent>
  </Collapsible>
  <MemorySaveNotice v-if="tool.status === 'succeeded' && tool.memorySave" :result="tool.memorySave" />
  <p v-if="tool.status === 'succeeded' && tool.projectMemoryUpdate?.outcome === 'saved'" role="status" class="my-2 text-sm text-muted-foreground">
    项目记忆已更新，可在项目上下文的“记忆”中查看，在聊天中更正。
  </p>
</template>

<style scoped>
.tool-shimmer {
  background-image: linear-gradient(
    110deg,
    var(--muted-foreground) 35%,
    var(--border) 50%,
    var(--muted-foreground) 65%
  );
  background-size: 250% 100%;
  background-clip: text;
  -webkit-text-fill-color: transparent;
  animation: tool-shimmer 2s linear infinite;
}

@keyframes tool-shimmer {
  from {
    background-position: 150% 0;
  }
  to {
    background-position: -50% 0;
  }
}

@media (prefers-reduced-motion: reduce) {
  .tool-shimmer {
    background-image: none;
    -webkit-text-fill-color: currentColor;
    animation: none;
  }
}
</style>
