<script setup lang="ts">
import type { SourceTreeRow } from './useSourceBrowser'
import type { SourceAction, useSourceManager } from './useSourceManager'
import type { SourceEntry } from '@/contracts/source'
import { ChevronRight, LoaderCircle } from '@lucide/vue'
import { computed, defineAsyncComponent, onMounted, onScopeDispose, shallowRef, useTemplateRef, watch } from 'vue'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import { ContextMenu, ContextMenuContent, ContextMenuGroup, ContextMenuItem, ContextMenuTrigger } from '@/components/ui/context-menu'
import { Empty, EmptyHeader, EmptyTitle } from '@/components/ui/empty'
import { addSourceReferences } from './sourceWorkspace'

const props = defineProps<{ modId: string, rows: SourceTreeRow[], loading: boolean, error: string, disabled: boolean, editing: ReturnType<typeof useSourceManager>['editing']['value'], editError: string, statuses: Record<string, string> }>()
const emit = defineEmits<{ open: [path: string], toggle: [path: string], action: [action: SourceAction, target?: SourceEntry], move: [entries: SourceEntry[], directory: string], name: [name: string], submit: [name: string], cancel: [] }>()
const selection = defineModel<string[]>('selection', { required: true })
const SourceNameInput = defineAsyncComponent(() => import('./SourceNameInput.vue'))
const SourceFileIcon = defineAsyncComponent(() => import('./SourceFileIcon.vue'))
const tree = useTemplateRef<HTMLElement>('tree')
const focused = shallowRef('')
let suppressClick = false
const root: SourceEntry = { name: '', path: '', kind: 'directory' }
const menu = [
  ['file', '新建文件'],
  ['folder', '新建文件夹'],
  ['import', '导入文件'],
  ['importFolder', '导入文件夹'],
  ['copy', '复制'],
  ['cut', '剪切'],
  ['paste', '粘贴'],
  ['rename', '重命名'],
  ['delete', '移入回收站'],
  ['export', '导出'],
  ['reference', '添加到对话'],
  ['relative', '复制相对路径'],
  ['absolute', '复制完整路径'],
  ['open', '系统应用打开'],
  ['reveal', '在文件管理器中定位'],
  ['refresh', '刷新'],
] as const
const readActions = new Set<SourceAction>(['reference', 'relative', 'absolute', 'reveal', 'refresh'])
function available(entry: SourceEntry) {
  const multiple = selection.value.includes(entry.path) && selection.value.length > 1
  return menu.filter(([kind]) => {
    if (!entry.path) {
      return ['file', 'folder', 'import', 'importFolder', 'paste', 'reveal', 'refresh'].includes(kind)
    }
    if (multiple) {
      return ['copy', 'cut', 'delete', 'export', 'reference', 'refresh'].includes(kind)
    }
    return kind !== 'open' || entry.kind === 'file'
  })
}
const displayed = computed(() => {
  const rows: (SourceTreeRow & { creating?: boolean })[] = []
  const insert = (path: string, depth: number) => {
    if (props.editing && props.editing.mode !== 'rename' && props.editing.directory === path) {
      rows.push({ path: `\0new:${path}`, name: '', kind: 'file', depth, expanded: false, loading: false, error: '', empty: false, creating: true })
    }
  }
  insert('', 0)
  for (const row of props.rows) {
    rows.push(row)
    insert(row.path, row.depth + 1)
  }
  return rows
})

function choose(row: SourceTreeRow, event: MouseEvent): void {
  if (suppressClick || row.kind === 'blocked') {
    return
  }
  const previous = focused.value
  focused.value = row.path
  if (event.shiftKey && previous) {
    const paths = props.rows.filter(row => row.kind !== 'blocked').map(row => row.path)
    const start = paths.indexOf(previous)
    const end = paths.indexOf(row.path)
    selection.value = paths.slice(Math.min(start, end), Math.max(start, end) + 1)
  }
  else if (event.ctrlKey || event.metaKey) {
    selection.value = selection.value.includes(row.path) ? selection.value.filter(path => path !== row.path) : [...selection.value, row.path]
  }
  else {
    selection.value = [row.path]
    if (row.kind === 'directory') {
      emit('toggle', row.path)
    }
    else {
      emit('open', row.path)
    }
  }
}

function context(entry: SourceEntry): void {
  if (!entry.path) {
    selection.value = []
  }
  else if (!selection.value.includes(entry.path)) {
    selection.value = [entry.path]
  }
  focused.value = entry.path
}

function keydown(event: KeyboardEvent): void {
  if (event.target instanceof HTMLInputElement || props.editing || event.isComposing) {
    return
  }
  const row = props.rows.find(row => row.path === focused.value)
  const command = event.ctrlKey || event.metaKey
  const kind = command ? ({ c: 'copy', x: 'cut', v: 'paste' } as const)[event.key.toLowerCase() as 'c' | 'x' | 'v'] : event.key === 'Delete' ? 'delete' : event.key === 'F2' ? 'rename' : undefined
  if (kind) {
    event.preventDefault()
    if (kind === 'paste') {
      emit('action', kind, row ?? root)
    }
    else if (selection.value.length) {
      emit('action', kind)
    }
  }
  else if (command && event.key.toLowerCase() === 'a') {
    event.preventDefault()
    selection.value = props.rows.filter(row => row.kind !== 'blocked').map(row => row.path)
  }
  else if (['ArrowDown', 'ArrowUp', 'ArrowLeft', 'ArrowRight', 'Enter'].includes(event.key)) {
    event.preventDefault()
    if (event.key === 'ArrowDown' || event.key === 'ArrowUp') {
      const rows = props.rows.filter(row => row.kind !== 'blocked')
      const index = Math.max(0, Math.min(rows.length - 1, rows.findIndex(item => item.path === focused.value) + (event.key === 'ArrowDown' ? 1 : -1)))
      const target = rows[index]
      if (target) {
        focused.value = target.path
        if (!event.shiftKey) {
          selection.value = [target.path]
        }
        else {
          selection.value = [...new Set([...selection.value, target.path])]
        }
        focusRow(target.path)
      }
    }
    else if (row?.kind === 'directory' && (event.key === 'Enter' || (event.key === 'ArrowLeft' && row.expanded) || (event.key === 'ArrowRight' && !row.expanded))) {
      emit('toggle', row.path)
    }
    else if (row?.kind === 'file' && event.key === 'Enter') {
      emit('open', row.path)
    }
    else if (row && event.key === 'ArrowLeft') {
      const path = row.path.split('/').slice(0, -1).join('/')
      if (path) {
        selection.value = [path]
        focusRow(path)
      }
    }
    else if (row?.kind === 'directory' && row.expanded && event.key === 'ArrowRight') {
      const next = props.rows[props.rows.indexOf(row) + 1]
      if (next?.path.startsWith(`${row.path}/`)) {
        selection.value = [next.path]
        focusRow(next.path)
      }
    }
  }
}

function focusRow(path: string): void {
  focused.value = path
  tree.value?.querySelectorAll<HTMLElement>('[data-source-path]').forEach((element) => {
    if (element.dataset.sourcePath === path) {
      element.focus()
    }
  })
}

// 原生外部拖放与 HTML5 drag 在 Windows 上互斥；内部移动使用指针手势。
const dragging = shallowRef<{ x: number, y: number, entries: SourceEntry[], target?: string, chat?: boolean }>()
let start: { x: number, y: number, row: SourceTreeRow } | undefined
let hoverTimer: ReturnType<typeof setTimeout> | undefined
let hoverPath: string | undefined
function pointerDown(event: PointerEvent, row: SourceTreeRow): void {
  if (event.button !== 0 || row.kind === 'blocked' || props.editing) {
    return
  }
  start = { x: event.clientX, y: event.clientY, row }
}
function pointerMove(event: PointerEvent): void {
  if (!start || (Math.hypot(event.clientX - start.x, event.clientY - start.y) < 6 && !dragging.value)) {
    return
  }
  event.preventDefault()
  suppressClick = true
  if (!selection.value.includes(start.row.path)) {
    selection.value = [start.row.path]
  }
  const entries = props.rows.filter(row => selection.value.includes(row.path) && row.kind !== 'blocked')
  const hit = document.elementFromPoint(event.clientX, event.clientY)
  const chat = hit?.closest<HTMLElement>('[data-source-chat]')?.dataset.sourceChat === props.modId
  const destination = hit?.closest<HTMLElement>('[data-source-directory]')
  const sameTree = destination?.closest<HTMLElement>('[data-mod-tree]')?.dataset.modTree === props.modId
  const path = sameTree ? destination?.dataset.sourceDirectory : undefined
  const allowed = !props.disabled && path !== undefined && !entries.some(entry => entry.path === path || path.toLowerCase().startsWith(`${entry.path.toLowerCase()}/`))
  dragging.value = { x: event.clientX, y: event.clientY, entries, target: allowed ? path : undefined, chat }
  if (path !== hoverPath) {
    clearTimeout(hoverTimer)
    hoverPath = path
    const row = props.rows.find(row => row.path === path)
    if (allowed && row?.kind === 'directory' && !row.expanded) {
      hoverTimer = setTimeout(emit, 650, 'toggle', row.path)
    }
  }
  const bounds = tree.value?.getBoundingClientRect()
  if (bounds && event.clientX >= bounds.left && event.clientX <= bounds.right) {
    if (event.clientY < bounds.top + 30) {
      tree.value!.scrollTop -= 16
    }
    if (event.clientY > bounds.bottom - 30) {
      tree.value!.scrollTop += 16
    }
  }
}
function stop(): void {
  start = undefined
  dragging.value = undefined
  hoverPath = undefined
  clearTimeout(hoverTimer)
  setTimeout(() => {
    suppressClick = false
  }, 0)
}
function pointerUp(): void {
  const drag = dragging.value
  if (drag?.chat) {
    addSourceReferences(props.modId, drag.entries)
  }
  else if (drag?.target !== undefined) {
    emit('move', drag.entries, drag.target)
  }
  stop()
}
onMounted(() => {
  document.addEventListener('pointermove', pointerMove)
  document.addEventListener('pointerup', pointerUp)
  document.addEventListener('pointercancel', stop)
})
onScopeDispose(() => {
  stop()
  document.removeEventListener('pointermove', pointerMove)
  document.removeEventListener('pointerup', pointerUp)
  document.removeEventListener('pointercancel', stop)
})
watch(() => props.modId, stop)
watch(() => props.rows, (rows) => {
  selection.value = selection.value.filter(path => rows.some(row => row.path === path))
})
</script>

<template>
  <div ref="tree" role="tree" aria-label="MOD 工作文件" aria-multiselectable="true" :data-mod-tree="modId" class="flex h-full min-h-0 select-none flex-col overflow-auto text-xs" @keydown="keydown" @dragstart.prevent>
    <p v-if="loading" role="status" class="p-3 text-muted-foreground">
      正在读取文件…
    </p>
    <p v-if="error" role="alert" class="p-3 text-destructive">
      {{ error }} <Button variant="link" @click="emit('action', 'refresh')">
        重试
      </Button>
    </p>
    <template v-for="row in displayed" :key="row.path">
      <SourceNameInput v-if="editing && (row.creating || (editing.mode === 'rename' && editing.path === row.path))" :style="{ paddingLeft: `${row.depth * 14 + 8}px` }" :name="editing.name" :directory="editing.isDirectory" :disabled="disabled" :pending="disabled" :error="editError" @update:name="emit('name', $event)" @submit="emit('submit', $event)" @cancel="emit('cancel')" />
      <ContextMenu v-else>
        <ContextMenuTrigger as-child :disabled="row.kind === 'blocked'">
          <Button variant="ghost" size="sm" role="treeitem" :aria-selected="selection.includes(row.path)" :aria-expanded="row.kind === 'directory' ? row.expanded : undefined" :aria-level="row.depth + 1" :disabled="row.kind === 'blocked'" :data-source-path="row.path" :data-source-directory="row.kind === 'directory' ? row.path : undefined" :title="row.path" class="w-full shrink-0 justify-start gap-1 rounded-none pr-2" :class="[selection.includes(row.path) && 'bg-accent', dragging?.target === row.path && 'ring-2 ring-inset ring-ring']" :style="{ paddingLeft: `${row.depth * 14 + 8}px` }" @click="choose(row, $event)" @contextmenu="context(row)" @pointerdown="pointerDown($event, row)">
            <ChevronRight v-if="row.kind === 'directory'" :class="row.expanded && 'rotate-90'" @click.stop="emit('toggle', row.path)" />
            <LoaderCircle v-if="row.loading" class="animate-spin" /><SourceFileIcon v-else :name="row.name" :directory="row.kind === 'directory'" :expanded="row.expanded" />
            <span class="truncate">{{ row.name }}</span><Badge v-if="statuses[row.path]" variant="secondary" class="ml-auto">
              {{ statuses[row.path] }}
            </Badge>
          </Button>
        </ContextMenuTrigger>
        <ContextMenuContent>
          <ContextMenuGroup>
            <ContextMenuItem v-for="[kind, label] in available(row)" :key="kind" :disabled="disabled && !readActions.has(kind)" @select="emit('action', kind, row)">
              {{ label }}
            </ContextMenuItem>
          </ContextMenuGroup>
        </ContextMenuContent>
      </ContextMenu>
      <p v-if="row.error" role="alert" class="px-4 py-1 text-destructive">
        {{ row.error }}<Button variant="link" @click="emit('action', 'refresh')">
          重试
        </Button>
      </p>
      <p v-else-if="row.kind === 'directory' && row.expanded && row.empty" class="px-6 py-1 text-muted-foreground">
        空文件夹
      </p>
    </template>
    <ContextMenu>
      <ContextMenuTrigger as-child>
        <div data-source-directory="" class="min-h-24 flex-1" :class="dragging?.target === '' && 'ring-2 ring-inset ring-ring'" @click="selection = []" @contextmenu="context(root)">
          <Empty v-if="!rows.length && !loading && !error && !editing">
            <EmptyHeader><EmptyTitle>暂无文件</EmptyTitle></EmptyHeader><Button variant="outline" :disabled="disabled" @click.stop="emit('action', 'import', root)">
              导入文件
            </Button>
          </Empty>
        </div>
      </ContextMenuTrigger>
      <ContextMenuContent>
        <ContextMenuGroup>
          <ContextMenuItem v-for="[kind, label] in available(root)" :key="kind" :disabled="disabled && !readActions.has(kind)" @select="emit('action', kind, root)">
            {{ label }}
          </ContextMenuItem>
        </ContextMenuGroup>
      </ContextMenuContent>
    </ContextMenu>
    <Teleport to="body">
      <div v-if="dragging" class="pointer-events-none fixed rounded border bg-popover px-3 py-2 text-xs shadow" :style="{ left: `${dragging.x + 12}px`, top: `${dragging.y + 12}px` }">
        {{ dragging.entries.length }} 项 · {{ dragging.chat ? '添加到对话' : dragging.target !== undefined ? '移动' : '不可放置' }}
      </div>
    </Teleport>
  </div>
</template>
