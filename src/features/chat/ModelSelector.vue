<script setup lang="ts">
import type { ModelSelection, ProviderConfiguration, SessionModelState } from '@/contracts/config'
import type { ThinkingLevel } from '@/contracts/session'
import { Check, ChevronDown, RotateCcw, Zap } from '@lucide/vue'
import { DropdownMenuContent, DropdownMenuItem, DropdownMenuPortal, DropdownMenuRoot, DropdownMenuTrigger, PopoverContent, PopoverPortal, PopoverRoot, PopoverTrigger } from 'reka-ui'
import { computed, shallowRef, watch } from 'vue'
import { Button } from '@/components/ui/button'

const props = defineProps<{ providers: ProviderConfiguration[], modelState?: SessionModelState, thinkingLevel: ThinkingLevel, thinkingLevels: ThinkingLevel[], disabled: boolean, thinkingDisabled: boolean }>()
const emit = defineEmits<{ select: [selection: ModelSelection], thinking: [level: ThinkingLevel] }>()
const modelOpen = shallowRef(false)
const thinkingOpen = shallowRef(false)
const previewIndex = shallowRef(0)
const labels: Record<ThinkingLevel, string> = { off: '关闭', minimal: '最低', low: '轻度', medium: '中等', high: '高', xhigh: '更高', max: '最高' }
const name = computed(() => props.modelState?.definition?.name || props.modelState?.selection.modelId || '未配置模型')
const count = computed(() => props.providers.reduce((total, provider) => total + provider.models.length, 0))
const currentExists = computed(() => props.providers.some(provider => provider.id === props.modelState?.selection.providerId && provider.models.some(model => model.id === props.modelState?.selection.modelId)))
const canChoose = computed(() => count.value > 1 || (count.value === 1 && !currentExists.value))
const previewLevel = computed(() => props.thinkingLevels[previewIndex.value] ?? props.thinkingLevel)
const extendedThinking = computed(() => previewLevel.value === 'xhigh' || previewLevel.value === 'max')
const progressWidth = computed(() => {
  const progress = previewIndex.value / Math.max(1, props.thinkingLevels.length - 1)
  return `calc(${progress * 100}% + ${18 - 36 * progress}px)`
})
const defaultLevel = computed(() => {
  const order: ThinkingLevel[] = ['off', 'minimal', 'low', 'medium', 'high', 'xhigh', 'max']
  return props.thinkingLevels.reduce<ThinkingLevel | undefined>((closest, level) => !closest || Math.abs(order.indexOf(level) - 3) < Math.abs(order.indexOf(closest) - 3) ? level : closest, undefined)
})
watch(() => [props.thinkingLevel, props.thinkingLevels, props.thinkingDisabled], () => {
  // 保存期间保留用户选择的预览，结束后再核对实际档位或回退失败选择。
  if (!props.thinkingDisabled) {
    previewIndex.value = Math.max(0, props.thinkingLevels.indexOf(props.thinkingLevel))
  }
}, { immediate: true })
watch(() => props.modelState?.selection, () => {
  modelOpen.value = false
  thinkingOpen.value = false
})
watch(() => props.disabled, (disabled) => {
  if (disabled) {
    modelOpen.value = false
  }
})
function choose(providerId: string, modelId: string): void {
  if (!props.disabled) {
    emit('select', { providerId, modelId })
  }
}
function changeThinking(level?: ThinkingLevel): void {
  if (level && !props.thinkingDisabled && props.thinkingLevels.includes(level) && level !== props.thinkingLevel) {
    emit('thinking', level)
  }
}
</script>

<template>
  <div class="flex min-w-0 items-center rounded-full bg-muted/60">
    <DropdownMenuRoot v-if="canChoose" v-model:open="modelOpen">
      <DropdownMenuTrigger as-child>
        <Button variant="ghost" size="sm" class="h-8 min-w-0 gap-1.5 rounded-full px-3 text-xs" :disabled="disabled" aria-label="选择模型" :title="name">
          <span class="max-w-40 truncate">{{ name }}</span><ChevronDown class="size-3.5 shrink-0 text-muted-foreground" />
        </Button>
      </DropdownMenuTrigger>
      <DropdownMenuPortal>
        <DropdownMenuContent side="top" align="end" :side-offset="8" class="z-50 max-h-80 w-72 max-w-[calc(100vw-2rem)] overflow-y-auto rounded-2xl border bg-popover p-1.5 text-popover-foreground shadow-lg">
          <p class="px-3 py-2 text-xs text-muted-foreground">
            选择模型
          </p>
          <template v-for="provider in providers" :key="provider.id">
            <p v-if="provider.models.length" class="px-3 pt-2 pb-1 text-xs text-muted-foreground">
              {{ provider.name }}
            </p>
            <DropdownMenuItem v-for="model in provider.models" :key="model.id" :disabled="disabled || !provider.keyConfigured" :title="provider.keyConfigured ? `${provider.name} · ${model.id}` : '请先在设置页填写此服务商的 API Key'" class="flex cursor-pointer items-center justify-between gap-4 rounded-xl px-3 py-2.5 text-sm outline-none data-[disabled]:cursor-default data-[disabled]:opacity-40 data-[highlighted]:bg-muted" @select="choose(provider.id, model.id)">
              <span class="min-w-0 truncate">{{ model.name }}</span><Check v-if="modelState?.selection.providerId === provider.id && modelState.selection.modelId === model.id" class="size-4 shrink-0 text-muted-foreground" />
            </DropdownMenuItem>
          </template>
        </DropdownMenuContent>
      </DropdownMenuPortal>
    </DropdownMenuRoot>
    <span v-else class="max-w-44 truncate px-3 py-1.5 text-xs" :title="name">{{ name }}</span>
    <PopoverRoot v-if="modelState?.definition?.reasoning && thinkingLevels.length" v-model:open="thinkingOpen">
      <PopoverTrigger as-child>
        <Button variant="ghost" size="sm" class="h-8 gap-1 rounded-full pr-3 pl-2 text-xs text-muted-foreground" :disabled="thinkingDisabled" aria-label="选择思考强度">
          <span>{{ labels[thinkingLevel] }}</span><ChevronDown class="size-3.5" />
        </Button>
      </PopoverTrigger>
      <PopoverPortal>
        <PopoverContent side="top" align="end" :side-offset="8" class="z-50 w-80 max-w-[calc(100vw-2rem)] rounded-3xl border bg-popover p-4 text-popover-foreground shadow-lg">
          <div class="thinking-panel" :class="{ 'thinking-panel-extended': extendedThinking }">
            <div class="flex items-start justify-between gap-3">
              <Zap class="mt-1 size-4 text-muted-foreground" />
              <div class="min-w-0 text-center">
                <p class="thinking-title text-lg font-semibold">
                  {{ labels[previewLevel] }}
                </p>
                <p class="mt-0.5 truncate text-xs text-muted-foreground" :title="name">
                  {{ name }}
                </p>
              </div>
              <Button variant="ghost" size="icon" class="size-6 text-muted-foreground" :disabled="thinkingDisabled || thinkingLevel === defaultLevel" aria-label="恢复默认思考强度" title="恢复默认思考强度" @click="changeThinking(defaultLevel)">
                <RotateCcw class="size-4" />
              </Button>
            </div>
            <div v-if="thinkingLevels.length > 1" class="thinking-track relative mt-5 h-9 rounded-full bg-muted">
              <div v-if="previewIndex > 0" class="thinking-fill pointer-events-none absolute inset-y-0 left-0 rounded-full" :style="{ width: progressWidth }" />
              <div class="pointer-events-none absolute inset-0 flex items-center justify-between px-[18px]">
                <span v-for="(level, index) in thinkingLevels" :key="level" class="size-1.5 rounded-full" :class="index < previewIndex ? 'bg-white/45' : 'bg-muted-foreground/40'" />
              </div>
              <input type="range" min="0" :max="thinkingLevels.length - 1" step="1" :value="previewIndex" :disabled="thinkingDisabled" class="thinking-slider relative h-full w-full cursor-pointer disabled:cursor-default" aria-label="模型思考强度" :aria-valuetext="labels[previewLevel]" @input="previewIndex = Number(($event.target as HTMLInputElement).value)" @change="changeThinking(previewLevel)">
            </div>
            <p v-else class="mt-4 text-center text-xs text-muted-foreground">
              此模型仅支持 {{ labels[thinkingLevel] }}
            </p>
          </div>
        </PopoverContent>
      </PopoverPortal>
    </PopoverRoot>
  </div>
</template>

<style scoped>
.thinking-panel {
  --thinking-accent: #527cff;
}
.thinking-panel-extended {
  --thinking-accent: #9852ff;
}
.thinking-title {
  color: var(--thinking-accent);
}
.thinking-fill {
  background: var(--thinking-accent);
  transition: width 160ms ease;
}
.thinking-panel-extended .thinking-fill {
  background:
    radial-gradient(circle at 16% 35%, #ffffff80 0 1px, transparent 2px),
    radial-gradient(circle at 63% 65%, #ffffff70 0 1px, transparent 2px), linear-gradient(100deg, #4845dc, #ad63ff);
  background-size:
    37px 100%,
    53px 100%,
    100% 100%;
}
@media (prefers-reduced-motion: reduce) {
  .thinking-fill {
    transition: none;
  }
}
.thinking-track {
  box-shadow: inset 0 0 0 1px var(--border);
}
.thinking-slider {
  display: block;
  margin: 0;
  appearance: none;
  background: transparent;
  border-radius: 999px;
}
.thinking-slider:focus-visible {
  outline: 2px solid var(--ring);
  outline-offset: 3px;
}
.thinking-slider::-webkit-slider-thumb {
  appearance: none;
  width: 36px;
  height: 36px;
  border: 1px solid var(--border);
  border-radius: 50%;
  background: var(--popover);
  cursor: grab;
  box-shadow: 0 1px 3px rgb(0 0 0 / 10%);
}
</style>
