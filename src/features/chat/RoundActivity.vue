<script setup lang="ts">
import { computed, shallowRef, watch } from "vue";
import { useIntervalFn } from "@vueuse/core";
import { ChevronRight } from "@lucide/vue";
import type { ConversationRound } from "./conversationRounds";
import type { ToolRun } from "@/contracts/session";
import MessageParts from "./MessageParts.vue";
const props = defineProps<{ round: ConversationRound; tools: Record<string, ToolRun>; modId: string; activity: string; lastActivityAt: number }>();
const now = shallowRef(Date.now());
const processOpen = shallowRef(false);
const processMessages = computed(() => props.round.process.filter(message => message.error || message.stopReason === "aborted" || message.parts.some(part => part.type === "tool" || (part.type === "text" && part.text.trim()))));
const timer = useIntervalFn(() => { now.value = Date.now(); }, 1000, { immediate: false });
watch(() => props.round.running, running => { now.value = Date.now(); if (running) { timer.resume(); } else { timer.pause(); } }, { immediate: true });
const duration = computed(() => {
  const start = props.round.startedAt;
  const end = props.round.running ? now.value : props.round.endedAt;
  if (start === undefined || end === undefined || !Number.isFinite(start) || !Number.isFinite(end)) { return "执行过程"; }
  const seconds = Math.max(0, Math.floor((end - start) / 1000));
  return `用时 ${seconds >= 60 ? `${Math.floor(seconds / 60)}分钟 ` : ''}${seconds % 60}秒`;
});
const summary = computed(() => props.lastActivityAt && now.value - props.lastActivityAt > 30_000 ? "等待模型或命令返回" : props.activity);
/** 折叠状态只保留最新一段公开过程正文，不展示模型思考。 */
const latestText = computed(() => {
  for (const message of props.round.process.slice().reverse()) {
    for (const part of message.parts.slice().reverse()) {
      if (part.type === "text" && part.text.trim()) { return { ...message, parts: [part], error: undefined, stopReason: undefined }; }
    }
  }
  return undefined;
});
</script>

<template>
  <div>
  <details class="group/round" @toggle="processOpen = ($event.target as HTMLDetailsElement).open">
    <summary class="flex cursor-pointer list-none flex-wrap items-center gap-2 border-b pb-3 text-xs text-muted-foreground focus-visible:outline focus-visible:outline-ring">
      <span>{{ duration }}</span><ChevronRight class="size-3.5 transition-transform group-open/round:rotate-90" />
      <span v-if="round.running" role="status" class="ml-1">{{ summary }}</span>
    </summary>
    <div class="space-y-2 py-2 text-muted-foreground"><MessageParts v-for="message in processMessages" :key="message.id" :message="message" :tools="tools" :mod-id="modId" /><p v-if="!processMessages.length" class="text-xs">{{ round.running ? '正在准备，等待模型响应…' : '暂无执行明细。' }}</p></div>
  </details>
  <div v-if="round.running && latestText && !processOpen" class="activity-text relative mt-2 overflow-hidden text-muted-foreground"><MessageParts :message="latestText" :tools="tools" :mod-id="modId" /></div>
  </div>
</template>

<style scoped>
.activity-text::after {
  content: "";
  position: absolute;
  inset: 0;
  pointer-events: none;
  background: linear-gradient(100deg, transparent 35%, color-mix(in srgb, var(--background) 45%, transparent) 50%, transparent 65%);
  transform: translateX(-100%);
  animation: activity-sheen 3s ease-in-out infinite;
}
@keyframes activity-sheen { to { transform: translateX(100%); } }
@media (prefers-reduced-motion: reduce) { .activity-text::after { animation: none; display: none; } }
</style>
