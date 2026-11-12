<script setup lang="ts">
import { Folder, FolderOpen } from '@lucide/vue'
import { shallowRef, watch } from 'vue'
import { gameService } from '@/features/games/service'

const props = defineProps<{ gameId: string, expanded?: boolean }>()
const url = shallowRef('')

watch(() => props.gameId, async (gameId, _previous, onCleanup) => {
  let current = true
  let objectUrl = ''
  url.value = ''
  onCleanup(() => {
    current = false
    if (objectUrl) {
      URL.revokeObjectURL(objectUrl)
    }
  })
  if (!gameId) {
    return
  }
  try {
    const bytes = await gameService.icon(gameId)
    if (current) {
      objectUrl = URL.createObjectURL(new Blob([bytes], { type: 'image/jpeg' }))
      url.value = objectUrl
    }
  }
  catch {
    // 游戏图标缺失时保留项目文件夹入口。
  }
}, { immediate: true })
</script>

<template>
  <span class="inline-flex size-4 shrink-0 items-center justify-center text-muted-foreground" aria-hidden="true">
    <img v-if="url" :src="url" alt="" class="size-full rounded-sm object-contain" @error="url = ''">
    <FolderOpen v-else-if="expanded" class="size-full" />
    <Folder v-else class="size-full" />
  </span>
</template>
