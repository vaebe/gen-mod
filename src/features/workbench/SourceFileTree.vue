<script setup lang="ts">
import type { SourceTreeRow } from './useSourceBrowser'
import { ChevronRight, FileCode, Folder, FolderOpen, LoaderCircle, LockKeyhole } from '@lucide/vue'
import { Button } from '@/components/ui/button'

defineProps<{ rows: SourceTreeRow[], selected: string, selectedDirectory?: string, loading: boolean, error: string }>()
defineEmits<{ select: [path: string], selectDirectory: [path: string], toggle: [path: string], retry: [path: string] }>()
</script>

<template>
  <nav aria-label="源码文件" class="min-h-0 overflow-auto py-2 text-xs">
    <Button variant="ghost" size="sm" class="w-full justify-start rounded-none" :class="selectedDirectory === '' && !selected && 'bg-accent'" @click="$emit('selectDirectory', '')">
      <Folder data-icon="inline-start" />source
    </Button>
    <p v-if="loading" role="status" class="flex items-center gap-2 px-3 py-2 text-muted-foreground">
      <LoaderCircle class="size-3.5 animate-spin" />正在读取源码目录…
    </p>
    <div v-else-if="error" class="space-y-2 px-3 py-2">
      <p role="alert" class="break-words text-destructive">
        {{ error }}
      </p><Button type="button" variant="link" size="xs" class="h-auto p-0" @click="$emit('retry', '')">
        重试
      </Button>
    </div>
    <p v-else-if="!rows.length" class="px-3 py-4 text-muted-foreground">
      源码目录为空。
    </p>
    <template v-for="row in rows" :key="row.path">
      <div class="flex items-center" :style="{ paddingLeft: `${row.depth * 14}px` }">
        <Button v-if="row.kind === 'directory'" variant="ghost" size="icon" class="size-7 shrink-0 rounded-none" :aria-label="`${row.expanded ? '收起' : '展开'} ${row.name}`" :aria-expanded="row.expanded" @click="$emit('toggle', row.path)">
          <ChevronRight :class="row.expanded && 'rotate-90'" />
        </Button>
        <span v-else class="w-7 shrink-0" />
        <Button variant="ghost" size="sm" class="min-w-0 flex-1 justify-start rounded-none" :class="(selected === row.path || (row.kind === 'directory' && selectedDirectory === row.path)) && 'bg-accent text-accent-foreground'" :title="row.kind === 'blocked' ? `${row.path}（链接或特殊文件不可访问）` : row.path" :disabled="row.kind === 'blocked'" :aria-current="selected === row.path || selectedDirectory === row.path ? 'true' : undefined" @click="row.kind === 'directory' ? $emit('selectDirectory', row.path) : $emit('select', row.path)">
          <LoaderCircle v-if="row.loading" class="animate-spin" /><FolderOpen v-else-if="row.kind === 'directory' && row.expanded" /><Folder v-else-if="row.kind === 'directory'" /><LockKeyhole v-else-if="row.kind === 'blocked'" /><FileCode v-else />
          <span class="truncate font-mono">{{ row.name }}</span>
        </Button>
      </div>
      <div v-if="row.expanded && row.error" :style="{ paddingLeft: `${row.depth * 14 + 30}px` }" class="space-y-1 py-2 pr-3">
        <p role="alert" class="break-words text-destructive">
          {{ row.error }}
        </p><Button type="button" variant="link" size="xs" class="h-auto p-0" @click="$emit('retry', row.path)">
          重试
        </Button>
      </div>
      <p v-else-if="row.expanded && row.empty" :style="{ paddingLeft: `${row.depth * 14 + 30}px` }" class="py-1 text-muted-foreground">
        空文件夹
      </p>
    </template>
  </nav>
</template>
