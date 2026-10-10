<script setup lang="ts">
import { computed } from "vue";
import { LoaderCircle } from "@lucide/vue";
import { Tooltip, TooltipContent, TooltipProvider, TooltipTrigger } from "@/components/ui/tooltip";
import type { ContextUsage } from "@/contracts/session";
const props = defineProps<{ usage?: ContextUsage; compacting: boolean }>();
const known = computed(() => props.usage?.tokens != null && Number.isFinite(props.usage.tokens));
const percent = computed(() => known.value ? Math.min(100, Math.max(0, props.usage!.tokens! / props.usage!.contextWindow * 100)) : undefined);
const usedPercent = computed(() => percent.value !== undefined ? Math.round(percent.value) : undefined);
/** 当前窗口占用以紧凑单位显示；手动压缩由输入框功能菜单管理。 */
function count(value: number): string { return value >= 1000 ? `${Math.round(value / 1000)}k` : `${Math.round(value)}`; }
</script>
<template>
  <TooltipProvider :delay-duration="150"><Tooltip><TooltipTrigger as-child>
    <span tabindex="0" class="flex items-center rounded-md px-1.5 py-1 text-muted-foreground focus-visible:outline focus-visible:outline-ring" aria-label="上下文用量">
      <LoaderCircle v-if="compacting" class="size-3.5 animate-spin motion-reduce:animate-none" />
      <svg v-else viewBox="0 0 20 20" class="size-3.5 -rotate-90" fill="none" stroke-width="3" aria-hidden="true"><circle cx="10" cy="10" r="7" class="stroke-muted-foreground/25" /><circle v-if="percent !== undefined" cx="10" cy="10" r="7" class="stroke-muted-foreground" pathLength="100" :stroke-dasharray="`${percent} 100`" stroke-linecap="round" /></svg>
    </span>
  </TooltipTrigger><TooltipContent side="top" class="rounded-xl px-4 py-2.5 text-center text-xs leading-5"><p class="opacity-60">{{ usage?.historical ? '上次上下文统计：' : '背景信息窗口：' }}</p><p v-if="usage?.sampledAt" class="opacity-60">{{ new Date(usage.sampledAt).toLocaleString() }}{{ usage.estimated ? ' · 估算' : '' }}</p><template v-if="usedPercent !== undefined && usage"><p>{{ usedPercent }}% 已用（剩余 {{ 100 - usedPercent }}%）</p><p>已用 {{ count(usage.tokens!) }} 标记，共 {{ count(usage.contextWindow) }}</p></template><p v-else>{{ compacting ? '正在压缩上下文' : '暂无有效用量数据' }}</p></TooltipContent></Tooltip></TooltipProvider>
</template>
