<script setup lang="ts">
import { shallowRef, watch } from "vue";
import MarkdownRender from "markstream-vue";
import "markstream-vue/index.css";
import type { SkillView } from "@/contracts/skill";
import { nativeCall } from "@/services/native";
import { Button } from "@/components/ui/button";
import { Dialog, DialogContent, DialogDescription, DialogFooter, DialogHeader, DialogTitle } from "@/components/ui/dialog";

const props = defineProps<{ skill: SkillView }>();
const open = defineModel<boolean>("open", { default: false });
const content = shallowRef("");
const loading = shallowRef(false);
const error = shallowRef("");

// 与聊天正文一致，仅渲染文本 Markdown，不执行文件中的 HTML 或扩展语法。
const markdown: NonNullable<import("markstream-vue").NodeRendererProps["customMarkdownIt"]> = (md) => {
  md.set({ html: false, linkify: false });
  md.disable(["html_inline", "html_block", "image", "math_inline", "math_block"], true);
  return md;
};

watch(() => [open.value, props.skill.name] as const, async ([visible, name], _, onCleanup) => {
  let current = true;
  onCleanup(() => { current = false; });
  content.value = ""; error.value = "";
  if (!visible) { return; }
  loading.value = true;
  try {
    const text = await nativeCall<string>("read_skill_content", { name });
    if (current) { content.value = text; }
  } catch (cause) { if (current) { error.value = (cause as Error).message; } }
  finally { if (current) { loading.value = false; } }
}, { immediate: true });

/** 定位当前已注册能力的 SKILL.md，失败时在弹窗中显示原因。 */
async function reveal(): Promise<void> {
  error.value = "";
  try { await nativeCall("reveal_skill_file", { name: props.skill.name }); }
  catch (cause) { error.value = (cause as Error).message; }
}

/** 文档内链接仅供阅读，避免 WebView 跳离应用。 */
function interceptLink(event: MouseEvent): void {
  if ((event.target as Element).closest("a")) { event.preventDefault(); event.stopPropagation(); }
}
</script>

<template>
  <Dialog v-model:open="open">
    <DialogContent class="flex max-h-[85dvh] flex-col overflow-hidden sm:max-w-3xl">
      <DialogHeader class="shrink-0 pr-6">
        <DialogTitle class="break-words">{{ skill.name }}</DialogTitle>
        <DialogDescription>SKILL.md · {{ skill.version }}</DialogDescription>
      </DialogHeader>
      <div class="min-h-0 flex-1 overflow-auto py-2" @click.capture="interceptLink">
        <p v-if="skill.error" role="alert" class="mb-3 text-sm text-destructive">{{ skill.error }}</p>
        <p v-if="loading" role="status" class="text-sm text-muted-foreground">正在读取 SKILL.md…</p>
        <MarkdownRender v-else-if="content" class="skill-markdown" mode="chat" :content="content" :final="true" :custom-markdown-it="markdown" :custom-html-tags="[]" :render-code-blocks-as-pre="true" :show-tooltips="false" />
        <p v-else-if="!error" class="text-sm text-muted-foreground">SKILL.md 内容为空。</p>
      </div>
      <p v-if="error" role="alert" class="shrink-0 break-words text-sm text-destructive">{{ error }}</p>
      <DialogFooter class="shrink-0">
        <Button variant="outline" @click="reveal">在文件资源管理器中显示</Button>
        <Button @click="open = false">关闭</Button>
      </DialogFooter>
    </DialogContent>
  </Dialog>
</template>

<style scoped>
.skill-markdown {
  --ms-text-body: 0.875rem;
  --ms-leading-body: 1.7;
  --ms-text-h1: 1.5rem;
  --ms-text-h2: 1.25rem;
  --ms-text-h3: 1.125rem;
  overflow-wrap: anywhere;
}
</style>
