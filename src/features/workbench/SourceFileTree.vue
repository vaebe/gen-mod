<script setup lang="ts">
import { ChevronRight, FileCode, Folder, FolderOpen, LoaderCircle, LockKeyhole } from "@lucide/vue";
import type { SourceTreeRow } from "./useSourceBrowser";
defineProps<{ rows: SourceTreeRow[]; selected: string; loading: boolean; error: string }>();
defineEmits<{ select: [path: string]; toggle: [path: string]; retry: [path: string] }>();
</script>

<template>
  <nav aria-label="源码文件" class="min-h-0 overflow-auto py-2 text-xs">
    <p v-if="loading" role="status" class="flex items-center gap-2 px-3 py-2 text-muted-foreground"><LoaderCircle class="size-3.5 animate-spin" />正在读取源码目录…</p>
    <div v-else-if="error" class="space-y-2 px-3 py-2"><p role="alert" class="break-words text-destructive">{{ error }}</p><button type="button" class="underline underline-offset-4" @click="$emit('retry', '')">重试</button></div>
    <p v-else-if="!rows.length" class="px-3 py-4 text-muted-foreground">源码目录为空。</p>
    <template v-for="row in rows" :key="row.path">
      <button type="button" class="flex h-8 w-full items-center gap-1.5 pr-3 text-left transition-colors hover:bg-muted focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-inset focus-visible:ring-ring disabled:opacity-50" :class="selected === row.path && 'bg-accent text-accent-foreground'" :style="{ paddingLeft: `${row.depth * 14 + 10}px` }" :title="row.kind === 'blocked' ? `${row.path}（链接或特殊文件不可访问）` : row.path" :disabled="row.kind === 'blocked'" :aria-expanded="row.kind === 'directory' ? row.expanded : undefined" :aria-current="selected === row.path ? 'true' : undefined" @click="row.kind === 'directory' ? $emit('toggle', row.path) : $emit('select', row.path)">
        <ChevronRight v-if="row.kind === 'directory'" class="size-3 shrink-0 transition-transform" :class="row.expanded && 'rotate-90'" /><span v-else class="w-3 shrink-0" />
        <LoaderCircle v-if="row.loading" class="size-3.5 shrink-0 animate-spin text-muted-foreground" /><FolderOpen v-else-if="row.kind === 'directory' && row.expanded" class="size-3.5 shrink-0 text-muted-foreground" /><Folder v-else-if="row.kind === 'directory'" class="size-3.5 shrink-0 text-muted-foreground" /><LockKeyhole v-else-if="row.kind === 'blocked'" class="size-3.5 shrink-0" /><FileCode v-else class="size-3.5 shrink-0 text-muted-foreground" />
        <span class="truncate font-mono">{{ row.name }}</span>
      </button>
      <div v-if="row.expanded && row.error" :style="{ paddingLeft: `${row.depth * 14 + 30}px` }" class="space-y-1 py-2 pr-3"><p role="alert" class="break-words text-destructive">{{ row.error }}</p><button type="button" class="underline underline-offset-4" @click="$emit('retry', row.path)">重试</button></div>
      <p v-else-if="row.expanded && row.empty" :style="{ paddingLeft: `${row.depth * 14 + 30}px` }" class="py-1 text-muted-foreground">空文件夹</p>
    </template>
  </nav>
</template>
