<script setup lang="ts">
import type { SourceTreeRow } from './useSourceBrowser'
import type { SourceTreeEdit } from './useSourceTreeActions'
import type { SourceEntry } from '@/contracts/source'
import { ChevronRight, FileCode, Folder, FolderOpen, FolderPlus, LoaderCircle, LockKeyhole, Pencil, Upload } from '@lucide/vue'
import { computed, defineAsyncComponent, shallowRef, watch } from 'vue'
import { Button } from '@/components/ui/button'
import { ContextMenu, ContextMenuContent, ContextMenuGroup, ContextMenuItem, ContextMenuTrigger } from '@/components/ui/context-menu'
import { cn } from '@/lib/utils'

const props = defineProps<{
  rows: SourceTreeRow[]
  selected: string
  loading: boolean
  error: string
  editable?: boolean
  disabled?: boolean
  pending?: boolean
  editing?: SourceTreeEdit
  editError?: string
}>()
const emit = defineEmits<{
  select: [path: string]
  selectDirectory: [path: string]
  toggle: [path: string]
  retry: [path: string]
  focus: [entry: SourceEntry]
  create: [path: string]
  rename: [entry: SourceEntry]
  import: [path: string]
  submit: [name: string]
  cancel: []
  editName: [name: string]
  move: [entry: SourceEntry, targetDirectory: string]
}>()
const SourceNameInput = defineAsyncComponent(() => import('./SourceNameInput.vue'))
const dragged = shallowRef<SourceTreeRow>()
const dropTarget = shallowRef<string>()
const dragType = 'application/x-gen-mod-source'
type DisplayRow = SourceTreeRow & { creating?: boolean }
const displayRows = computed(() => {
  const root: SourceTreeRow = { path: '', name: 'source', kind: 'directory', depth: -1, expanded: true, loading: props.loading, error: '', empty: false }
  const result: DisplayRow[] = []
  for (const row of [root, ...props.rows]) {
    result.push(row)
    if (props.editing?.mode === 'create' && props.editing.path === row.path) {
      result.push({ ...row, path: `\0create:${row.path}`, depth: row.depth + 1, creating: true, loading: false, error: '', empty: false })
    }
  }
  return result
})
function isEditing(row: DisplayRow): boolean {
  return !!row.creating || (props.editing?.mode === 'rename' && props.editing.path === row.path)
}
function activate(row: SourceTreeRow): void {
  if (row.kind === 'directory') {
    emit('selectDirectory', row.path)
    if (row.path) {
      emit('toggle', row.path)
    }
  }
  else if (row.kind === 'file') {
    emit('select', row.path)
  }
}
function directoryFor(row: SourceTreeRow): string {
  return row.kind === 'directory' ? row.path : row.path.split('/').slice(0, -1).join('/')
}
function canDrop(row: SourceTreeRow): boolean {
  const entry = dragged.value
  if (!entry || props.disabled || props.editing || row.kind !== 'directory' || directoryFor(entry) === row.path) {
    return false
  }
  const source = entry.path.toLowerCase()
  const target = row.path.toLowerCase()
  return target !== source && !target.startsWith(`${source}/`)
}
function dragStart(event: DragEvent, row: SourceTreeRow): void {
  if (!props.editable || props.disabled || props.editing || !row.path || row.kind === 'blocked' || !event.dataTransfer) {
    event.preventDefault()
    return
  }
  dragged.value = row
  event.dataTransfer.setData(dragType, row.path)
  event.dataTransfer.effectAllowed = 'move'
  emit('focus', row)
}
function dragOver(event: DragEvent, row: SourceTreeRow): void {
  event.preventDefault()
  const allowed = canDrop(row) && event.dataTransfer?.types.includes(dragType)
  dropTarget.value = allowed ? row.path : undefined
  if (event.dataTransfer) {
    event.dataTransfer.dropEffect = allowed ? 'move' : 'none'
  }
}
function dragEnd(): void {
  dragged.value = undefined
  dropTarget.value = undefined
}
function drop(event: DragEvent, row: SourceTreeRow): void {
  event.preventDefault()
  if (canDrop(row) && dragged.value && event.dataTransfer?.types.includes(dragType)) {
    emit('move', dragged.value, row.path)
  }
  dragEnd()
}
watch(() => [props.disabled, props.editing], dragEnd)
function menuClosed(event: Event): void {
  if (props.editing) {
    event.preventDefault()
  }
}
</script>

<template>
  <nav aria-label="源码文件" class="min-h-0 overflow-auto py-2 text-xs" @dragover.prevent @drop.prevent="dragEnd">
    <p v-if="loading" role="status" class="flex items-center gap-2 px-3 py-2 text-muted-foreground">
      <LoaderCircle class="size-3.5 animate-spin" />正在读取源码目录…
    </p>
    <div v-if="error" class="flex flex-col gap-2 px-3 py-2">
      <p role="alert" class="break-words text-destructive">
        {{ error }}
      </p><Button type="button" variant="link" size="xs" class="h-auto p-0" @click="emit('retry', '')">
        重试
      </Button>
    </div>
    <template v-for="row in displayRows" :key="row.path">
      <div v-if="isEditing(row) && editing" class="flex items-start" :style="{ paddingLeft: `${Math.max(0, row.depth) * 14 + 28}px` }">
        <SourceNameInput :key="`${editing.mode}:${editing.path}`" :name="editing.name" :directory="editing.directory" :disabled="!!disabled" :pending="!!pending" :error="editError ?? ''" @update:name="emit('editName', $event)" @submit="emit('submit', $event)" @cancel="emit('cancel')" />
      </div>
      <ContextMenu v-else>
        <ContextMenuTrigger as-child :disabled="!editable || row.kind === 'blocked'">
          <Button
            variant="ghost" size="sm"
            :class="cn('w-full min-w-0 justify-start gap-1 rounded-none pr-2', selected === row.path && 'bg-accent text-accent-foreground', dropTarget === row.path && 'ring-2 ring-inset ring-ring')"
            :style="{ paddingLeft: `${Math.max(0, row.depth) * 14 + 8}px` }"
            :title="row.kind === 'blocked' ? `${row.path}（链接或特殊文件不可访问）` : row.path || 'source'"
            :disabled="row.kind === 'blocked'" :aria-current="selected === row.path ? 'true' : undefined"
            :aria-expanded="row.kind === 'directory' ? row.expanded : undefined"
            :draggable="!!editable && !disabled && !editing && !!row.path && row.kind !== 'blocked'"
            @click="activate(row)" @contextmenu="row.kind !== 'blocked' && emit('focus', row)"
            @keydown.f2.prevent="editable && !disabled && emit('rename', row)"
            @dragstart="dragStart($event, row)" @dragend="dragEnd"
            @dragover.stop="dragOver($event, row)" @dragleave="dropTarget = undefined" @drop.stop="drop($event, row)"
          >
            <ChevronRight v-if="row.kind === 'directory' && row.path" :class="row.expanded && 'rotate-90'" />
            <span v-else class="w-4 shrink-0" />
            <LoaderCircle v-if="row.loading" class="animate-spin" /><FolderOpen v-else-if="row.kind === 'directory' && row.expanded" /><Folder v-else-if="row.kind === 'directory'" /><LockKeyhole v-else-if="row.kind === 'blocked'" /><FileCode v-else />
            <span class="truncate font-mono">{{ row.name }}</span>
          </Button>
        </ContextMenuTrigger>
        <ContextMenuContent v-if="editable" class="w-44" @close-auto-focus="menuClosed">
          <ContextMenuGroup>
            <ContextMenuItem :disabled="disabled || !!editing" @select="emit('import', directoryFor(row))">
              <Upload />导入文件
            </ContextMenuItem>
            <ContextMenuItem :disabled="disabled" @select="emit('create', directoryFor(row))">
              <FolderPlus />新建文件夹
            </ContextMenuItem>
            <ContextMenuItem v-if="row.path" :disabled="disabled" @select="emit('rename', row)">
              <Pencil />重命名
            </ContextMenuItem>
          </ContextMenuGroup>
        </ContextMenuContent>
      </ContextMenu>
      <div v-if="!row.creating && row.expanded && row.error" :style="{ paddingLeft: `${row.depth * 14 + 30}px` }" class="flex flex-col gap-1 py-2 pr-3">
        <p role="alert" class="break-words text-destructive">
          {{ row.error }}
        </p><Button type="button" variant="link" size="xs" class="h-auto p-0" @click="emit('retry', row.path)">
          重试
        </Button>
      </div>
      <p v-else-if="!row.creating && row.expanded && row.empty && editing?.path !== row.path" :style="{ paddingLeft: `${row.depth * 14 + 30}px` }" class="py-1 text-muted-foreground">
        空文件夹
      </p>
    </template>
    <p v-if="!loading && !error && !rows.length && !editing" class="px-3 py-4 text-muted-foreground">
      源码目录为空。
    </p>
  </nav>
</template>
