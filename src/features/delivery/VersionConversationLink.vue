<script setup lang="ts">
import type { ArtifactVersion } from '@/contracts/delivery'
import { onScopeDispose, shallowRef } from 'vue'
import { useRouter } from 'vue-router'
import { Button } from '@/components/ui/button'
import { modService } from '@/features/projects/service'

const props = defineProps<{ modId: string, version: ArtifactVersion }>()
const emit = defineEmits<{ navigated: [] }>()
const router = useRouter()
const busy = shallowRef(false)
const error = shallowRef('')
let active = true
onScopeDispose(() => {
  active = false
})
async function navigate(): Promise<void> {
  if (busy.value) {
    return
  }
  busy.value = true
  error.value = ''
  const modId = props.modId
  const version = props.version
  try {
    const project = await modService.get(modId)
    const sessionId = version.sessionId
    if (!project.sessions.some(session => session.sessionId === sessionId)) {
      throw new Error('生成此版本的会话已删除或无法找到，仍可使用此版本的其他操作')
    }
    if (!active || props.modId !== modId || props.version.versionId !== version.versionId) {
      return
    }
    await router.push({ path: `/mods/${modId}/chat`, query: { session: sessionId, task: version.taskId } })
    emit('navigated')
  }
  catch (cause) {
    if (props.modId === modId && props.version.versionId === version.versionId) {
      error.value = (cause as Error).message
    }
  }
  finally { busy.value = false }
}
</script>

<template>
  <div class="flex flex-col gap-1">
    <Button variant="link" size="sm" class="h-auto justify-start p-0" :disabled="busy" @click="navigate">
      {{ busy ? '正在查找…' : '返回生成对话' }}
    </Button>
    <p v-if="error" role="alert" class="break-words text-xs text-destructive">
      {{ error }}
    </p>
  </div>
</template>
