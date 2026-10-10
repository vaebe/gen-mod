<script setup lang="ts">
import { computed, shallowRef, watch } from "vue";
import { FileCode, LoaderCircle } from "@lucide/vue";
import type { SourcePreview } from "@/contracts/source";
import { nativeCall } from "@/services/native";
import { Button } from "@/components/ui/button";
import { highlightSource } from "./sourceHighlight";

const props = defineProps<{ modId: string; path: string; preview?: SourcePreview; loading: boolean; error: string }>();
const actionError = shallowRef("");
const highlighted = shallowRef("");
const highlightError = shallowRef("");
const imageUrl = shallowRef("");
const imageLoading = shallowRef(false);
const imageError = shallowRef("");
const originalSize = shallowRef(false);
const imageDimensions = shallowRef("");
const lines = computed(() => props.preview?.text?.split(/\r\n|\n|\r/u) ?? []);
const lineNumbers = computed(() => lines.value.map((_, index) => index + 1).join("\n"));
const displayText = computed(() => lines.value.join("\n"));
const unavailableMessage = computed(() => {
  if (props.preview?.unavailableReason === "tooLarge") { return `文件超过 ${props.preview.previewLimit / 1024 / 1024} MiB，无法预览。`; }
  if (props.preview?.unavailableReason === "binary") { return "此二进制文件不支持预览。"; }
  if (props.preview?.unavailableReason === "encoding") { return "文件不是 UTF-8 文本，无法预览。"; }
  return "";
});
watch(() => [props.modId, props.path], () => { actionError.value = ""; });

// 只插入 Shiki 对源码转义后生成的 HTML，不直接渲染源文件中的 HTML。
watch(() => [props.modId, props.path, props.preview?.text] as const, async ([, path, text], _, onCleanup) => {
  let current = true;
  onCleanup(() => { current = false; });
  highlighted.value = ""; highlightError.value = "";
  if (!text) { return; }
  try {
    const html = await highlightSource(text.replace(/\r\n|\r/gu, "\n"), path);
    if (current) { highlighted.value = html ?? ""; }
  } catch (cause) { if (current) { highlightError.value = `高亮加载失败，已显示原文：${(cause as Error).message}`; } }
}, { immediate: true });

// 图片通过二进制 IPC 获取，切换文件或关闭预览时释放 Object URL。
watch(() => [props.modId, props.path, props.preview] as const, async ([modId, path, preview], _, onCleanup) => {
  let current = true;
  let url = "";
  onCleanup(() => { current = false; if (url) { URL.revokeObjectURL(url); } });
  imageUrl.value = ""; imageError.value = ""; imageDimensions.value = ""; originalSize.value = false; imageLoading.value = false;
  if (!preview?.imageMimeType || preview.unavailableReason) { return; }
  imageLoading.value = true;
  try {
    const bytes = await nativeCall<ArrayBuffer>("read_source_image", { modId, relativePath: path });
    if (!current) { return; }
    url = URL.createObjectURL(new Blob([bytes], { type: preview.imageMimeType }));
    imageUrl.value = url;
  } catch (cause) { if (current) { imageError.value = (cause as Error).message; } }
  finally { if (current) { imageLoading.value = false; } }
}, { immediate: true });

/** 记录图片实际像素尺寸；事件必须仍属于当前预览图片。 */
function imageLoaded(event: Event): void {
  const image = event.target as HTMLImageElement;
  if (image.src === imageUrl.value) { imageDimensions.value = `${image.naturalWidth} × ${image.naturalHeight}`; }
}

/** 损坏图片解码失败时给出明确反馈，仍允许定位原文件。 */
function imageFailed(event: Event): void {
  if ((event.target as HTMLImageElement).src === imageUrl.value) { imageError.value = "图片无法解码，请定位文件检查图片内容。"; }
}

/** 复制完整原文或使用已有原生接口定位当前文件。 */
async function action(kind: "copy" | "reveal"): Promise<void> {
  const modId = props.modId;
  const path = props.path;
  actionError.value = "";
  try {
    if (kind === "copy") { await navigator.clipboard.writeText(props.preview?.text ?? ""); }
    else { await nativeCall("reveal_source_file", { modId, relativePath: path }); }
  } catch (cause) { if (modId === props.modId && path === props.path) { actionError.value = (cause as Error).message; } }
}
</script>

<template>
  <section aria-label="源码预览" class="flex min-h-0 min-w-0 flex-col bg-background">
    <div v-if="path" class="flex shrink-0 flex-wrap items-center gap-1 border-b px-3 py-2">
      <span class="mr-auto min-w-0 break-all font-mono text-xs" :title="path">{{ path }}</span>
      <Button v-if="imageUrl && !imageError" size="sm" variant="ghost" class="h-7 px-2 text-xs" @click="originalSize = !originalSize">{{ originalSize ? '适应面板' : '原尺寸' }}</Button>
      <Button size="sm" variant="ghost" class="h-7 px-2 text-xs" :disabled="preview?.text == null || loading" @click="action('copy')">复制</Button>
      <Button size="sm" variant="ghost" class="h-7 px-2 text-xs" :disabled="loading || !!error" @click="action('reveal')">定位</Button>
    </div>
    <div v-if="!path" class="flex flex-1 flex-col items-center justify-center gap-3 p-6 text-center text-muted-foreground"><FileCode class="size-7 opacity-50" /><p class="text-xs">选择文件查看源码</p></div>
    <p v-else-if="loading" role="status" class="flex items-center gap-2 p-4 text-xs text-muted-foreground"><LoaderCircle class="size-3.5 animate-spin" />正在读取文件…</p>
    <p v-else-if="error" role="alert" class="break-words p-4 text-xs text-destructive">{{ error }}</p>
    <p v-else-if="preview?.unavailableReason" class="p-4 text-xs leading-6 text-muted-foreground">{{ unavailableMessage }}大小 {{ preview.size.toLocaleString() }} B，可定位后查看。</p>
    <template v-else-if="preview?.imageMimeType">
      <p v-if="imageLoading" role="status" class="flex items-center gap-2 p-4 text-xs text-muted-foreground"><LoaderCircle class="size-3.5 animate-spin" />正在读取图片…</p>
      <p v-else-if="imageError" role="alert" class="break-words p-4 text-xs text-destructive">{{ imageError }}</p>
      <div v-else-if="imageUrl" class="image-preview min-h-0 flex-1 overflow-auto p-4"><div class="flex min-h-full min-w-full items-center justify-center" :class="originalSize ? 'w-max' : 'h-full'"><img :key="imageUrl" :src="imageUrl" :alt="path" :class="originalSize ? 'max-w-none shrink-0' : 'max-h-full max-w-full object-contain'" @load="imageLoaded" @error="imageFailed" /></div></div>
    </template>
    <p v-else-if="preview?.text === ''" class="p-4 text-xs text-muted-foreground">文件为空。</p>
    <div v-else-if="preview?.text != null" :key="`${modId}/${path}`" class="min-h-0 flex-1 overflow-auto py-2 font-mono text-[12px] leading-5">
      <div class="flex w-max min-w-full"><pre aria-hidden="true" class="sticky left-0 z-10 m-0 min-w-12 shrink-0 select-none border-r bg-background px-3 text-right text-muted-foreground/60">{{ lineNumbers }}</pre><div v-if="highlighted" class="highlighted-code flex-1 px-3 [tab-size:4]" v-html="highlighted" /><pre v-else class="m-0 flex-1 whitespace-pre px-3 [tab-size:4]">{{ displayText }}</pre></div>
    </div>
    <p v-if="actionError" role="alert" class="shrink-0 break-words border-t px-3 py-2 text-xs text-destructive">{{ actionError }}</p>
    <p v-if="highlightError" role="status" class="shrink-0 break-words border-t px-3 py-2 text-xs text-muted-foreground">{{ highlightError }}</p>
    <div v-if="preview" class="flex shrink-0 flex-wrap justify-between gap-2 border-t px-3 py-1.5 text-[10px] text-muted-foreground"><span>只读 · {{ preview.imageMimeType ? '图片' : '当前源码' }}<span v-if="imageDimensions"> · {{ imageDimensions }}</span></span><span>{{ preview.size.toLocaleString() }} B</span></div>
  </section>
</template>

<style scoped>
.highlighted-code :deep(pre) { margin: 0; background: transparent; }
.highlighted-code :deep(pre), .highlighted-code :deep(code) { font: inherit; white-space: pre; }
.highlighted-code :deep(.shiki), .highlighted-code :deep(.shiki span) { color: var(--shiki-light); }
:global(.dark .highlighted-code .shiki), :global(.dark .highlighted-code .shiki span) { color: var(--shiki-dark); }
.image-preview { background-image: conic-gradient(from 90deg, var(--color-muted) 25%, transparent 0 50%, var(--color-muted) 0 75%, transparent 0); background-size: 16px 16px; }
</style>
