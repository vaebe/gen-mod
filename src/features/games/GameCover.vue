<script setup lang="ts">
import { shallowRef, watch } from 'vue'
import { gameService } from '@/features/games/service'

const props = defineProps<{ gameId: string, name: string }>()
const url = shallowRef('')
const error = shallowRef('')

watch(() => props.gameId, async (gameId, _previous, onCleanup) => {
  let current = true
  let objectUrl = ''
  url.value = ''
  error.value = ''
  onCleanup(() => {
    current = false
    if (objectUrl) {
      URL.revokeObjectURL(objectUrl)
    }
  })
  try {
    const bytes = await gameService.cover(gameId)
    if (current) {
      objectUrl = URL.createObjectURL(new Blob([bytes], { type: 'image/jpeg' }))
      url.value = objectUrl
    }
  }
  catch (cause) {
    if (current) {
      error.value = (cause as Error).message
    }
  }
}, { immediate: true })
</script>

<template>
  <div class="flex aspect-[460/215] w-24 shrink-0 items-center justify-center overflow-hidden rounded bg-muted">
    <span v-if="error" role="alert" class="p-1 text-xs text-destructive">{{ error }}</span>
    <img v-else-if="url" :src="url" :alt="`${name}封面`" class="size-full object-cover" @error="error = '游戏封面损坏，无法显示'">
    <span v-else class="text-xs text-muted-foreground">读取封面…</span>
  </div>
</template>
