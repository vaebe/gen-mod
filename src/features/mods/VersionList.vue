<script setup lang="ts">
import type { ArtifactVersion } from '@/contracts/mod'
import { Check, Copy, Download, Eye, FolderOpen, MoreHorizontal, PackageCheck, Trash2 } from '@lucide/vue'
import { computed, onBeforeUnmount, shallowRef } from 'vue'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import { DropdownMenu, DropdownMenuContent, DropdownMenuGroup, DropdownMenuItem, DropdownMenuSeparator, DropdownMenuTrigger } from '@/components/ui/dropdown-menu'
import { Empty, EmptyDescription, EmptyHeader, EmptyTitle } from '@/components/ui/empty'

export type VersionAction = 'details' | 'install' | 'export' | 'open' | 'delete'
const props = defineProps<{ versions: ArtifactVersion[], latest?: string, installed: string[], busy: boolean }>()
defineEmits<{ action: [kind: VersionAction, id: string] }>()
const versionsNewestFirst = computed(() => [...props.versions].reverse())
const actions = [
  { kind: 'details', label: '查看详情', icon: Eye },
  { kind: 'install', label: '安装此版本', icon: PackageCheck },
  { kind: 'export', label: '导出 ZIP', icon: Download },
  { kind: 'open', label: '打开版本目录', icon: FolderOpen },
  { kind: 'delete', label: '删除版本', icon: Trash2 },
] as const
const copiedId = shallowRef('')
const copyError = shallowRef('')
let copyTimer: ReturnType<typeof setTimeout> | undefined
async function copyVersion(id: string): Promise<void> {
  copyError.value = ''
  try {
    await navigator.clipboard.writeText(id)
    copiedId.value = id
    clearTimeout(copyTimer)
    copyTimer = setTimeout(() => {
      copiedId.value = ''
    }, 2000)
  }
  catch (cause) { copyError.value = `复制失败：${cause instanceof Error ? cause.message : String(cause)}` }
}
function shortDate(value: string): string {
  return new Date(value).toLocaleString('zh-CN', { month: '2-digit', day: '2-digit', hour: '2-digit', minute: '2-digit', hour12: false })
}
onBeforeUnmount(() => clearTimeout(copyTimer))
</script>

<template>
  <section class="version-list flex min-h-0 flex-1 flex-col overflow-hidden rounded-lg border" aria-label="交付版本列表">
    <p v-if="copyError" role="alert" class="shrink-0 border-b px-3 py-2 text-xs text-destructive">
      {{ copyError }}
    </p>
    <div class="version-row shrink-0 border-b bg-muted/40 px-3 py-2 text-xs text-muted-foreground">
      <span>版本</span><span class="version-date">生成时间</span><span class="version-files">文件数</span><span>状态</span><span class="text-right">操作</span>
    </div>
    <div class="min-h-0 flex-1 overflow-y-auto">
      <Empty v-if="!versions.length">
        <EmptyHeader><EmptyTitle>尚无成功交付版本</EmptyTitle><EmptyDescription>生成完成并通过文件校验后会显示在这里。</EmptyDescription></EmptyHeader>
      </Empty>
      <div v-for="version in versionsNewestFirst" :key="version.versionId" class="version-row min-h-13 border-b px-3 py-2 last:border-b-0 hover:bg-muted/30">
        <div class="min-w-0">
          <Button type="button" variant="ghost" size="xs" class="max-w-full" :aria-label="`复制版本号 ${version.versionId}`" :title="copiedId === version.versionId ? '已复制' : `复制完整版本号 ${version.versionId}`" @click="copyVersion(version.versionId)">
            <span class="font-mono">{{ version.versionId.slice(0, 8) }}</span><Check v-if="copiedId === version.versionId" class="size-3 text-muted-foreground" aria-hidden="true" /><Copy v-else class="size-3 text-muted-foreground" aria-hidden="true" />
          </Button>
          <span v-if="copiedId === version.versionId" class="sr-only" role="status">版本号已复制</span>
          <p class="version-mobile-meta mt-1 text-[11px] text-muted-foreground" :title="new Date(version.createdAt).toLocaleString()">
            {{ shortDate(version.createdAt) }} · {{ version.files.length }} 个文件
          </p>
        </div>
        <span class="version-date text-xs text-muted-foreground" :title="new Date(version.createdAt).toLocaleString()">{{ shortDate(version.createdAt) }}</span>
        <span class="version-files text-xs text-muted-foreground">{{ version.files.length }}</span>
        <div class="flex flex-wrap items-center gap-1">
          <Badge v-if="latest === version.versionId" variant="secondary">
            最新
          </Badge>
          <Badge v-if="installed.includes(version.versionId)" variant="outline">
            已安装
          </Badge>
        </div>
        <DropdownMenu>
          <DropdownMenuTrigger as-child>
            <Button variant="ghost" size="icon" class="size-7 justify-self-end" :disabled="busy" :aria-label="`版本 ${version.versionId.slice(0, 8)} 的操作`">
              <MoreHorizontal class="size-4" />
            </Button>
          </DropdownMenuTrigger>
          <DropdownMenuContent align="end" :side-offset="5" class="min-w-40">
            <DropdownMenuGroup>
              <template v-for="action in actions" :key="action.kind">
                <DropdownMenuSeparator v-if="action.kind === 'delete'" />
                <DropdownMenuItem :disabled="busy" :class="action.kind === 'delete' && 'text-destructive'" @select="$emit('action', action.kind, version.versionId)">
                  <component :is="action.icon" class="size-4" />{{ action.label }}
                </DropdownMenuItem>
              </template>
            </DropdownMenuGroup>
          </DropdownMenuContent>
        </DropdownMenu>
      </div>
    </div>
  </section>
</template>

<style scoped>
.version-list {
  container-type: inline-size;
}
.version-date,
.version-files {
  display: none;
}
.version-row {
  display: grid;
  grid-template-columns: minmax(0, 1fr) 100px 28px;
  align-items: center;
  gap: 0.75rem;
}
@container (min-width: 560px) {
  .version-row {
    grid-template-columns: minmax(0, 1fr) 130px 56px 110px 28px;
  }
  .version-date,
  .version-files {
    display: block;
  }
  .version-mobile-meta {
    display: none;
  }
}
</style>
