<script setup lang="ts">
import { computed, onBeforeUnmount, shallowRef } from "vue";
import type { ArtifactVersion } from "@/contracts/mod";
import { Button } from "@/components/ui/button";
import { Check, Copy, Download, Eye, FolderOpen, MoreHorizontal, PackageCheck, Trash2 } from "@lucide/vue";
import { DropdownMenuRoot, DropdownMenuTrigger, DropdownMenuPortal, DropdownMenuContent, DropdownMenuItem, DropdownMenuSeparator } from "reka-ui";
export type VersionAction = "details" | "install" | "export" | "open" | "delete";
const props = defineProps<{ versions: ArtifactVersion[]; latest?: string; installed: string[]; busy: boolean }>();
defineEmits<{ action: [kind: VersionAction, id: string] }>();
const versionsNewestFirst = computed(() => [...props.versions].reverse());
const actions = [
  { kind: "details", label: "查看详情", icon: Eye },
  { kind: "install", label: "安装此版本", icon: PackageCheck },
  { kind: "export", label: "导出 ZIP", icon: Download },
  { kind: "open", label: "打开版本目录", icon: FolderOpen },
  { kind: "delete", label: "删除版本", icon: Trash2 },
] as const;
const copiedId = shallowRef("");
const copyError = shallowRef("");
let copyTimer: ReturnType<typeof setTimeout> | undefined;
async function copyVersion(id: string): Promise<void> {
  copyError.value = "";
  try {
    await navigator.clipboard.writeText(id);
    copiedId.value = id;
    clearTimeout(copyTimer);
    copyTimer = setTimeout(() => { copiedId.value = ""; }, 2000);
  } catch (cause) { copyError.value = `复制失败：${cause instanceof Error ? cause.message : String(cause)}`; }
}
function shortDate(value: string): string {
  return new Date(value).toLocaleString("zh-CN", { month: "2-digit", day: "2-digit", hour: "2-digit", minute: "2-digit", hour12: false });
}
onBeforeUnmount(() => clearTimeout(copyTimer));
</script>

<template>
  <section class="version-list flex min-h-0 flex-1 flex-col overflow-hidden rounded-lg border" aria-label="交付版本列表">
    <p v-if="copyError" role="alert" class="shrink-0 border-b px-3 py-2 text-xs text-destructive">{{ copyError }}</p>
    <div class="version-row shrink-0 border-b bg-muted/40 px-3 py-2 text-xs text-muted-foreground">
      <span>版本</span><span class="version-date">生成时间</span><span class="version-files">文件数</span><span>状态</span><span class="text-right">操作</span>
    </div>
    <div class="min-h-0 flex-1 overflow-y-auto">
      <p v-if="!versions.length" class="p-5 text-sm text-muted-foreground">尚无成功交付版本。生成完成并通过文件校验后会显示在这里。</p>
      <div v-for="version in versionsNewestFirst" :key="version.versionId" class="version-row min-h-13 border-b px-3 py-2 last:border-b-0 hover:bg-muted/30">
        <div class="min-w-0">
          <button type="button" class="inline-flex items-center gap-2 rounded text-xs hover:text-primary" :aria-label="`复制版本号 ${version.versionId}`" :title="copiedId === version.versionId ? '已复制' : `复制完整版本号 ${version.versionId}`" @click="copyVersion(version.versionId)">
            <span class="font-mono">{{ version.versionId.slice(0, 8) }}</span><Check v-if="copiedId === version.versionId" class="size-3 text-muted-foreground" aria-hidden="true" /><Copy v-else class="size-3 text-muted-foreground" aria-hidden="true" />
          </button>
          <span v-if="copiedId === version.versionId" class="sr-only" role="status">版本号已复制</span>
          <p class="version-mobile-meta mt-1 text-[11px] text-muted-foreground" :title="new Date(version.createdAt).toLocaleString()">{{ shortDate(version.createdAt) }} · {{ version.files.length }} 个文件</p>
        </div>
        <span class="version-date text-xs text-muted-foreground" :title="new Date(version.createdAt).toLocaleString()">{{ shortDate(version.createdAt) }}</span>
        <span class="version-files text-xs text-muted-foreground">{{ version.files.length }}</span>
        <div class="flex flex-wrap items-center gap-1">
          <span v-if="latest === version.versionId" class="rounded bg-muted px-1.5 py-0.5 text-[11px] text-muted-foreground">最新</span>
          <span v-if="installed.includes(version.versionId)" class="rounded bg-emerald-500/10 px-1.5 py-0.5 text-[11px] text-emerald-700 dark:text-emerald-400">已安装</span>
        </div>
        <DropdownMenuRoot>
          <DropdownMenuTrigger as-child><Button variant="ghost" size="icon" class="size-7 justify-self-end" :disabled="busy" :aria-label="`版本 ${version.versionId.slice(0, 8)} 的操作`"><MoreHorizontal class="size-4" /></Button></DropdownMenuTrigger>
          <DropdownMenuPortal><DropdownMenuContent align="end" :side-offset="5" class="z-50 min-w-40 rounded-lg border bg-popover p-1 text-popover-foreground shadow-md">
            <template v-for="action in actions" :key="action.kind">
              <DropdownMenuSeparator v-if="action.kind === 'delete'" class="my-1 h-px bg-border" />
              <DropdownMenuItem :disabled="busy" class="version-action" :class="action.kind === 'delete' && 'text-destructive'" @select="$emit('action', action.kind, version.versionId)"><component :is="action.icon" class="size-4" />{{ action.label }}</DropdownMenuItem>
            </template>
          </DropdownMenuContent></DropdownMenuPortal>
        </DropdownMenuRoot>
      </div>
    </div>
  </section>
</template>

<style scoped>
.version-list { container-type: inline-size; }
.version-date, .version-files { display: none; }
.version-row { display: grid; grid-template-columns: minmax(0, 1fr) 100px 28px; align-items: center; gap: 0.75rem; }
.version-action { display: flex; align-items: center; gap: 0.5rem; cursor: pointer; border-radius: 0.25rem; padding: 0.5rem 0.75rem; font-size: 0.8125rem; outline: none; }
.version-action[data-highlighted] { background: var(--muted); }
.version-action.text-destructive[data-highlighted] { background: color-mix(in srgb, var(--destructive) 10%, transparent); }
.version-action[data-disabled] { pointer-events: none; opacity: 0.4; }
@container (min-width: 560px) {
  .version-row { grid-template-columns: minmax(0, 1fr) 130px 56px 110px 28px; }
  .version-date, .version-files { display: block; }
  .version-mobile-meta { display: none; }
}
</style>
