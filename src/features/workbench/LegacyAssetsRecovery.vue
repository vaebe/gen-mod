<script setup lang="ts">
import { onScopeDispose, shallowRef, watch } from 'vue'
import { Button } from '@/components/ui/button'
import { Dialog, DialogContent, DialogDescription, DialogHeader, DialogTitle } from '@/components/ui/dialog'
import { nativeCall } from '@/services/native'

const props = defineProps<{ modId: string, blockedReason: string }>()
const emit = defineEmits<{ restored: [path: string] }>()
const open = defineModel<boolean>('open', { required: true })
const assets = shallowRef<{ assetId: string, originalName: string, inputAssetId?: string | null, restored: boolean }[]>([])
const busy = shallowRef(false)
const error = shallowRef('')
let generation = 0
onScopeDispose(() => {
  generation += 1
})
watch(() => [props.modId, open.value], async () => {
  const epoch = ++generation
  assets.value = []
  error.value = ''
  busy.value = false
  if (!props.modId || !open.value) {
    return
  }
  busy.value = true
  try {
    const result = await nativeCall<typeof assets.value>('list_legacy_assets', { modId: props.modId })
    if (epoch === generation) {
      assets.value = result.filter(asset => !asset.restored)
    }
  }
  catch (cause) {
    if (epoch === generation) {
      error.value = (cause as Error).message
    }
  }
  finally {
    if (epoch === generation) {
      busy.value = false
    }
  }
})
async function restore(assetId: string): Promise<void> {
  if (busy.value || props.blockedReason) {
    return
  }
  const epoch = generation
  busy.value = true
  error.value = ''
  try {
    const result = await nativeCall<{ assetId: string, sourcePath: string, status: 'restored' | 'alreadyPresent' | 'conflict' }>('restore_legacy_asset', { modId: props.modId, assetId })
    if (epoch === generation) {
      if (result.status === 'conflict') {
        error.value = `恢复目标已有不同内容，未覆盖：${result.sourcePath}`
        return
      }
      emit('restored', result.sourcePath)
      open.value = false
    }
  }
  catch (cause) {
    if (epoch === generation) {
      error.value = (cause as Error).message
    }
  }
  finally {
    if (epoch === generation) {
      busy.value = false
    }
  }
}
</script>

<template>
  <Dialog v-model:open="open">
    <DialogContent>
      <DialogHeader>
        <DialogTitle>恢复旧素材</DialogTitle>
        <DialogDescription>选择旧版本保留的文件，原样复制到 source/assets/legacy。原件保留，同名不同内容不覆盖。</DialogDescription>
      </DialogHeader>
      <p v-if="error || blockedReason" role="alert" class="text-sm text-destructive">
        {{ error || blockedReason }}
      </p>
      <p v-if="busy" role="status" class="text-sm text-muted-foreground">
        处理中…
      </p>
      <p v-else-if="!assets.length && !error" class="text-sm text-muted-foreground">
        此项目没有旧素材。
      </p>
      <ul class="flex max-h-[50vh] flex-col gap-3 overflow-auto">
        <li v-for="asset in assets" :key="asset.assetId" class="flex items-center justify-between gap-3">
          <span class="min-w-0 truncate text-sm">{{ asset.originalName }}{{ asset.inputAssetId ? '（历史派生文件）' : '' }}</span>
          <Button variant="outline" size="sm" :disabled="busy || !!blockedReason" @click="restore(asset.assetId)">
            原样恢复
          </Button>
        </li>
      </ul>
    </DialogContent>
  </Dialog>
</template>
