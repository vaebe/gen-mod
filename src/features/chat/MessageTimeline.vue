<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, onMounted, shallowRef, useTemplateRef, watch } from "vue";
import { Check, Copy } from "@lucide/vue";
import type { ChatMessage, ToolRun } from "@/contracts/session";
import MessageParts from "./MessageParts.vue";
import RoundActivity from "./RoundActivity.vue";
import { conversationRounds } from "./conversationRounds";
import type { TaskRecord } from "@/contracts/task";
const props = defineProps<{ messages: ChatMessage[]; tools: Record<string, ToolRun>; modId: string; tasks: TaskRecord[]; working: boolean; startedAt?: number; endedAt?: number; activity: string; lastActivityAt: number }>();
const rounds = computed(() => conversationRounds(props.messages, props.tasks, props.working, props.startedAt, props.endedAt));
const scroll = useTemplateRef<HTMLElement>("scroll");
const content = useTemplateRef<HTMLElement>("content");
const following = shallowRef(true);
const copyingId = shallowRef("");
const copiedId = shallowRef("");
const copyError = shallowRef<{ id: string; message: string }>();
let copyTimer: ReturnType<typeof setTimeout> | undefined;
let resizeObserver: ResizeObserver | undefined;
async function copyReply(message: ChatMessage): Promise<void> {
  if (copyingId.value) { return; }
  copyingId.value = message.id;
  copyError.value = undefined;
  try {
    const text = message.parts.flatMap(part => part.type === "text" ? [part.text] : []).join("\n\n");
    await navigator.clipboard.writeText(text);
    copiedId.value = message.id;
    clearTimeout(copyTimer);
    copyTimer = setTimeout(() => { copiedId.value = ""; }, 2000);
  } catch (cause) {
    copyError.value = { id: message.id, message: `复制失败：${cause instanceof Error ? cause.message : String(cause)}` };
  } finally { copyingId.value = ""; }
}
function replyTime(timestamp?: number): string {
  if (timestamp === undefined || !Number.isFinite(timestamp)) { return ""; }
  const date = new Date(timestamp);
  return Number.isFinite(date.getTime()) ? date.toLocaleTimeString("zh-CN", { hour: "2-digit", minute: "2-digit", hour12: false }) : "";
}
/** 首次恢复及异步正文渲染时贴底；用户上翻后不抢回滚动位置。 */
function scrollToBottom(): void {
  if (scroll.value && following.value) { scroll.value.scrollTop = scroll.value.scrollHeight; }
}
onMounted(async () => {
  await nextTick();
  if (!content.value) { return; }
  scrollToBottom();
  resizeObserver = new ResizeObserver(scrollToBottom);
  resizeObserver.observe(content.value);
});
onBeforeUnmount(() => { resizeObserver?.disconnect(); clearTimeout(copyTimer); });
function onScroll(): void {
  const element = scroll.value;
  if (element) { following.value = element.scrollHeight - element.scrollTop - element.clientHeight < 80; }
}
watch(() => [props.messages, props.working], async () => {
  if (!following.value) { return; }
  await nextTick();
  scrollToBottom();
});
</script>
<template>
  <div ref="scroll" class="min-h-0 flex-1 overflow-auto px-5 py-5 lg:px-8" @scroll="onScroll"><div ref="content" class="mx-auto max-w-3xl space-y-5">
    <section v-for="round in rounds" :key="round.id" class="space-y-4">
      <article v-if="round.user" class="ml-auto max-w-[85%] space-y-2">
        <div v-if="round.user.parts.some(part => part.type === 'image')" class="flex flex-wrap justify-end gap-2">
          <img v-for="(image, index) in round.user.parts.filter(part => part.type === 'image')" :key="`${round.user.id}-image-${index}`" :src="`data:${image.mimeType};base64,${image.data}`" :alt="`会话图片附件 ${index + 1}`" class="size-20 rounded-xl border bg-muted object-cover" loading="lazy" />
        </div>
        <div v-if="round.user.parts.some(part => part.type === 'text' && part.text.trim()) || round.user.error" class="rounded-xl bg-muted px-3 py-2">
          <MessageParts :message="{ ...round.user, parts: round.user.parts.filter(part => part.type !== 'image') }" :tools="tools" :mod-id="modId" />
        </div>
        <p v-if="!round.user.final" role="status" class="text-right text-xs text-muted-foreground">{{ round.user.inputStatus === 'unknown' ? '提交状态未确认，请检查历史，勿重复发送' : round.user.inputStatus === 'failed' ? '提交失败' : '正在提交…' }}</p>
      </article>
      <article v-for="input in round.supplements" :key="input.id" class="ml-auto max-w-[85%] space-y-2">
        <div class="flex flex-wrap justify-end gap-2"><img v-for="(part,index) in input.parts.filter(part => part.type === 'image')" :key="index" :src="'data:' + part.mimeType + ';base64,' + part.data" alt="引导图片" class="size-20 rounded-xl border object-cover" /></div>
        <div class="rounded-xl bg-muted px-3 py-2"><MessageParts :message="{...input,parts:input.parts.filter(part => part.type !== 'image')}" :tools="tools" :mod-id="modId" /></div>
        <p class="text-right text-xs text-muted-foreground">{{ input.inputStatus === 'confirmed' ? '补充要求已进入会话' : input.inputStatus === 'handled' ? '补充要求由扩展接收' : input.inputStatus === 'unknown' ? '状态未确认，请检查历史' : ['failed', 'rejected'].includes(input.inputStatus ?? '') ? '补充要求提交失败' : '补充要求已提交，等待处理' }}</p>
      </article>
      <RoundActivity :round="round" :tools="tools" :mod-id="modId" :activity="activity" :last-activity-at="lastActivityAt" />
      <p v-if="!round.running && round.status === 'cancelled' && !round.process.some(message => message.stopReason === 'aborted')" class="text-xs text-muted-foreground">已手动停止</p>
      <p v-else-if="!round.running && round.status === 'interrupted'" class="text-xs text-muted-foreground">任务已中断，请检查运行日志</p>
      <article v-if="round.answer" class="assistant-reply select-text">
        <MessageParts :message="{ ...round.answer, parts: round.answer.parts.filter(part => part.type === 'text') }" :tools="tools" :mod-id="modId" />
        <div class="mt-2 flex select-none items-center gap-2 text-xs text-muted-foreground">
          <button type="button" class="inline-flex size-8 items-center justify-center rounded-md transition-colors hover:bg-muted hover:text-foreground disabled:opacity-50" :disabled="!!copyingId" :title="copiedId === round.answer.id ? '已复制' : '复制回复'" :aria-label="copiedId === round.answer.id ? '已复制' : '复制回复'" @click="copyReply(round.answer)">
            <Check v-if="copiedId === round.answer.id" class="size-4" aria-hidden="true" />
            <Copy v-else class="size-4" aria-hidden="true" />
          </button>
          <span v-if="copiedId === round.answer.id" role="status">已复制</span>
          <span v-if="replyTime(round.answer.timestamp)">{{ replyTime(round.answer.timestamp) }}</span>
        </div>
        <p v-if="copyError?.id === round.answer.id" role="alert" class="mt-2 text-xs text-destructive">{{ copyError.message }}</p>
      </article>
    </section>
  </div></div>
</template>

<style scoped>
.assistant-reply :deep(.chat-markdown) {
  --ms-text-body: 0.875rem;
  --ms-leading-body: 1.65;
  --ms-flow-paragraph-y: 0.5em;
  --ms-flow-list-y: 0.5em;
  --ms-flow-list-item-y: 0.15em;
  --ms-flow-hr-y: 1em;
  --ms-flow-codeblock-y: 0.75em;
  --ms-text-h1: 1.25rem;
  --ms-text-h2: 1.125rem;
  --ms-text-h3: 1rem;
  --ms-text-h4: 0.875rem;
  --ms-text-h5: 0.875rem;
  --ms-text-h6: 0.875rem;
  color: var(--foreground);
  overflow-wrap: anywhere;
  user-select: text;
}
.assistant-reply :deep(p code),
.assistant-reply :deep(li > code) {
  padding: 0.1em 0.35em;
  border-radius: 0.35rem;
  background: var(--muted);
  font-size: 0.9em;
}
.assistant-reply :deep(pre) {
  overflow-x: auto;
  padding: 0.75rem;
  border: 1px solid var(--border);
  border-radius: 0.75rem;
  background: var(--muted);
  font-size: 0.8125rem;
  line-height: 1.6;
}
</style>
