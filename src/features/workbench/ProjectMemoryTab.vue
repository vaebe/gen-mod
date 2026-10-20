<script setup lang="ts">
import { defineAsyncComponent } from 'vue'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'
import { ScrollArea } from '@/components/ui/scroll-area'
import { useProjectMemory } from './useProjectMemory'

const props = defineProps<{ modId: string, revision: number, active: boolean, readOnly?: boolean }>()
const GameInspectionPanel = defineAsyncComponent(() => import('../mods/GameInspectionPanel.vue'))
const { memory, archived, error, loading, refresh } = useProjectMemory(() => props.modId, () => props.active, () => props.revision)
</script>

<template>
  <ScrollArea class="min-h-0 flex-1">
    <div class="flex flex-col gap-3 p-3">
      <p class="text-xs leading-5 text-muted-foreground">
        只属于当前项目。AI 自动记录明确需求和工具事实，可在聊天中更正；当前用户指令优先。
      </p>
      <p v-if="archived" role="status" class="text-sm text-muted-foreground">
        项目已归档，仅可查看。恢复后可继续制作和更新记忆。
      </p>
      <Button variant="outline" size="sm" :disabled="loading || !modId" @click="refresh">
        {{ loading ? '读取中…' : '刷新记忆' }}
      </Button>
      <p v-if="error" role="alert" class="text-sm text-destructive">
        {{ error }}
      </p>
      <template v-if="memory">
        <Card>
          <CardHeader><CardTitle>已确认需求</CardTitle></CardHeader>
          <CardContent>
            <ul v-if="memory.confirmedRequirements.length" class="flex list-disc flex-col gap-2 pl-4 text-sm">
              <li v-for="(item, index) in memory.confirmedRequirements" :key="index">
                {{ item }}
              </li>
            </ul>
            <p v-else class="text-sm text-muted-foreground">
              尚未记录明确需求。
            </p>
          </CardContent>
        </Card>
        <Card>
          <CardHeader><CardTitle>游戏与前置</CardTitle></CardHeader>
          <CardContent class="flex flex-col gap-3 text-sm">
            <p>
              游戏版本：{{ memory.gameVersion?.value ?? '未知' }}
            </p>
            <p v-for="(item, index) in memory.prerequisites" :key="index">
              {{ item.name }} · {{ item.version ?? '版本未知' }}
            </p>
            <p v-if="!memory.prerequisites.length" class="text-muted-foreground">
              尚未记录前置条件。
            </p>
          </CardContent>
        </Card>
        <Card>
          <CardHeader><CardTitle>参考资料</CardTitle></CardHeader>
          <CardContent class="flex flex-col gap-2 text-sm">
            <p v-for="item in memory.referenceSets" :key="item.id" class="break-all">
              {{ item.directory }} · {{ item.files.length }} 个参考文件
            </p>
            <p v-if="!memory.referenceSets.length" class="text-muted-foreground">
              尚未选择参考资料，按任务需要选择即可。
            </p>
          </CardContent>
        </Card>
        <div class="flex flex-wrap gap-2">
          <Badge variant="outline">
            第 {{ memory.revision }} 版记忆
          </Badge>
          <Badge v-for="(sources, field) in memory.fieldSources" :key="field" variant="secondary">
            {{ field === 'gameVersion' ? '游戏版本' : field === 'prerequisites' ? '前置条件' : '需求' }} · {{ sources?.length ?? 0 }} 条来源
          </Badge>
        </div>
        <GameInspectionPanel :mod-id="modId" :context-revision="memory.revision" :can-apply="!archived && !readOnly" @applied="refresh" />
      </template>
      <p v-else-if="!error && !loading" class="text-sm text-muted-foreground">
        选择项目后查看记忆。
      </p>
    </div>
  </ScrollArea>
</template>
