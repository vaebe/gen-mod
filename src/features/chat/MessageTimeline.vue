<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, onMounted, shallowRef, useTemplateRef, watch } from "vue";
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
let resizeObserver: ResizeObserver | undefined;
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
onBeforeUnmount(() => resizeObserver?.disconnect());
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
  <div ref="scroll" class="min-h-0 flex-1 overflow-auto px-5 py-8 lg:px-8" @scroll="onScroll"><div ref="content" class="mx-auto max-w-3xl space-y-7">
    <section v-for="round in rounds" :key="round.id" class="space-y-6">
      <article v-if="round.user" class="ml-auto max-w-[85%] space-y-2">
        <div v-if="round.user.parts.some(part => part.type === 'image')" class="flex flex-wrap justify-end gap-2">
          <img v-for="(image, index) in round.user.parts.filter(part => part.type === 'image')" :key="`${round.user.id}-image-${index}`" :src="`data:${image.mimeType};base64,${image.data}`" :alt="`会话图片附件 ${index + 1}`" class="size-20 rounded-xl border bg-muted object-cover" loading="lazy" />
        </div>
        <div v-if="round.user.parts.some(part => part.type === 'text' && part.text.trim()) || round.user.error" class="rounded-xl bg-muted px-4 py-3">
          <MessageParts :message="{ ...round.user, parts: round.user.parts.filter(part => part.type !== 'image') }" :tools="tools" :mod-id="modId" />
        </div>
      </article>
      <article v-for="input in round.supplements" :key="input.id" class="ml-auto max-w-[85%] space-y-2">
        <div class="flex flex-wrap justify-end gap-2"><img v-for="(part,index) in input.parts.filter(part => part.type === 'image')" :key="index" :src="'data:' + part.mimeType + ';base64,' + part.data" alt="引导图片" class="size-20 rounded-xl border object-cover" /></div>
        <div class="rounded-xl bg-muted px-4 py-3"><MessageParts :message="{...input,parts:input.parts.filter(part => part.type !== 'image')}" :tools="tools" :mod-id="modId" /></div>
        <p class="text-right text-xs text-muted-foreground">{{ input.inputStatus === 'confirmed' ? '补充要求已进入会话' : input.inputStatus === 'handled' ? '补充要求由扩展接收' : input.inputStatus === 'unknown' ? '状态未确认，请检查历史' : '补充要求已提交，等待处理' }}</p>
      </article>
      <RoundActivity :round="round" :tools="tools" :mod-id="modId" :activity="activity" :last-activity-at="lastActivityAt" />
      <p v-if="!round.running && round.status === 'cancelled' && !round.process.some(message => message.stopReason === 'aborted')" class="text-xs text-muted-foreground">已手动停止</p>
      <p v-else-if="!round.running && round.status === 'interrupted'" class="text-xs text-muted-foreground">任务已中断，请检查运行日志</p>
      <article v-if="round.answer"><MessageParts :message="{ ...round.answer, parts: round.answer.parts.filter(part => part.type === 'text') }" :tools="tools" :mod-id="modId" /></article>
    </section>
  </div></div>
</template>
