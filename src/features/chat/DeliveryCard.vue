<script setup lang="ts">
import type { ModProject } from '@/contracts/mod'
import { computed, defineAsyncComponent, shallowRef } from 'vue'
import { useRouter } from 'vue-router'
import { Button } from '@/components/ui/button'
import { Card, CardContent, CardFooter, CardHeader, CardTitle } from '@/components/ui/card'

const props = defineProps<{ project: ModProject, versionId: string }>()
const emit = defineEmits<{ targetSelected: [target: { modId: string, directory: string }] }>()
const InstallDialog = defineAsyncComponent(() => import('@/features/delivery/InstallDialog.vue'))
const ArtifactPanel = defineAsyncComponent(() => import('@/features/delivery/ArtifactPanel.vue'))
const installing = shallowRef(false)
const details = shallowRef(false)
const router = useRouter()
async function repair(record: import('@/features/verification/types').VerificationRecord): Promise<void> {
  details.value = false
  await router.push({ path: `/mods/${record.modId}/chat`, query: { repair: record.recordId } })
}
const version = computed(() => props.project.versions.find(version => version.versionId === props.versionId))
function rememberTarget(directory: string): void {
  emit('targetSelected', { modId: props.project.modId, directory })
}
</script>

<template>
  <Card>
    <CardHeader>
      <CardTitle>交付版本 {{ versionId.slice(0, 8) }}</CardTitle>
    </CardHeader>
    <CardContent>
      <p v-if="version" class="whitespace-pre-wrap break-words text-sm">
        {{ version.summary }}
      </p>
      <p v-else class="text-sm text-muted-foreground">
        此版本已删除，无法查看或安装。
      </p>
    </CardContent>
    <CardFooter v-if="version" class="gap-2">
      <Button :disabled="project.archived" @click="installing = true">
        安装此版本
      </Button>
      <Button variant="outline" @click="details = true">
        查看说明
      </Button>
    </CardFooter>
  </Card>
  <InstallDialog v-if="version && installing" v-model:open="installing" :mod-id="project.modId" :version-id="versionId" :previous-target="project.lastDeploymentDirectory" @target-selected="rememberTarget" />
  <ArtifactPanel v-if="version && details" v-model:open="details" :mod-id="project.modId" :version="version" @repair="repair" />
</template>
