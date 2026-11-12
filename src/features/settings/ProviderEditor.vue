<script setup lang="ts">
import type { ModelForm, ProviderApi, ProviderConfiguration } from '@/features/settings/types'
import { defineAsyncComponent, reactive, shallowRef, useId } from 'vue'
import { Button } from '@/components/ui/button'
import { Field, FieldDescription, FieldError, FieldGroup, FieldLabel } from '@/components/ui/field'
import { Input } from '@/components/ui/input'
import { Select, SelectContent, SelectGroup, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select'
import { Separator } from '@/components/ui/separator'
import { providerApis } from '@/features/settings/types'

const props = defineProps<{ provider?: ProviderConfiguration, existingIds: string[] }>()
const emit = defineEmits<{ apply: [provider: ProviderConfiguration], cancel: [] }>()
const ModelEditor = defineAsyncComponent(() => import('./ModelEditor.vue'))
const error = shallowRef('')
const fieldId = useId()
const form = reactive({
  id: props.provider?.id ?? '',
  name: props.provider?.name ?? '',
  baseUrl: props.provider?.baseUrl ?? '',
  api: props.provider?.api ?? 'openai-completions' as ProviderApi,
  apiKey: props.provider?.apiKey ?? '',
  models: props.provider?.models.map(model => ({ ...model, input: [...model.input] })) ?? [] as ModelForm[],
})
function apply(): void {
  error.value = ''
  const id = form.id.trim()
  if (!/^[\w.-]+$/.test(id)) {
    error.value = '服务商 ID 只能包含字母、数字、点、横线和下划线。'
    return
  }
  if (props.existingIds.includes(id) && id !== props.provider?.id) {
    error.value = '服务商 ID 已存在。'
    return
  }
  if (!form.models.length) {
    error.value = '请至少添加一个模型。'
    return
  }
  const models = form.models.map(model => ({ ...model, id: model.id.trim(), name: model.name.trim(), contextWindow: Number(model.contextWindow), maxTokens: Number(model.maxTokens) }))
  if (new Set(models.map(model => model.id)).size !== models.length) {
    error.value = '同一模型服务下的模型 ID 不能重复。'
    return
  }
  emit('apply', {
    id,
    name: form.name.trim(),
    baseUrl: form.baseUrl.trim(),
    api: form.api,
    apiKey: form.apiKey.trim(),
    keyConfigured: props.provider?.keyConfigured ?? false,
    models,
  })
  form.apiKey = ''
}
</script>

<template>
  <form class="flex min-h-0 flex-col" @submit.prevent="apply">
    <div class="flex min-h-0 flex-col gap-6 overflow-y-auto px-6 py-5">
      <FieldGroup class="grid gap-x-4 gap-y-5 sm:grid-cols-2">
        <Field>
          <FieldLabel :for="`${fieldId}-name`">
            名称
          </FieldLabel>
          <Input :id="`${fieldId}-name`" v-model="form.name" required placeholder="例如：我的模型服务" />
        </Field>
        <Field :data-disabled="!!provider">
          <FieldLabel :for="`${fieldId}-id`">
            服务商 ID
          </FieldLabel>
          <Input :id="`${fieldId}-id`" v-model="form.id" required :disabled="!!provider" :aria-describedby="`${fieldId}-id-help`" placeholder="例如：my-provider" />
          <FieldDescription :id="`${fieldId}-id-help`">
            创建后不可修改
          </FieldDescription>
        </Field>
        <Field>
          <FieldLabel :for="`${fieldId}-url`">
            服务地址（Base URL）
          </FieldLabel>
          <Input :id="`${fieldId}-url`" v-model="form.baseUrl" type="url" required placeholder="https://api.example.com/v1" />
        </Field>
        <Field>
          <FieldLabel :for="`${fieldId}-api`">
            API 协议
          </FieldLabel>
          <Select v-model="form.api">
            <SelectTrigger :id="`${fieldId}-api`" class="w-full">
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              <SelectGroup>
                <SelectItem v-for="api in providerApis" :key="api.value" :value="api.value">
                  {{ api.label }}
                </SelectItem>
              </SelectGroup>
            </SelectContent>
          </Select>
        </Field>
        <Field class="sm:col-span-2">
          <FieldLabel :for="`${fieldId}-key`">
            API Key
          </FieldLabel>
          <Input :id="`${fieldId}-key`" v-model="form.apiKey" type="password" autocomplete="new-password" :aria-describedby="`${fieldId}-key-help`" :placeholder="provider?.keyConfigured ? '已配置；留空保留原 Key' : '填写 API Key'" />
          <FieldDescription :id="`${fieldId}-key-help`">
            {{ provider?.keyConfigured ? '留空保留原 Key，填写新值可替换。' : '本地无鉴权服务可填任意占位值。' }}
          </FieldDescription>
        </Field>
      </FieldGroup>
      <Separator />
      <ModelEditor v-model="form.models" />
      <FieldError v-if="error">
        {{ error }}
      </FieldError>
    </div>
    <div class="flex shrink-0 justify-end gap-2 border-t px-6 py-4">
      <Button type="button" variant="outline" @click="emit('cancel')">
        取消
      </Button><Button type="submit">
        {{ provider ? '更新到列表' : '添加到列表' }}
      </Button>
    </div>
  </form>
</template>
