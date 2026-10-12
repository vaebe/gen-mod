<script setup lang="ts">
import type { ModelForm, ProviderApi, ProviderConfiguration } from '@/contracts/config'
import { defineAsyncComponent, reactive, shallowRef } from 'vue'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { providerApis } from '@/contracts/config'

const props = defineProps<{ provider?: ProviderConfiguration, existingIds: string[] }>()
const emit = defineEmits<{ apply: [provider: ProviderConfiguration], cancel: [] }>()
const ModelEditor = defineAsyncComponent(() => import('./ModelEditor.vue'))
const error = shallowRef('')
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
    error.value = 'Provider ID 只能包含字母、数字、点、横线和下划线。'
    return
  }
  if (props.existingIds.includes(id) && id !== props.provider?.id) {
    error.value = 'Provider ID 已存在。'
    return
  }
  if (!form.models.length) {
    error.value = '请至少添加一个模型。'
    return
  }
  const models = form.models.map(model => ({ ...model, id: model.id.trim(), name: model.name.trim(), contextWindow: Number(model.contextWindow), maxTokens: Number(model.maxTokens) }))
  if (new Set(models.map(model => model.id)).size !== models.length) {
    error.value = '同一 provider 下的模型 ID 不能重复。'
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
    <div class="min-h-0 space-y-6 overflow-y-auto px-6 py-5">
      <div class="grid gap-x-4 gap-y-5 sm:grid-cols-2">
        <label class="flex flex-col gap-2 text-sm"><span>名称</span><Input v-model="form.name" required placeholder="例如：我的模型服务" /></label>
        <label class="flex flex-col gap-2 text-sm"><span class="flex items-center justify-between gap-2">Provider ID <span class="text-xs text-muted-foreground">创建后不可修改</span></span><Input v-model="form.id" required :disabled="!!provider" placeholder="例如：my-provider" /></label>
        <label class="flex flex-col gap-2 text-sm"><span>Base URL</span><Input v-model="form.baseUrl" type="url" required placeholder="https://api.example.com/v1" /></label>
        <label class="flex flex-col gap-2 text-sm"><span>API 协议</span><select v-model="form.api" class="h-9 w-full rounded-md border bg-background px-3 text-sm focus-visible:outline focus-visible:outline-ring"><option v-for="api in providerApis" :key="api.value" :value="api.value">{{ api.label }}</option></select></label>
        <label class="flex flex-col gap-2 text-sm sm:col-span-2"><span>API Key</span><Input v-model="form.apiKey" type="password" autocomplete="new-password" :placeholder="provider?.keyConfigured ? '已配置；留空保留原 Key' : '填写 API Key'" /><span class="text-xs leading-5 text-muted-foreground">{{ provider?.keyConfigured ? '留空保留原 Key，填写新值可替换。' : '本地无鉴权服务可填任意占位值。' }}</span></label>
      </div>
      <div class="border-t pt-5">
        <ModelEditor v-model="form.models" />
      </div>
      <p v-if="error" role="alert" class="text-sm text-destructive">
        {{ error }}
      </p>
    </div>
    <div class="flex shrink-0 justify-end gap-2 border-t px-6 py-4">
      <Button type="button" variant="outline" @click="emit('cancel')">
        取消
      </Button><Button type="submit">
        {{ provider ? '应用修改' : '添加到列表' }}
      </Button>
    </div>
  </form>
</template>
