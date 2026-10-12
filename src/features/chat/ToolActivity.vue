<script setup lang="ts">
import type { ToolRun } from '@/contracts/session'
import { Check, LoaderCircle, Terminal, X } from '@lucide/vue'
import { computed } from 'vue'
import { Button } from '@/components/ui/button'
import { Collapsible, CollapsibleContent, CollapsibleTrigger } from '@/components/ui/collapsible'

const props = defineProps<{ tool: ToolRun }>()
const labels = { waiting: '等待执行', running: '执行中', succeeded: '已完成', failed: '失败', cancelled: '已取消' }
const summary = computed(() => {
  const input = props.tool.args && typeof props.tool.args === 'object' ? props.tool.args as Record<string, unknown> : {}
  const verbs: Record<string, string> = { read: '读取文件', write: '写入源码', edit: '修改源码', grep: '搜索代码', find: '查找文件', ls: '查看目录', run_command: '运行命令', prepare_delivery: '整理交付文件' }
  const detail = typeof input.path === 'string' ? input.path : typeof input.program === 'string' ? `${input.program} ${Array.isArray(input.args) ? input.args.join(' ') : ''}` : ''
  return `${verbs[props.tool.name] || props.tool.name}${detail ? ` ${detail}` : ''}`
})
const args = computed(() => JSON.stringify(props.tool.args, null, 2))
const output = computed(() => {
  const text = typeof props.tool.output === 'string' ? props.tool.output : JSON.stringify(props.tool.output, null, 2) ?? ''
  return text.length > 8_000 ? `${text.slice(0, 8_000)}\n…输出已截断，完整记录保存在任务日志中。` : text
})
</script>

<template>
  <Collapsible class="my-2 text-xs">
    <CollapsibleTrigger as-child>
      <Button type="button" variant="ghost" size="sm" class="h-auto w-full justify-start py-1">
        <LoaderCircle v-if="tool.status === 'running'" class="size-3.5 shrink-0 animate-spin" /><X v-else-if="tool.status === 'failed'" class="size-3.5 shrink-0 text-destructive" /><Check v-else-if="tool.status === 'succeeded'" class="size-3.5 shrink-0" /><Terminal v-else class="size-3.5 shrink-0" /><span class="min-w-0 truncate" :title="summary">{{ summary }}</span><span class="ml-auto shrink-0" :class="tool.status === 'failed' ? 'text-destructive' : ''">{{ labels[tool.status] }}</span>
      </Button>
    </CollapsibleTrigger><CollapsibleContent>
      <div class="mt-2 flex flex-col gap-3 rounded-md border bg-muted/30 p-3">
        <pre class="overflow-auto whitespace-pre-wrap break-all">{{ args }}</pre><pre v-if="output" class="max-h-72 overflow-auto whitespace-pre-wrap break-all">{{ output }}</pre>
      </div>
    </CollapsibleContent>
  </Collapsible>
</template>
