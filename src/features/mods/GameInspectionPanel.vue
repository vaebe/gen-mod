<script setup lang="ts">
import type { InspectionReport } from '@/contracts/inspection'
import { onScopeDispose, shallowRef, watch } from 'vue'
import { Button } from '@/components/ui/button'
import { Collapsible, CollapsibleContent, CollapsibleTrigger } from '@/components/ui/collapsible'
import { ToggleGroup, ToggleGroupItem } from '@/components/ui/toggle-group'
import { nativeCall } from '@/services/native'

const props = defineProps<{ modId: string, contextRevision: number, canApply: boolean }>()
const emit = defineEmits<{ applied: [] }>()
const kind = shallowRef('references')
const factLabels: Record<string, string> = { engineMarker: '引擎标记', referenceAssembly: '参考程序集', loaderMarker: 'MOD 目录标记' }
const unknownLabels: Record<string, string> = { gameVersion: '游戏版本', apiCompleteness: 'API 完整性', runtimeBehavior: '游戏运行效果' }
const errorLabels: Record<string, string> = { permission: '无法读取', format: '路径或格式不支持', limit: '达到读取上限' }
const reports = shallowRef<InspectionReport[]>([])
const busy = shallowRef(false)
const error = shallowRef('')
let generation = 0
watch(() => props.modId, async (modId) => {
  const current = ++generation
  reports.value = []
  busy.value = false
  error.value = ''
  try {
    const value = await nativeCall<InspectionReport[]>('list_inspections', { modId })
    if (current === generation) {
      reports.value = value
    }
  }
  catch (cause) {
    if (current === generation) {
      error.value = (cause as Error).message
    }
  }
}, { immediate: true })
onScopeDispose(() => {
  generation += 1
})
async function inspect(): Promise<void> {
  if (!props.canApply || busy.value) {
    return
  }
  busy.value = true
  error.value = ''
  const current = generation
  try {
    const directory = await nativeCall<string | null>('pick_directory')
    if (!directory || current !== generation || !props.canApply) {
      return
    }
    const report = await nativeCall<InspectionReport>('inspect_supported_game', { modId: props.modId, kind: kind.value, directory })
    if (current === generation) {
      reports.value = [report, ...reports.value].slice(0, 20)
    }
  }
  catch (cause) {
    if (current === generation) {
      error.value = (cause as Error).message
    }
  }
  finally {
    if (current === generation) {
      busy.value = false
    }
  }
}
async function apply(reportId: string): Promise<void> {
  if (!props.canApply || busy.value) {
    return
  }
  const current = generation
  busy.value = true
  error.value = ''
  try {
    await nativeCall('apply_inspection', { modId: props.modId, reportId, expectedContextRevision: props.contextRevision })
    if (current === generation) {
      emit('applied')
    }
  }
  catch (cause) {
    if (current === generation) {
      error.value = (cause as Error).message
    }
  }
  finally {
    if (current === generation) {
      busy.value = false
    }
  }
}
</script>

<template>
  <Collapsible class="flex flex-col gap-2 rounded-md border p-3">
    <CollapsibleTrigger as-child>
      <Button variant="ghost" class="justify-start">
        检查所选目录
      </Button>
    </CollapsibleTrigger>
    <CollapsibleContent class="flex flex-col gap-3">
      <ToggleGroup type="single" :model-value="kind" :disabled="busy" @update:model-value="value => { if (value === 'install' || value === 'references') { kind = value } }">
        <ToggleGroupItem value="references">
          独立参考目录
        </ToggleGroupItem><ToggleGroupItem value="install">
          游戏安装目录
        </ToggleGroupItem>
      </ToggleGroup>
      <Button variant="outline" :disabled="busy || !canApply" @click="inspect">
        {{ busy ? '处理中…' : '选择目录并检查' }}
      </Button>
      <p class="text-xs text-muted-foreground">
        按需选择参考资料或检查游戏目录；只检查已支持游戏的已知事实。普通文件导入不经过此流程。
      </p>
      <article v-for="report in reports" :key="report.reportId" class="flex flex-col gap-1 border-t pt-2 text-xs">
        <p>{{ new Date(report.inspectedAt).toLocaleString() }} · {{ report.kind === 'references' ? '参考目录' : '安装目录' }}</p>
        <p v-if="report.truncated">
          读取达到上限，报告不完整。
        </p>
        <p v-for="finding in report.findings" :key="finding.relativePath" class="break-all">
          {{ finding.relativePath }} · {{ factLabels[finding.fact] }}
        </p>
        <p>未知项：{{ report.unknowns.map(item => unknownLabels[item] ?? item.replace('missing:', '缺少：')).join('、') }}</p>
        <p v-for="item in report.errors" :key="item.relativePath + item.code">
          {{ item.relativePath }} · {{ errorLabels[item.code] }}
        </p>
        <Button size="sm" variant="outline" :disabled="busy || !canApply || !report.findings.some(item => item.fact === 'referenceAssembly')" @click="apply(report.reportId)">
          采纳参考文件
        </Button>
      </article>
      <p v-if="error" role="alert" class="text-xs text-destructive">
        {{ error }}
      </p>
    </CollapsibleContent>
  </Collapsible>
</template>
