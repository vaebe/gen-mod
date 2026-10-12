<script setup lang="ts">
import type { ConversationRound } from './conversationRounds'
import type { ToolRun } from '@/contracts/session'
import { ChevronRight, LoaderCircle } from '@lucide/vue'
import { useIntervalFn } from '@vueuse/core'
import { computed, nextTick, onBeforeUnmount, onMounted, shallowRef, useId, useTemplateRef, watch } from 'vue'
import MessageParts from './MessageParts.vue'

const props = defineProps<{ round: ConversationRound, tools: Record<string, ToolRun>, modId: string, activity: string, lastActivityAt: number }>()
const now = shallowRef(Date.now())
const processOpen = shallowRef(false)
const following = shallowRef(true)
const scroll = useTemplateRef<HTMLElement>('scroll')
const content = useTemplateRef<HTMLElement>('content')
const processId = useId()
const thinkingScroll = useTemplateRef<HTMLElement>('thinkingScroll')
const thinkingFollowing = shallowRef(true)
const activeThinking = computed(() => {
  if (!props.round.running) {
    return undefined
  }
  const message = props.round.process[props.round.process.length - 1]
  if (message && !message.final && !message.error && message.activeThinking !== undefined) {
    return { id: message.id, text: message.activeThinking }
  }
  return undefined
})
let resizeObserver: ResizeObserver | undefined
const processMessages = computed(() => props.round.process.filter(message => message.error || message.stopReason === 'aborted' || message.parts.some(part => part.type === 'tool' || (part.type === 'text' && part.text.trim()))))
const stopped = computed(() => props.round.status === 'cancelled' || props.round.process.some(message => message.stopReason === 'aborted'))
const failed = computed(() => !props.round.answer && (props.round.status === 'failed' || props.round.status === 'interrupted' || props.round.process.some(message => message.error || message.parts.some(part => part.type === 'tool' && props.tools[part.toolCallId]?.status === 'failed'))))
const status = computed(() => props.round.running ? '正在执行' : stopped.value ? '已停止' : failed.value ? '执行未完成' : props.round.answer ? '已完成' : '本轮已结束')
const timer = useIntervalFn(() => {
  now.value = Date.now()
}, 1000, { immediate: false })
watch(() => props.round.running, (running) => {
  now.value = Date.now()
  processOpen.value = running || stopped.value || failed.value
  if (running) {
    following.value = true
    timer.resume()
  }
  else { timer.pause() }
}, { immediate: true })
const duration = computed(() => {
  const start = props.round.startedAt
  const end = props.round.running ? now.value : props.round.endedAt
  if (start === undefined || end === undefined || !Number.isFinite(start) || !Number.isFinite(end)) {
    return ''
  }
  const seconds = Math.max(0, Math.floor((end - start) / 1000))
  return `用时 ${seconds >= 60 ? `${Math.floor(seconds / 60)}分钟 ` : ''}${seconds % 60}秒`
})
const summary = computed(() => activeThinking.value ? '正在思考…' : props.lastActivityAt && now.value - props.lastActivityAt > 30_000 ? '等待模型或命令返回' : props.activity)
function onThinkingScroll(): void {
  const element = thinkingScroll.value
  if (element) {
    thinkingFollowing.value = element.scrollHeight - element.scrollTop - element.clientHeight < 24
  }
}
watch(() => activeThinking.value?.id, () => {
  thinkingFollowing.value = true
})
watch(() => activeThinking.value?.text, async () => {
  await nextTick()
  if (thinkingScroll.value && thinkingFollowing.value) {
    thinkingScroll.value.scrollTop = thinkingScroll.value.scrollHeight
  }
  scrollToBottom()
})
function scrollToBottom(): void {
  if (scroll.value && processOpen.value && following.value) {
    scroll.value.scrollTop = scroll.value.scrollHeight
  }
}
function onScroll(): void {
  const element = scroll.value
  if (element) {
    following.value = element.scrollHeight - element.scrollTop - element.clientHeight < 40
  }
}
function followLatest(): void {
  following.value = true
  scrollToBottom()
}
watch(processOpen, async (open) => {
  if (open) {
    following.value = true
    await nextTick()
    scrollToBottom()
  }
})
watch(() => [props.round.process, props.tools], async () => {
  await nextTick()
  scrollToBottom()
})
onMounted(() => {
  if (content.value) {
    resizeObserver = new ResizeObserver(scrollToBottom)
    resizeObserver.observe(content.value)
  }
  scrollToBottom()
  if (thinkingScroll.value) {
    thinkingScroll.value.scrollTop = thinkingScroll.value.scrollHeight
  }
})
onBeforeUnmount(() => {
  resizeObserver?.disconnect()
})
</script>

<template>
  <div class="text-muted-foreground">
    <button type="button" class="flex w-full flex-wrap items-center gap-2 border-b pb-2 text-left text-xs focus-visible:outline focus-visible:outline-ring" :aria-expanded="processOpen" :aria-controls="processId" @click="processOpen = !processOpen">
      <ChevronRight class="size-3.5 shrink-0 transition-transform" :class="processOpen && 'rotate-90'" />
      <LoaderCircle v-if="round.running" class="size-3.5 shrink-0 animate-spin motion-reduce:animate-none" aria-hidden="true" />
      <span :class="failed && 'text-destructive'">{{ status }}</span>
      <span v-if="duration">· {{ duration }}</span>
      <span v-if="round.running" role="status" class="min-w-0 flex-1 truncate">{{ summary }}</span>
      <span v-else class="ml-auto">{{ processOpen ? '收起过程' : '查看过程' }}</span>
    </button>
    <div v-show="processOpen" :id="processId">
      <div ref="scroll" class="process-scroll overflow-y-auto overscroll-contain py-3 pr-2" @scroll.stop="onScroll">
        <div ref="content" class="space-y-3">
          <MessageParts v-for="message in processMessages" :key="message.id" :message="message" :tools="tools" :mod-id="modId" />
          <div v-if="activeThinking?.text.trim()" class="space-y-2">
            <p role="status" class="flex items-center gap-2 text-xs">
              <LoaderCircle class="size-3.5 shrink-0 animate-spin motion-reduce:animate-none" aria-hidden="true" />正在思考…
            </p>
            <div ref="thinkingScroll" class="max-h-40 overflow-y-auto overscroll-contain whitespace-pre-wrap break-words text-xs leading-5" @scroll.stop="onThinkingScroll">
              {{ activeThinking.text }}
            </div>
          </div>
          <p v-else-if="round.running && (!processMessages.length || activeThinking)" role="status" class="flex items-center gap-2 text-xs">
            <LoaderCircle class="size-3.5 shrink-0 animate-spin motion-reduce:animate-none" aria-hidden="true" />
            <span>{{ summary }}</span>
          </p>
          <p v-else-if="!round.running && !processMessages.length" class="text-xs">
            暂无执行明细。
          </p>
        </div>
      </div>
      <button v-if="round.running && !following" type="button" class="mt-1 rounded-md px-2 py-1 text-xs hover:bg-muted focus-visible:outline focus-visible:outline-ring" @click="followLatest">
        回到最新
      </button>
    </div>
  </div>
</template>

<style scoped>
.process-scroll {
  max-height: min(280px, 35vh);
  overflow-wrap: anywhere;
}
</style>
