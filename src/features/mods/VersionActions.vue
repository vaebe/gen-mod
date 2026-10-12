<script setup lang="ts">
import { Download, Eye, FolderOpen, MoreHorizontal, PackageCheck, Trash2 } from '@lucide/vue'
import { Button } from '@/components/ui/button'
import { DropdownMenu, DropdownMenuContent, DropdownMenuGroup, DropdownMenuItem, DropdownMenuSeparator, DropdownMenuTrigger } from '@/components/ui/dropdown-menu'

export type VersionAction = 'details' | 'install' | 'export' | 'open' | 'delete'
defineProps<{ versionId: string, busy: boolean }>()
defineEmits<{ action: [kind: VersionAction, id: string] }>()
const actions = [
  { kind: 'details', label: '查看详情', icon: Eye },
  { kind: 'install', label: '安装此版本', icon: PackageCheck },
  { kind: 'export', label: '导出 ZIP', icon: Download },
  { kind: 'open', label: '打开版本目录', icon: FolderOpen },
  { kind: 'delete', label: '删除版本', icon: Trash2 },
] as const
</script>

<template>
  <DropdownMenu>
    <DropdownMenuTrigger as-child>
      <Button variant="ghost" size="icon" class="size-7 justify-self-end" :disabled="busy" :aria-label="`版本 ${versionId.slice(0, 8)} 的操作`">
        <MoreHorizontal />
      </Button>
    </DropdownMenuTrigger>
    <DropdownMenuContent align="end" :side-offset="5" class="min-w-40">
      <DropdownMenuGroup>
        <template v-for="action in actions" :key="action.kind">
          <DropdownMenuSeparator v-if="action.kind === 'delete'" />
          <DropdownMenuItem :disabled="busy" :class="action.kind === 'delete' && 'text-destructive'" @select="$emit('action', action.kind, versionId)">
            <component :is="action.icon" />{{ action.label }}
          </DropdownMenuItem>
        </template>
      </DropdownMenuGroup>
    </DropdownMenuContent>
  </DropdownMenu>
</template>
