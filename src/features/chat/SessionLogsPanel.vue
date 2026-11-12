<script setup lang="ts">
import { FileText, RefreshCw } from '@lucide/vue'
import { Button } from '@/components/ui/button'
import { ScrollArea } from '@/components/ui/scroll-area'
import { Tooltip, TooltipContent, TooltipProvider, TooltipTrigger } from '@/components/ui/tooltip'
import { useSessionStore } from '@/features/sessions/useSessionStore'
import { useSessionLogs } from './useSessionLogs'

const props = defineProps<{ active: boolean }>()
const session = useSessionStore()
const { entries: runtimeLogs, error: logError, refresh: refreshLogs, openTask: openLogs } = useSessionLogs(() => props.active)
</script>

<template>
  <div class="flex min-h-0 flex-1 flex-col overflow-hidden">
    <div class="flex shrink-0 items-center justify-between gap-3 border-b px-3 py-2">
      <div class="flex min-w-0 flex-col gap-1">
        <p class="text-sm font-medium">
          会话日志
        </p>
        <p class="text-xs text-muted-foreground">
          最近 200 条 · 每 3 秒自动刷新
        </p>
      </div>
      <TooltipProvider :delay-duration="150">
        <div class="flex shrink-0 items-center gap-1">
          <Tooltip v-if="session.task">
            <TooltipTrigger as-child>
              <Button size="icon-sm" variant="ghost" aria-label="打开任务完整日志" @click="openLogs">
                <FileText aria-hidden="true" />
              </Button>
            </TooltipTrigger>
            <TooltipContent>打开任务完整日志</TooltipContent>
          </Tooltip>
          <Tooltip>
            <TooltipTrigger as-child>
              <Button size="icon-sm" variant="ghost" aria-label="刷新日志" @click="refreshLogs">
                <RefreshCw aria-hidden="true" />
              </Button>
            </TooltipTrigger>
            <TooltipContent>刷新日志</TooltipContent>
          </Tooltip>
        </div>
      </TooltipProvider>
    </div>
    <ScrollArea class="min-h-0 flex-1">
      <p v-if="logError" role="alert" class="p-3 text-xs text-destructive">
        {{ logError }}
      </p><p v-if="!runtimeLogs.length" class="p-3 text-xs text-muted-foreground">
        暂无已保存的运行日志。更新前的运行摘要不会补录，既有命令日志仍可通过上方按钮查看。
      </p><div v-for="(entry, index) in runtimeLogs" :key="`${entry.timestamp}-${index}`" class="border-b px-3 py-2 text-[11px] leading-5">
        <p class="text-muted-foreground">
          {{ new Date(entry.timestamp).toLocaleString() }} · {{ entry.taskId ? `任务 ${entry.taskId.slice(0, 8)}` : '连接' }}
        </p><pre class="mt-1 whitespace-pre-wrap break-all">{{ entry.message }}</pre>
      </div><template v-if="session.diagnostics.length">
        <p class="px-3 pt-4 text-xs font-medium">
          当前连接诊断
        </p><pre v-for="(line, index) in session.diagnostics" :key="index" class="border-b px-3 py-2 font-mono text-[11px] leading-5 whitespace-pre-wrap break-all">{{ line }}</pre>
      </template>
    </ScrollArea>
  </div>
</template>
