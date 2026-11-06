<script setup lang="ts">
import type { ModelSelection, ProviderConfiguration, SessionModelState } from '@/contracts/config'
import type { ContextUsage, PromptAttachment, ThinkingLevel } from '@/contracts/session'
import { ArrowUp, FileText, Paperclip, Plus, Shrink, Square, X } from '@lucide/vue'
import { computed, shallowRef, useTemplateRef, watch } from 'vue'
import { Button } from '@/components/ui/button'
import { Popover, PopoverAnchor, PopoverContent, PopoverTrigger } from '@/components/ui/popover'
import { Textarea } from '@/components/ui/textarea'
import ChatImage from './ChatImage.vue'
import ContextUsageIndicator from './ContextUsageIndicator.vue'
import ModelSelector from './ModelSelector.vue'

const props = defineProps<{ attachments: PromptAttachment[], readingAttachments: boolean, working: boolean, canSteer: boolean, stopping: boolean, busy: boolean, configured: boolean, providers: ProviderConfiguration[], modelState?: SessionModelState, error: string, thinkingLevel: ThinkingLevel, thinkingLevels: ThinkingLevel[], contextUsage?: ContextUsage, compacting: boolean, canCompact: boolean }>()
const emit = defineEmits<{ send: [], cancel: [], save: [], model: [selection: ModelSelection], thinking: [level: ThinkingLevel], compact: [], files: [files: File[]], removeAttachment: [id: string] }>()
const requirement = defineModel<string>({ required: true })
const picker = useTemplateRef<HTMLInputElement>('picker')
const menuOpen = shallowRef(false)
const slashMenu = shallowRef(false)
const selectedAction = shallowRef(0)
const slash = computed(() => requirement.value.match(/(?:^|\s)\/([^\s/]*)$/u))
const contextPercent = computed(() => props.contextUsage?.tokens != null ? Math.round(Math.min(100, Math.max(0, props.contextUsage.tokens / props.contextUsage.contextWindow * 100))) : undefined)
const actions = computed(() => [
  { id: 'files', label: '选择文件', command: '/file', description: '图片、文本和源码', icon: Paperclip, disabled: props.readingAttachments },
  { id: 'compact', label: '压缩', command: '/compact', description: props.busy ? '会话空闲时可使用' : `压缩此聊天的上下文${contextPercent.value !== undefined ? `（已使用 ${contextPercent.value}%）` : ''}`, icon: Shrink, disabled: props.busy || !props.configured || !props.canCompact },
].filter(action => !slashMenu.value || `${action.label} ${action.command}`.toLowerCase().includes(slash.value?.[1]?.toLowerCase() ?? '')))
const imageError = computed(() => props.attachments.some(file => file.type === 'image') && props.modelState?.definition && !props.modelState.definition.input.includes('image') ? '当前模型不支持图片，请移除图片附件或切换模型' : '')
const canSend = computed(() => (!props.busy || props.canSteer) && !props.readingAttachments && props.configured && !imageError.value && (requirement.value.trim() || props.attachments.length))
watch(requirement, () => {
  if (slash.value) {
    slashMenu.value = true
    selectedAction.value = 0
    menuOpen.value = true
  }
  else if (slashMenu.value) {
    menuOpen.value = false
    slashMenu.value = false
  }
})
/** 两个入口使用同一份功能列表；斜杠指令只触发操作，不发给模型。 */
function runAction(id: string): void {
  const action = actions.value.find(item => item.id === id)
  if (!action || action.disabled) {
    return
  }
  const removeSlash = !!slash.value
  menuOpen.value = false
  if (removeSlash) {
    requirement.value = requirement.value.replace(/\/[^\s/]*$/u, '')
  }
  slashMenu.value = false
  if (id === 'files') {
    picker.value?.click()
  }
  else { emit('compact') }
}
function chooseFiles(event: Event): void {
  const input = event.target as HTMLInputElement
  if (input.files?.length) {
    emit('files', Array.from(input.files))
  }
  input.value = ''
}
function handleKey(event: KeyboardEvent): void {
  if (event.isComposing || event.keyCode === 229) {
    return
  }
  if (event.key === 'Enter' && event.shiftKey) {
    return
  }
  if (menuOpen.value && ['ArrowDown', 'ArrowUp', 'Enter', 'Escape'].includes(event.key) && !event.ctrlKey) {
    event.preventDefault()
    if (event.key === 'Escape') {
      menuOpen.value = false
    }
    else if (event.key === 'Enter') {
      const action = actions.value[selectedAction.value]
      if (action) {
        runAction(action.id)
      }
    }
    else if (actions.value.length) {
      selectedAction.value = (selectedAction.value + (event.key === 'ArrowDown' ? 1 : -1) + actions.value.length) % actions.value.length
    }
    return
  }
  if (event.target instanceof HTMLTextAreaElement && event.key === 'Enter' && !event.altKey && !event.metaKey) {
    event.preventDefault()
    if (!event.repeat && canSend.value && !slash.value) {
      emit('send')
    }
  }
}
function pasteFiles(event: ClipboardEvent): void {
  const files = Array.from(event.clipboardData?.files ?? [])
  if (files.length) {
    event.preventDefault()
    emit('files', files)
  }
}
</script>

<template>
  <div class="shrink-0 px-4 pt-3 pb-5 lg:px-8">
    <Popover v-model:open="menuOpen">
      <PopoverAnchor as-child>
        <div class="studio-composer mx-auto max-w-3xl rounded-xl border bg-card p-4">
          <input ref="picker" type="file" multiple class="hidden" @change="chooseFiles">
          <div v-if="attachments.length" class="mb-3 flex flex-wrap gap-2">
            <div v-for="file in attachments" :key="file.id" class="relative flex max-w-56 items-center text-xs" :class="file.type === 'image' ? 'shrink-0' : 'gap-2 rounded-lg border bg-muted/30 p-1.5'">
              <ChatImage v-if="file.type === 'image'" :image="file" :alt="file.name" compact /><FileText v-else class="size-5 shrink-0 text-muted-foreground" />
              <span v-if="file.type !== 'image'" class="truncate" :title="file.name">{{ file.name }}</span><Button variant="ghost" size="icon" class="shrink-0" :class="file.type === 'image' ? 'absolute -top-1.5 -right-1.5 z-10 size-5 rounded-full border bg-background shadow-xs hover:bg-muted' : 'size-6'" :aria-label="`移除 ${file.name}`" @click="$emit('removeAttachment', file.id)">
                <X class="size-3" />
              </Button>
            </div>
          </div>
          <Textarea v-model="requirement" rows="2" class="min-h-[56px] max-h-50 resize-none border-0 bg-transparent p-0 text-sm leading-6 shadow-none focus-visible:ring-0 dark:bg-transparent" aria-label="MOD 需求" placeholder="描述你希望实现的 MOD…" @blur="$emit('save')" @keydown="handleKey" @paste="pasteFiles" />
          <div class="mt-2 flex flex-wrap items-center gap-2">
            <PopoverTrigger as-child>
              <Button variant="ghost" size="icon" class="size-8 rounded-full text-muted-foreground" aria-label="添加文件或使用会话功能" title="功能（也可输入 /）" @click="slashMenu = false" @keydown="handleKey">
                <Plus class="size-4" />
              </Button>
            </PopoverTrigger>
            <span class="min-w-0 flex-1 truncate text-xs text-muted-foreground" title="Enter 发送，Shift + Enter 换行">{{ working ? '可发送补充要求' : 'Enter 发送 · Shift + Enter 换行' }}</span>
            <ContextUsageIndicator :usage="contextUsage" :compacting="compacting" />
            <ModelSelector :providers="providers" :model-state="modelState" :thinking-level="thinkingLevel" :thinking-levels="thinkingLevels" :disabled="busy || !modelState" :thinking-disabled="busy || !configured" @select="$emit('model', $event)" @thinking="$emit('thinking', $event)" />
            <Button v-if="working || compacting" size="icon" class="size-8 rounded-full" :disabled="stopping" aria-label="停止会话" title="停止会话" @click="$emit('cancel')">
              <Square class="size-3.5 fill-current" />
            </Button>
            <Button v-if="!(working || compacting) || (working && attachments.length + requirement.trim().length > 0)" size="icon" class="size-8 rounded-full" :disabled="!canSend || !!slash" :aria-label="working ? '发送引导' : '发送提示词'" :title="working ? '发送引导（Enter）' : '发送（Enter）'" @click="$emit('send')">
              <ArrowUp class="size-4" />
            </Button>
          </div>
          <p v-if="error || imageError" role="alert" class="mt-2 break-words text-xs leading-5 text-destructive">
            {{ error || imageError }}
          </p>
        </div>
      </PopoverAnchor>
      <PopoverContent side="top" align="start" :side-offset="8" class="w-(--reka-popover-trigger-width) max-h-80 overflow-y-auto p-1.5" @open-auto-focus="slashMenu && $event.preventDefault()" @interact-outside="slashMenu && ($event.target as HTMLElement).closest('[data-slot=textarea]') && $event.preventDefault()">
        <Button v-for="(action, index) in actions" :key="action.id" type="button" :disabled="action.disabled" variant="ghost" class="h-auto min-w-0 w-full justify-start gap-2.5" :class="slashMenu && selectedAction === index && 'bg-muted'" @click="runAction(action.id)">
          <component :is="action.icon" class="size-4 shrink-0 text-muted-foreground" /><span class="shrink-0 text-sm">{{ action.label }}</span><span class="min-w-0 truncate text-sm text-muted-foreground">{{ action.description }}</span>
        </Button>
        <p v-if="!actions.length" class="px-3 py-2 text-xs text-muted-foreground">
          没有匹配的功能
        </p>
      </PopoverContent>
    </Popover>
  </div>
</template>
