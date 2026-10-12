<script setup lang="ts">
import type { FileDiff } from '@/contracts/changes'
import { shallowRef } from 'vue'
import { Button } from '@/components/ui/button'
import { nativeCall } from '@/services/native'

const props = defineProps<{ diff: FileDiff, modId: string }>()
const error = shallowRef('')
async function locate(): Promise<void> {
  try {
    await nativeCall('reveal_source_file', { modId: props.modId, relativePath: props.diff.change.path })
  }
  catch (cause) { error.value = (cause as Error).message }
}
async function copy(): Promise<void> {
  try {
    await navigator.clipboard.writeText(props.diff.after ?? props.diff.before ?? '')
  }
  catch (cause) { error.value = (cause as Error).message }
}
</script>

<template>
  <section class="mt-4 border-t">
    <div class="flex flex-wrap items-center gap-2 px-3 py-3">
      <span class="mr-auto break-all font-mono text-xs">{{ diff.change.path }}</span><Button variant="ghost" size="sm" :disabled="!diff.change.textAvailable" @click="copy">
        复制
      </Button><Button variant="ghost" size="sm" :disabled="diff.change.kind === 'deleted'" @click="locate">
        定位
      </Button>
    </div>
    <p v-if="!diff.change.textAvailable" class="px-3 pb-4 text-xs leading-6 text-muted-foreground">
      二进制或大于 1 MiB 的文件不展示全文。原大小 {{ diff.change.beforeSize ?? 0 }} B，当前 {{ diff.change.afterSize ?? 0 }} B。
    </p>
    <div v-else class="overflow-x-auto pb-4">
      <pre v-for="(line, index) in diff.lines" :key="index" class="px-3 font-mono text-[11px] leading-5" :class="line.kind === 'insert' ? 'bg-green-500/10 text-green-800' : line.kind === 'delete' ? 'bg-red-500/10 text-red-800' : 'text-muted-foreground'">{{ line.kind === 'insert' ? '+' : line.kind === 'delete' ? '−' : ' ' }}{{ line.text }}</pre>
    </div>
    <p v-if="error" role="alert" class="px-3 pb-3 text-xs text-destructive">
      {{ error }}
    </p>
  </section>
</template>
