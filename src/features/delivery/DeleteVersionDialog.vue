<script setup lang="ts">
import { Button } from '@/components/ui/button'
import { Dialog, DialogContent, DialogDescription, DialogHeader, DialogTitle } from '@/components/ui/dialog'

defineProps<{ versionId: string, installed: boolean, busy: boolean }>()
defineEmits<{ confirm: [] }>()
const open = defineModel<boolean>('open', { default: false })
</script>

<template>
  <Dialog :open="open" @update:open="value => !busy && (open = value)">
    <DialogContent @interact-outside="busy && $event.preventDefault()" @escape-key-down="busy && $event.preventDefault()">
      <DialogHeader><DialogTitle>删除版本 {{ versionId.slice(0, 8) }}</DialogTitle><DialogDescription>删除应用内的版本文件和 ZIP，无法撤销。已部署到游戏目录的文件、源码及安装记录会保留。</DialogDescription></DialogHeader>
      <p v-if="installed" class="text-sm text-muted-foreground">
        此版本仍有已安装记录；删除后仍可继续使用已部署的文件。
      </p>
      <div class="flex justify-end gap-2">
        <Button variant="ghost" :disabled="busy" @click="open = false">
          取消
        </Button><Button variant="destructive" :disabled="busy" @click="$emit('confirm')">
          {{ busy ? '正在删除…' : '删除版本' }}
        </Button>
      </div>
    </DialogContent>
  </Dialog>
</template>
