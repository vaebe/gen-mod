<script setup lang="ts">
import MarkdownRender, { disableMermaid, disableKatex } from "markstream-vue";
import "markstream-vue/index.css";
import type { ChatMessage, ToolRun } from "@/contracts/session";
import ToolActivity from "./ToolActivity.vue";
import { nativeCall } from "@/services/native";
import { shallowRef } from "vue";
defineProps<{ message: ChatMessage; tools: Record<string, ToolRun>; modId: string }>();
const linkError = shallowRef("");
disableMermaid();
disableKatex();
// 仅使用文本 Markdown；原始 HTML、图像和扩展语法不参与渲染。
const markdown: NonNullable<import("markstream-vue").NodeRendererProps["customMarkdownIt"]> = (md) => {
  md.set({ html: false, linkify: false });
  md.disable(["html_inline", "html_block", "image", "math_inline", "math_block"], true);
  return md;
};
async function interceptLink(event: MouseEvent, modId: string): Promise<void> {
  const anchor = (event.target as HTMLElement).closest("a");
  if (!anchor) { return; }
  event.preventDefault();
  event.stopPropagation();
  linkError.value = "";
  try {
    await nativeCall("open_message_link", { modId, link: anchor.getAttribute("href") || "" });
  } catch (cause) { linkError.value = (cause as Error).message; }
}
</script>
<template>
  <div @click.capture="interceptLink($event, modId)">
    <template v-for="(part, index) in message.parts" :key="`${message.id}-${index}`">
      <p v-if="part.type === 'text' && message.role === 'user'" class="whitespace-pre-wrap text-sm leading-6">{{ part.text }}</p>
      <MarkdownRender v-else-if="part.type === 'text'" class="chat-markdown" mode="chat" :content="part.text" :final="message.final" :custom-markdown-it="markdown" :custom-html-tags="[]" :render-code-blocks-as-pre="true" :show-tooltips="false" />
      <ToolActivity v-else-if="part.type === 'tool' && tools[part.toolCallId]" :tool="tools[part.toolCallId]!" />
    </template>
    <p v-if="message.stopReason === 'aborted'" role="status" class="mt-3 text-xs text-muted-foreground">已手动停止</p>
    <p v-else-if="message.error" role="alert" class="mt-3 text-sm text-destructive">{{ message.error }}</p><p v-if="linkError" role="alert" class="mt-2 text-xs text-destructive">{{ linkError }}</p>
  </div>
</template>

<style scoped>
/* 使用渲染器的字号变量，让正文、列表和标题保持一致的缩放。 */
.chat-markdown {
  --ms-text-body: 0.875rem;
  --ms-leading-body: 1.7;
  --ms-flow-paragraph-y: 0.5em;
  --ms-text-h1: 1.5rem;
  --ms-text-h2: 1.25rem;
  --ms-text-h3: 1.125rem;
  --ms-text-h4: 1rem;
  --ms-text-h5: 0.875rem;
  --ms-text-h6: 0.875rem;
}
</style>
