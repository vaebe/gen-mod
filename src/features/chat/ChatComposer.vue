<script setup lang="ts">
import { Textarea } from "@/components/ui/textarea";
import { Button } from "@/components/ui/button";
import { ArrowUp, Square, Plus, Paperclip, Shrink, X, FileText, ChevronDown, Check } from "@lucide/vue";
import { computed, shallowRef, useTemplateRef, watch } from "vue";
import { PopoverRoot, PopoverAnchor, PopoverTrigger, PopoverPortal, PopoverContent, DropdownMenuRoot, DropdownMenuTrigger, DropdownMenuPortal, DropdownMenuContent, DropdownMenuItem } from "reka-ui";
import ContextUsageIndicator from "./ContextUsageIndicator.vue";
import type { ContextUsage, PromptAttachment, ThinkingLevel } from "@/contracts/session";
const requirement = defineModel<string>({ required: true });
const props = defineProps<{ attachments: PromptAttachment[]; readingAttachments: boolean; working: boolean; busy: boolean; saving: boolean; configured: boolean; model: string; error: string; thinkingLevel: ThinkingLevel; thinkingLevels: ThinkingLevel[]; contextUsage?: ContextUsage; compacting: boolean; canCompact: boolean }>();
const emit = defineEmits<{ send: []; cancel: []; save: []; thinking: [level: ThinkingLevel]; compact: []; files: [files: File[]]; removeAttachment: [id: string] }>();
const picker = useTemplateRef<HTMLInputElement>("picker");
const menuOpen = shallowRef(false);
const slashMenu = shallowRef(false);
const selectedAction = shallowRef(0);
const slash = computed(() => requirement.value.match(/(?:^|\s)\/([^\s/]*)$/u));
const contextPercent = computed(() => props.contextUsage?.tokens != null ? Math.round(Math.min(100, Math.max(0, props.contextUsage.tokens / props.contextUsage.contextWindow * 100))) : undefined);
const actions = computed(() => [
  { id: "files", label: "选择文件", command: "/file", description: "图片、文本和源码", icon: Paperclip, disabled: props.readingAttachments },
  { id: "compact", label: "压缩", command: "/compact", description: props.busy ? "会话空闲时可使用" : `压缩此聊天的上下文${contextPercent.value !== undefined ? `（已使用 ${contextPercent.value}%）` : ''}`, icon: Shrink, disabled: props.busy || !props.configured || !props.canCompact },
].filter(action => !slashMenu.value || `${action.label} ${action.command}`.toLowerCase().includes(slash.value?.[1]?.toLowerCase() ?? "")));
const canSend = computed(() => !props.busy && !props.saving && !props.readingAttachments && props.configured && (requirement.value.trim() || props.attachments.length));
watch(requirement, () => {
  if (slash.value) { slashMenu.value = true; selectedAction.value = 0; menuOpen.value = true; }
  else if (slashMenu.value) { menuOpen.value = false; slashMenu.value = false; }
});
/** 两个入口使用同一份功能列表；斜杠指令只触发操作，不发给模型。 */
function runAction(id: string): void {
  const action = actions.value.find(item => item.id === id);
  if (!action || action.disabled) { return; }
  const removeSlash = !!slash.value;
  menuOpen.value = false;
  if (removeSlash) { requirement.value = requirement.value.replace(/\/[^\s/]*$/u, ""); }
  slashMenu.value = false;
  if (id === "files") { picker.value?.click(); }
  else { emit("compact"); }
}
function chooseFiles(event: Event): void {
  const input = event.target as HTMLInputElement;
  if (input.files?.length) { emit("files", Array.from(input.files)); }
  input.value = "";
}
function handleKey(event: KeyboardEvent): void {
  if (event.isComposing) { return; }
  if (menuOpen.value && ["ArrowDown", "ArrowUp", "Enter", "Escape"].includes(event.key) && !event.ctrlKey) {
    event.preventDefault();
    if (event.key === "Escape") { menuOpen.value = false; }
    else if (event.key === "Enter") { const action = actions.value[selectedAction.value]; if (action) { runAction(action.id); } }
    else if (actions.value.length) { selectedAction.value = (selectedAction.value + (event.key === "ArrowDown" ? 1 : -1) + actions.value.length) % actions.value.length; }
    return;
  }
  if (event.ctrlKey && event.key === "Enter") { event.preventDefault(); if (canSend.value && !slash.value) { emit("send"); } }
}
function pasteFiles(event: ClipboardEvent): void {
  const files = Array.from(event.clipboardData?.files ?? []);
  if (files.length) { event.preventDefault(); emit("files", files); }
}
const labels: Record<ThinkingLevel, string> = { off: "关闭", minimal: "最低", low: "低", medium: "中", high: "高", xhigh: "更高", max: "最高" };
</script>
<template>
  <div class="shrink-0 px-4 pt-3 pb-5 lg:px-8">
    <PopoverRoot v-model:open="menuOpen"><PopoverAnchor as-child>
    <div class="mx-auto max-w-3xl rounded-2xl border bg-card p-3 shadow-xs">
      <input ref="picker" type="file" multiple class="hidden" @change="chooseFiles" />
      <div v-if="attachments.length" class="mb-3 flex flex-wrap gap-2">
        <div v-for="file in attachments" :key="file.id" class="flex max-w-56 items-center gap-2 rounded-lg border bg-muted/30 p-1.5 text-xs">
          <img v-if="file.type === 'image'" :src="`data:${file.mimeType};base64,${file.data}`" :alt="file.name" class="size-10 rounded object-cover" /><FileText v-else class="size-5 shrink-0 text-muted-foreground" />
          <span class="truncate" :title="file.name">{{ file.name }}</span><Button variant="ghost" size="icon" class="size-6 shrink-0" :aria-label="`移除 ${file.name}`" @click="$emit('removeAttachment', file.id)"><X class="size-3" /></Button>
        </div>
      </div>
      <Textarea v-model="requirement" rows="2" class="min-h-[56px] max-h-50 resize-none border-0 p-0 shadow-none focus-visible:ring-0" aria-label="MOD 需求" placeholder="描述你希望实现的 MOD…" @blur="$emit('save')" @keydown="handleKey" @paste="pasteFiles" />
      <div class="mt-3 flex flex-wrap items-center gap-2">
        <PopoverTrigger as-child><Button variant="ghost" size="icon" class="size-8 rounded-full text-muted-foreground" aria-label="添加文件或使用会话功能" title="功能（也可输入 /）" @click="slashMenu = false" @keydown="handleKey"><Plus class="size-4" /></Button></PopoverTrigger>
        <span class="min-w-0 flex-1 truncate text-[11px] text-muted-foreground">{{ working ? '新增需求保存为草稿' : '' }}</span>
        <ContextUsageIndicator :usage="contextUsage" :compacting="compacting" />
        <span class="max-w-40 truncate text-xs text-foreground">{{ model }}</span>
        <DropdownMenuRoot><DropdownMenuTrigger as-child><Button variant="ghost" size="sm" class="h-8 gap-1 rounded-lg px-2 text-xs text-muted-foreground" :disabled="busy || !thinkingLevels.length" aria-label="模型思考强度"><span>{{ labels[thinkingLevel] }}</span><ChevronDown class="size-3.5" /></Button></DropdownMenuTrigger>
          <DropdownMenuPortal><DropdownMenuContent side="top" align="end" :side-offset="8" class="z-50 min-w-44 rounded-xl border bg-popover p-1.5 text-popover-foreground shadow-md">
            <p class="px-3 pt-2 pb-1.5 text-xs text-muted-foreground">思考强度</p>
            <DropdownMenuItem v-for="level in thinkingLevels" :key="level" class="flex cursor-pointer items-center justify-between gap-6 rounded-lg px-3 py-2.5 text-sm outline-none data-[highlighted]:bg-muted" :class="thinkingLevel === level && 'bg-muted/60'" @select="!busy && level !== thinkingLevel && $emit('thinking', level)"><span>{{ labels[level] }}</span><Check v-if="thinkingLevel === level" class="size-4 text-muted-foreground" /></DropdownMenuItem>
          </DropdownMenuContent></DropdownMenuPortal>
        </DropdownMenuRoot>
        <Button v-if="working || compacting" size="icon" class="size-8 rounded-full" aria-label="停止会话" title="停止会话" @click="$emit('cancel')"><Square class="size-3.5 fill-current" /></Button>
        <Button v-else size="icon" class="size-8 rounded-full" :disabled="!canSend || !!slash" aria-label="发送提示词" title="发送（Ctrl + Enter）" @click="$emit('send')"><ArrowUp class="size-4" /></Button>
      </div>
      <p v-if="error" role="alert" class="mt-2 text-xs text-destructive">{{ error }}</p>
    </div>
    </PopoverAnchor>
    <PopoverPortal><PopoverContent side="top" align="start" :side-offset="8" class="z-50 w-[var(--reka-popover-trigger-width)] max-h-80 overflow-y-auto rounded-2xl border bg-popover p-1.5 text-popover-foreground shadow-sm" @open-auto-focus="slashMenu && $event.preventDefault()" @interact-outside="slashMenu && ($event.target as HTMLElement).closest('[data-slot=textarea]') && $event.preventDefault()">
      <button v-for="(action, index) in actions" :key="action.id" type="button" :disabled="action.disabled" class="flex w-full items-center gap-2.5 rounded-full px-3 py-2 text-left hover:bg-muted focus-visible:bg-muted focus-visible:outline-none disabled:opacity-40" :class="slashMenu && selectedAction === index && 'bg-muted'" @click="runAction(action.id)"><component :is="action.icon" class="size-4 shrink-0 text-muted-foreground" /><span class="shrink-0 text-sm">{{ action.label }}</span><span class="min-w-0 truncate text-sm text-muted-foreground">{{ action.description }}</span></button>
      <p v-if="!actions.length" class="px-3 py-2 text-xs text-muted-foreground">没有匹配的功能</p>
    </PopoverContent></PopoverPortal></PopoverRoot>
  </div>
</template>
