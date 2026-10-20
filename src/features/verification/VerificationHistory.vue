<script setup lang="ts">
import type { VerificationRecord } from '@/contracts/verification'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'

defineProps<{ records: VerificationRecord[], readOnly?: boolean }>()
defineEmits<{ repair: [record: VerificationRecord] }>()
const labels = { passed: '通过', partial: '部分通过', failed: '失败', unknown: '未确认' }
</script>

<template>
  <section class="flex flex-col gap-3" aria-label="玩家验收历史">
    <p v-if="!records.length" class="text-xs text-muted-foreground">
      尚无玩家确认的验收记录。
    </p>
    <article v-for="record in records" :key="record.recordId" class="flex flex-col gap-2 rounded-md border p-3">
      <div class="flex flex-wrap items-center gap-2 text-xs">
        <Badge variant="outline">
          {{ labels[record.payload.result] }}
        </Badge>
        <span>{{ new Date(record.confirmedAt).toLocaleString() }}</span>
        <span class="font-mono">{{ record.versionId.slice(0, 8) }}</span>
      </div>
      <p class="whitespace-pre-wrap break-words text-sm">
        {{ record.payload.observations }}
      </p>
      <p v-if="record.supersedes" class="text-xs text-muted-foreground">
        更正记录 {{ record.supersedes }}
      </p>
      <p v-for="source in record.evidence" :key="`${source.ref.entryId}-${source.ref.attachmentIndex}`" class="break-words text-xs text-muted-foreground">
        {{ source.summary || '图片反馈' }} · 保存的证据摘要；原来源可能已不可打开
      </p>
      <Button size="sm" variant="outline" class="self-start" @click="$emit('repair', record)">
        {{ readOnly ? '在新会话继续修复' : '继续修复' }}
      </Button>
    </article>
  </section>
</template>
