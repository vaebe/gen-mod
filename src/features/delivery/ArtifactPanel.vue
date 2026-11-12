<script setup lang="ts">
import type { ArtifactVersion, DeliveryMetadata } from '@/contracts/delivery'
import { defineAsyncComponent, shallowRef, watch } from 'vue'
import { Dialog, DialogContent, DialogDescription, DialogHeader, DialogTitle } from '@/components/ui/dialog'
import { useVerification } from '@/features/verification/useVerification'
import { deliveryService } from './service'

const props = defineProps<{ version: ArtifactVersion, modId: string }>()
defineEmits<{ repair: [record: import('@/features/verification/types').VerificationRecord] }>()
const open = defineModel<boolean>('open', { default: false })
const VersionConversationLink = defineAsyncComponent(() => import('./VersionConversationLink.vue'))
const VerificationHistory = defineAsyncComponent(() => import('@/features/verification/VerificationHistory.vue'))
const metadata = shallowRef<DeliveryMetadata>()
const metadataError = shallowRef('')
const verification = useVerification(() => props.modId, () => Number(open.value), () => props.version.versionId)
watch(() => [props.modId, props.version.versionId], async () => {
  const id = props.version.versionId
  const modId = props.modId
  metadata.value = undefined
  metadataError.value = ''
  try {
    const value = await deliveryService.metadata(modId, id)
    if (props.modId === modId && props.version.versionId === id) {
      metadata.value = value[0]
      metadataError.value = ''
    }
  }
  catch (cause) {
    if (props.modId === modId && props.version.versionId === id) {
      metadataError.value = (cause as Error).message
    }
  }
}, { immediate: true })
function fileSize(bytes: number): string {
  if (bytes < 1024) {
    return `${bytes} B`
  }
  if (bytes < 1024 * 1024) {
    return `${(bytes / 1024).toFixed(1)} KB`
  }
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`
}
</script>

<template>
  <Dialog v-model:open="open">
    <DialogContent class="flex max-h-[calc(100dvh-3rem)] flex-col gap-3 overflow-hidden p-5 sm:max-w-2xl">
      <DialogHeader class="shrink-0 pr-6">
        <DialogTitle>版本详情 {{ version.versionId.slice(0, 8) }}</DialogTitle><DialogDescription>版本信息及交付文件清单。</DialogDescription>
      </DialogHeader>
      <div class="min-h-0 overflow-y-auto">
        <div class="mb-3 flex flex-col gap-2">
          <p class="whitespace-pre-wrap break-words text-sm">
            {{ version.summary }}
          </p>
          <VersionConversationLink :mod-id="modId" :version="version" @navigated="open = false" />
        </div>
        <dl class="grid grid-cols-[auto_minmax(0,1fr)] gap-x-4 gap-y-2 rounded-md bg-muted/40 p-3 text-xs">
          <dt class="text-muted-foreground">
            版本号
          </dt><dd class="break-all font-mono select-text">
            {{ version.versionId }}
          </dd>
          <dt class="text-muted-foreground">
            任务 ID
          </dt><dd class="break-all font-mono select-text">
            {{ version.taskId }}
          </dd>
          <dt class="text-muted-foreground">
            生成时间
          </dt><dd>{{ new Date(version.createdAt).toLocaleString() }}</dd>
          <dt class="text-muted-foreground">
            文件数量
          </dt><dd>{{ version.files.length }} 个文件</dd>
        </dl>
        <div class="mt-4 flex justify-between border-b pb-2 text-xs text-muted-foreground">
          <span>文件路径</span><span>大小</span>
        </div>
        <ul class="divide-y text-xs">
          <li v-for="file in version.files" :key="file.path" class="flex justify-between gap-4 py-2.5">
            <span class="min-w-0 break-all font-mono select-text">{{ file.path }}</span><span class="shrink-0 text-muted-foreground" :title="`${file.size} B`">{{ fileSize(file.size) }}</span>
          </li>
        </ul>
        <section class="mt-3 flex flex-col gap-2 text-xs">
          <p v-if="metadataError" role="alert" class="text-destructive">
            {{ metadataError }}
          </p>
          <p v-if="metadata?.checkReport">
            内容规则 {{ metadata.checkReport.rulesVersion }}：候选字节与受限参考指纹；不展开嵌套压缩包。
          </p>
          <p v-for="finding in metadata?.checkReport?.findings" :key="finding.path + finding.code">
            {{ finding.level }} · {{ finding.path }} · {{ finding.code }}
          </p>
          <p v-if="metadata?.checkReport && !metadata.checkReport.findings.length">
            本次规则范围内未发现命中项。
          </p>
          <p v-if="!metadata?.cases.length">
            此版本没有结构化验收步骤。
          </p>
          <p v-for="item in metadata?.cases" :key="item.id">
            {{ item.precondition }} → {{ item.action }} → {{ item.expected }}；证据：{{ item.evidenceHint }}
          </p>
          <VerificationHistory :records="verification.data.value.records" @repair="$emit('repair', $event)" />
        </section>
        <p class="mt-3 text-xs leading-5 text-muted-foreground">
          版本文件校验通过才可安装。交付成功不代表游戏运行效果已验证。
        </p>
      </div>
    </DialogContent>
  </Dialog>
</template>
