<script setup lang="ts">
import type { VersionAction } from './VersionActions.vue'
import type { ArtifactVersion } from '@/contracts/delivery'
import type { VerificationRecord } from '@/features/verification/types'
import { Check, Copy } from '@lucide/vue'
import { computed, defineAsyncComponent, onBeforeUnmount, shallowRef } from 'vue'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import { Empty, EmptyDescription, EmptyHeader, EmptyTitle } from '@/components/ui/empty'
import VersionActions from './VersionActions.vue'

export type { VersionAction } from './VersionActions.vue'
const props = defineProps<{ modId: string, versions: ArtifactVersion[], latest?: string, installed: string[], busy: boolean, readOnly?: boolean, verificationRecords?: VerificationRecord[] }>()
defineEmits<{ action: [kind: VersionAction, id: string] }>()
const VersionConversationLink = defineAsyncComponent(() => import('./VersionConversationLink.vue'))
const versionsNewestFirst = computed(() => [...props.versions].reverse())
function verificationLabel(versionId: string): string {
  const record = props.verificationRecords?.find(item => item.versionId === versionId)
  return record ? { passed: '玩家确认通过', partial: '玩家确认部分通过', failed: '玩家确认失败', unknown: '玩家结果未确认' }[record.payload.result] : '待玩家验证'
}
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
          <p class="mt-1 whitespace-pre-wrap break-words text-sm">
            {{ version.summary }}
          </p>
          <VersionConversationLink :mod-id="modId" :version="version" />
          <span v-if="copiedId === version.versionId" class="sr-only" role="status">版本号已复制</span>
          <p class="version-mobile-meta mt-1 text-[11px] text-muted-foreground" :title="new Date(version.createdAt).toLocaleString()">
            {{ shortDate(version.createdAt) }} · {{ version.files.length }} 个文件
          </p>
        </div>
        <span class="version-date text-xs text-muted-foreground" :title="new Date(version.createdAt).toLocaleString()">{{ shortDate(version.createdAt) }}</span>
        <span class="version-files text-xs text-muted-foreground">{{ version.files.length }}</span>
        <div class="flex flex-wrap items-center gap-1">
          <Badge variant="outline">
            {{ verificationLabel(version.versionId) }}
          </Badge>
          <Badge v-if="latest === version.versionId" variant="secondary">
            最新
          </Badge>
          <Badge v-if="installed.includes(version.versionId)" variant="outline">
            已安装
          </Badge>
        </div>
        <VersionActions :version-id="version.versionId" :busy="busy" :read-only="readOnly" @action="(kind, id) => $emit('action', kind, id)" />
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
