<script setup lang="ts">
import type { ModelSelection, ProviderConfiguration, SessionModelState } from '@/contracts/config'
import type { ThinkingLevel } from '@/contracts/session'
import { Check, ChevronDown, RotateCcw, Zap } from '@lucide/vue'
import { computed, shallowRef, watch } from 'vue'
import { Button } from '@/components/ui/button'
import { DropdownMenu, DropdownMenuContent, DropdownMenuGroup, DropdownMenuItem, DropdownMenuLabel, DropdownMenuTrigger } from '@/components/ui/dropdown-menu'
import { Popover, PopoverContent, PopoverTrigger } from '@/components/ui/popover'
import { Slider } from '@/components/ui/slider'

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
    <DropdownMenu v-if="canChoose" v-model:open="modelOpen">
      <DropdownMenuTrigger as-child>
        <Button variant="ghost" size="sm" class="h-8 min-w-0 gap-1.5 rounded-full px-3 text-xs" :disabled="disabled" aria-label="选择模型" :title="name">
          <span class="max-w-40 truncate">{{ name }}</span><ChevronDown class="size-3.5 shrink-0 text-muted-foreground" />
        </Button>
      </DropdownMenuTrigger>
      <DropdownMenuContent side="top" align="end" :side-offset="8" class="max-h-80 w-72 max-w-[calc(100vw-2rem)] overflow-y-auto">
        <DropdownMenuGroup><DropdownMenuLabel>选择模型</DropdownMenuLabel></DropdownMenuGroup>
        <DropdownMenuGroup v-for="provider in providers" :key="provider.id">
          <DropdownMenuLabel v-if="provider.models.length">
            {{ provider.name }}
          </DropdownMenuLabel>
          <DropdownMenuItem v-for="model in provider.models" :key="model.id" :disabled="disabled || !provider.keyConfigured" :title="provider.keyConfigured ? `${provider.name} · ${model.id}` : '请先在设置页填写此服务商的 API Key'" class="justify-between gap-4" @select="choose(provider.id, model.id)">
            <span class="min-w-0 truncate">{{ model.name }}</span><Check v-if="modelState?.selection.providerId === provider.id && modelState.selection.modelId === model.id" class="size-4 shrink-0 text-muted-foreground" />
          </DropdownMenuItem>
        </DropdownMenuGroup>
      </DropdownMenuContent>
    </DropdownMenu>
    <span v-else class="max-w-44 truncate px-3 py-1.5 text-xs" :title="name">{{ name }}</span>
    <Popover v-if="modelState?.definition?.reasoning && thinkingLevels.length" v-model:open="thinkingOpen">
      <PopoverTrigger as-child>
        <Button variant="ghost" size="sm" class="h-8 gap-1 rounded-full pr-3 pl-2 text-xs text-muted-foreground" :disabled="thinkingDisabled" aria-label="选择思考强度">
          <span>{{ labels[thinkingLevel] }}</span><ChevronDown class="size-3.5" />
        </Button>
      </PopoverTrigger>
      <PopoverContent side="top" align="end" :side-offset="8" class="w-80 max-w-[calc(100vw-2rem)]">
        <div>
          <div class="flex items-start justify-between gap-3">
            <Zap class="mt-1 size-4 text-muted-foreground" />
            <div class="min-w-0 text-center">
              <p class="text-lg font-semibold">
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
          <Slider v-if="thinkingLevels.length > 1" :model-value="[previewIndex]" :min="0" :max="thinkingLevels.length - 1" :step="1" :disabled="thinkingDisabled" class="mt-5" aria-label="模型思考强度" :aria-valuetext="labels[previewLevel]" @update:model-value="previewIndex = $event?.[0] ?? previewIndex" @value-commit="changeThinking(thinkingLevels[$event[0]!])" />
          <p v-else class="mt-4 text-center text-xs text-muted-foreground">
            此模型仅支持 {{ labels[thinkingLevel] }}
          </p>
        </div>
      </PopoverContent>
    </Popover>
  </div>
</template>
