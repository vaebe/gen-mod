<script setup lang="ts">
import type { FeedbackDraft, FeedbackPayload, VerificationRecord, VerificationResult } from '@/features/verification/types'
import { shallowRef, watch } from 'vue'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import { Card, CardContent, CardDescription, CardFooter, CardHeader, CardTitle } from '@/components/ui/card'
import { Field, FieldGroup, FieldLabel } from '@/components/ui/field'
import { Input } from '@/components/ui/input'
import { Select, SelectContent, SelectGroup, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select'
import { Textarea } from '@/components/ui/textarea'
import { ToggleGroup, ToggleGroupItem } from '@/components/ui/toggle-group'
import { invalidateVerification } from './useVerification'
import { verificationService } from './verificationService'

const props = defineProps<{ draft: FeedbackDraft, records?: VerificationRecord[] }>()
const emit = defineEmits<{ changed: [] }>()
const payload = shallowRef<FeedbackPayload>(structuredClone(props.draft.payload))
const revision = shallowRef(props.draft.revision)
const error = shallowRef('')
const busy = shallowRef(false)
const supersedes = shallowRef('none')
const results: { value: VerificationResult, label: string }[] = [
  { value: 'passed', label: '通过' },
  { value: 'partial', label: '部分通过' },
  { value: 'failed', label: '失败' },
  { value: 'unknown', label: '未确认' },
]
watch(() => props.draft, (value, previous) => {
  if (value.draftId === previous.draftId && JSON.stringify(payload.value) !== JSON.stringify(previous.payload)) {
    if (value.revision !== revision.value) {
      error.value = '草稿已变化，输入已保留；请复制需要的内容后重新加载'
    }
    return
  }
  payload.value = structuredClone(value.payload)
  revision.value = value.revision
})
function reload(): void {
  payload.value = structuredClone(props.draft.payload)
  revision.value = props.draft.revision
  error.value = ''
  emit('changed')
}
function setResult(value: unknown): void {
  if (results.some(item => item.value === value)) {
    payload.value = { ...payload.value, result: value as VerificationResult }
  }
}
function updateCase(index: number, value: unknown): void {
  if (results.some(item => item.value === value)) {
    payload.value = { ...payload.value, caseResults: payload.value.caseResults.map((item, i) => i === index ? { ...item, result: value as VerificationResult } : item) }
  }
}
async function act(action: 'save' | 'confirm' | 'reject'): Promise<void> {
  busy.value = true
  error.value = ''
  const modId = props.draft.modId
  try {
    const args = { modId: props.draft.modId, draftId: props.draft.draftId, expectedRevision: revision.value }
    if (action === 'save') {
      const saved = await verificationService.edit({ ...args, payload: payload.value })
      payload.value = structuredClone(saved.payload)
      revision.value = saved.revision
    }
    else if (action === 'confirm') {
      if (JSON.stringify(payload.value) !== JSON.stringify(props.draft.payload)) {
        throw new Error('请先保存修改，查看最新草稿后再确认')
      }
      await verificationService.confirm({ ...args, supersedes: supersedes.value === 'none' ? undefined : supersedes.value })
    }
    else { await verificationService.reject(args) }
    invalidateVerification(modId)
    emit('changed')
  }
  catch (cause) { error.value = (cause as Error).message }
  finally { busy.value = false }
}
</script>

<template>
  <Card>
    <CardHeader>
      <CardTitle>
        玩家验收草稿 <Badge variant="outline">
          待你确认
        </Badge>
      </CardTitle>
      <CardDescription class="break-all">
        交付版本 {{ draft.payload.versionId }}
      </CardDescription>
    </CardHeader>
    <CardContent>
      <FieldGroup>
        <Field>
          <FieldLabel :for="`feedback-${draft.draftId}`">
            观察结果
          </FieldLabel>
          <Textarea :id="`feedback-${draft.draftId}`" :model-value="payload.observations" :disabled="busy" @update:model-value="payload = { ...payload, observations: String($event) }" />
        </Field>
        <Field>
          <FieldLabel>结果</FieldLabel>
          <ToggleGroup type="single" :model-value="payload.result" :disabled="busy" @update:model-value="setResult">
            <ToggleGroupItem v-for="item in results" :key="item.value" :value="item.value">
              {{ item.label }}
            </ToggleGroupItem>
          </ToggleGroup>
        </Field>
        <Field>
          <FieldLabel :for="`feedback-game-${draft.draftId}`">
            游戏版本
          </FieldLabel>
          <Input :id="`feedback-game-${draft.draftId}`" :model-value="payload.gameVersion ?? ''" :disabled="busy" @update:model-value="payload = { ...payload, gameVersion: String($event) || undefined }" />
        </Field>
        <Field v-for="(item, index) in payload.caseResults" :key="item.caseId">
          <FieldLabel>步骤 {{ item.caseId }}：{{ item.observed }}</FieldLabel>
          <ToggleGroup type="single" :model-value="item.result" :disabled="busy" @update:model-value="updateCase(index, $event)">
            <ToggleGroupItem v-for="result in results" :key="result.value" :value="result.value">
              {{ result.label }}
            </ToggleGroupItem>
          </ToggleGroup>
        </Field>
        <Field v-if="records?.length">
          <FieldLabel>是否更正旧记录</FieldLabel>
          <Select v-model="supersedes" :disabled="busy">
            <SelectTrigger><SelectValue /></SelectTrigger>
            <SelectContent>
              <SelectGroup>
                <SelectItem value="none">
                  新增记录
                </SelectItem>
                <SelectItem v-for="record in records" :key="record.recordId" :value="record.recordId">
                  {{ new Date(record.confirmedAt).toLocaleString() }} · {{ record.payload.observations.slice(0, 40) }}
                </SelectItem>
              </SelectGroup>
            </SelectContent>
          </Select>
        </Field>
        <p class="text-xs text-muted-foreground">
          原反馈摘要保留在记录中；原会话删除后可能无法打开原消息或附件。
        </p>
        <p v-for="source in draft.evidence" :key="`${source.ref.entryId}-${source.ref.attachmentIndex}`" class="break-words text-xs text-muted-foreground">
          {{ source.summary || '图片反馈' }} · 消息 {{ source.ref.entryId }}
        </p>
        <p v-if="error" role="alert" class="text-sm text-destructive">
          {{ error }}
        </p>
      </FieldGroup>
    </CardContent>
    <CardFooter class="flex flex-wrap gap-2">
      <Button :disabled="busy" @click="act('confirm')">
        确认记录
      </Button>
      <Button variant="outline" :disabled="busy" @click="act('save')">
        保存修改
      </Button>
      <Button v-if="error" variant="outline" :disabled="busy" @click="reload">
        重新加载（放弃编辑）
      </Button>
      <Button variant="ghost" :disabled="busy" @click="act('reject')">
        拒绝草稿
      </Button>
    </CardFooter>
  </Card>
</template>
