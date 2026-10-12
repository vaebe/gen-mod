<script setup lang="ts">
import type { ModelForm } from '@/contracts/config'
import { useId } from 'vue'
import { Button } from '@/components/ui/button'
import { Checkbox } from '@/components/ui/checkbox'
import { Empty, EmptyDescription, EmptyHeader, EmptyTitle } from '@/components/ui/empty'
import { Field, FieldGroup, FieldLabel, FieldLegend, FieldSet } from '@/components/ui/field'
import { Input } from '@/components/ui/input'

const models = defineModel<ModelForm[]>({ required: true })
const fieldId = useId()
function add(): void {
  models.value = [...models.value, { id: '', name: '', contextWindow: '', maxTokens: '', reasoning: false, input: ['text'] }]
}
function image(index: number, enabled: boolean): void {
  models.value = models.value.map((model, position) => position === index ? { ...model, input: enabled ? ['text', 'image'] : ['text'] } : model)
}
</script>

<template>
  <div class="flex flex-col gap-4">
    <div class="flex items-center justify-between gap-3">
      <h3 class="text-sm font-medium">
        模型
      </h3><Button type="button" size="sm" variant="outline" @click="add">
        添加模型
      </Button>
    </div>
    <Empty v-if="!models.length">
      <EmptyHeader><EmptyTitle>尚未添加模型</EmptyTitle><EmptyDescription>点击右侧“添加模型”，按服务商规格填写参数。</EmptyDescription></EmptyHeader>
    </Empty>
    <FieldSet v-for="(model, index) in models" :key="index" class="rounded-lg border p-4" :aria-label="`模型 ${index + 1}`">
      <div class="flex items-center justify-between">
        <span class="text-xs text-muted-foreground">模型 {{ index + 1 }}</span><Button type="button" variant="ghost" size="sm" class="text-destructive" @click="models = models.filter((_, position) => position !== index)">
          移除
        </Button>
      </div>
      <FieldGroup class="grid gap-4 sm:grid-cols-2">
        <Field>
          <FieldLabel :for="`${fieldId}-${index}-id`">
            模型 ID
          </FieldLabel><Input :id="`${fieldId}-${index}-id`" v-model="model.id" required placeholder="服务商提供的模型 ID" />
        </Field>
        <Field>
          <FieldLabel :for="`${fieldId}-${index}-name`">
            显示名称
          </FieldLabel><Input :id="`${fieldId}-${index}-name`" v-model="model.name" required placeholder="便于识别的名称" />
        </Field>
        <Field>
          <FieldLabel :for="`${fieldId}-${index}-context`">
            上下文长度
          </FieldLabel><Input :id="`${fieldId}-${index}-context`" v-model.number="model.contextWindow" type="number" required min="1" max="4294967295" step="1" placeholder="Token 数，按模型规格填写" />
        </Field>
        <Field>
          <FieldLabel :for="`${fieldId}-${index}-tokens`">
            最大输出
          </FieldLabel><Input :id="`${fieldId}-${index}-tokens`" v-model.number="model.maxTokens" type="number" required min="1" :max="model.contextWindow || 4294967295" step="1" placeholder="Token 数，按模型规格填写" />
        </Field>
      </FieldGroup>
      <FieldSet>
        <FieldLegend variant="label">
          模型能力
        </FieldLegend>
        <FieldGroup class="gap-3">
          <Field orientation="horizontal">
            <Checkbox :id="`${fieldId}-${index}-image`" :model-value="model.input.includes('image')" @update:model-value="image(index, $event === true)" /><FieldLabel :for="`${fieldId}-${index}-image`">
              支持图片输入
            </FieldLabel>
          </Field>
          <Field orientation="horizontal">
            <Checkbox :id="`${fieldId}-${index}-reasoning`" :model-value="model.reasoning" @update:model-value="model.reasoning = $event === true" /><FieldLabel :for="`${fieldId}-${index}-reasoning`">
              支持推理
            </FieldLabel>
          </Field>
        </FieldGroup>
      </FieldSet>
    </FieldSet>
  </div>
</template>
