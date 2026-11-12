<script setup lang="ts">
import type { MemorySaveResult } from './types'
import { useRouter } from 'vue-router'
import { Button } from '@/components/ui/button'

defineProps<{ result: MemorySaveResult }>()
const router = useRouter()
</script>

<template>
  <div v-if="result.outcome !== 'unchanged'" class="my-2 flex flex-wrap items-center gap-2 rounded-md border p-3 text-sm" role="status">
    <span>{{ result.outcome === 'saved' ? `已保存长期记忆：${result.title}` : '已保留用户维护的记忆，AI 未覆盖。' }}</span>
    <Button variant="outline" size="sm" @click="router.push({ path: '/settings', query: { tab: 'memory', memoryId: result.noteId } })">
      查看、修改或删除
    </Button>
  </div>
</template>
